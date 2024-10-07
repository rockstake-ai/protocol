use crate::storage::{self, Bet, BetStatus, BetType, Market, MarketStatus, Selection};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: storage::StorageModule 
    + crate::events::EventsModule 
    + crate::nft_manager::NftManagerModule {

    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(&self, market_id: u64, selection_id: u64, odds: BigUint, bet_type: BetType) -> SCResult<(u64, BigUint, BigUint)> {
        let mut market = self.markets(&market_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
            require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
            require!(market.market_status == MarketStatus::Open, "Market is not open for betting");
            require!(current_timestamp < market.close_timestamp, "Market is closed");
            require!(odds >= BigUint::from(101u32) && odds <= BigUint::from(100000u32), "Odds must be between 1.01 and 1000.00");
    
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, stake_amount) = self.call_value().egld_or_single_esdt().into_tuple();
        let bet_id = self.get_last_bet_id() + 1;

        let token_identifier_clone = token_identifier.clone();
        let total_amount = self.blockchain().get_esdt_balance(&caller, &token_identifier_clone.unwrap_esdt(), token_nonce);

        let selection_index = market.selections.iter()
            .position(|s| &s.selection_id == &selection_id)
            .expect("Selection not found in this market");
        let mut selection = market.selections.get(selection_index);
    
        let (stake, liability) = match bet_type {
            BetType::Back => {
                let stake = stake_amount.clone();
                (stake.clone(), stake)
            },
            BetType::Lay => {
                let liability = self.calculate_potential_liability(&bet_type, &stake_amount, &odds);
                let stake = self.calculate_stake_from_liability(&liability, &odds);
                (stake, liability)
            }
        };
    
        require!(total_amount >= liability, "Insufficient funds for this bet");
    
        let (initial_status, matched_amount, unmatched_amount) = self.matching_bet(&mut market, &mut selection, &bet_type, &odds, &stake); 
    
        match bet_type {
            BetType::Back => {
                selection.back_liquidity += &unmatched_amount;
                market.back_liquidity += &unmatched_amount;
            },
            BetType::Lay => {
                let lay_liquidity = self.calculate_win_amount(&BetType::Lay, &unmatched_amount, &odds);
                selection.lay_liquidity += &lay_liquidity;
                market.lay_liquidity += &lay_liquidity;
            }
        }
    
        let potential_profit = self.calculate_potential_profit(&bet_type, &stake, &odds);
        let potential_liability = self.calculate_potential_liability(&bet_type, &stake, &odds);
    
        let bet = Bet {
            bettor: caller.clone(),
            event: market_id,
            selection: selection.clone(),
            stake_amount: stake.clone(),
            liability: match bet_type {
                BetType::Back => BigUint::zero(),
                BetType::Lay => liability.clone() - &stake,
            },
            matched_amount: matched_amount.clone(),
            unmatched_amount: unmatched_amount.clone(),
            potential_profit,
            potential_liability,
            odd: odds.clone(),
            bet_type: bet_type.clone(),
            status: initial_status,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
        };
    
        let bet_nft_nonce = self.mint_bet_nft(&bet);
        self.bet_by_id(bet_id).set(&bet);
        market.bets.push(bet.clone());
    
        let _ = market.selections.set(selection_index, &selection);
        market.total_matched_amount += &matched_amount;
        self.markets(&market_id).set(&market);
    
        if unmatched_amount > BigUint::zero() || liability > stake {
            let total_locked = match bet_type {
                BetType::Back => unmatched_amount.clone(),
                BetType::Lay => liability.clone() - &stake + &unmatched_amount,
            };
            self.locked_funds(&caller).update(|current_locked| *current_locked += &total_locked);
        }
    
    
        self.send().direct_esdt(&caller, self.bet_nft_token().get_token_id_ref(), bet_nft_nonce, &BigUint::from(1u64));
        
        self.bet_placed_event(
            &caller,
            self.bet_nft_token().get_token_id_ref(),
            &market_id,
            &selection_id,
            &stake,
            &odds,
            bet_type,
            &token_identifier,
            token_nonce,
            &matched_amount,
            &unmatched_amount,
            &(liability.clone() - &stake)
        );
    
        let surplus = stake_amount - liability;
        if surplus > BigUint::zero() {
            self.send().direct(&caller, &token_identifier, token_nonce, &surplus);
        }
    
        Ok((bet_id, odds, stake))
    }

    fn matching_bet(
        &self,
        market: &mut Market<Self::Api>,
        selection: &mut Selection<Self::Api>,
        bet_type: &BetType,
        odds: &BigUint,
        stake: &BigUint
    ) -> (BetStatus, BigUint, BigUint) {
        let mut matched_amount = BigUint::zero();
        let mut unmatched_amount = stake.clone();
    
        for i in 0..market.bets.len() {
            let mut existing_bet = market.bets.get(i);
    
            if existing_bet.selection.selection_id == selection.selection_id &&
               (existing_bet.status == BetStatus::Unmatched || existing_bet.status == BetStatus::PartiallyMatched) &&
               existing_bet.bet_type != *bet_type {
    
                if (bet_type == &BetType::Back && odds <= &existing_bet.odd) ||
                   (bet_type == &BetType::Lay && odds >= &existing_bet.odd) {
                    let existing_unmatched = &existing_bet.stake_amount - &existing_bet.matched_amount;
                    let mut match_amount = unmatched_amount.clone().min(existing_unmatched.clone());
    
                    match bet_type {
                        BetType::Back => {
                            let potential_win = self.calculate_potential_profit(bet_type, &match_amount, odds);
                            if potential_win > selection.lay_liquidity {
                                match_amount = self.calculate_stake_from_win(&selection.lay_liquidity, odds);
                            }
                        },
                        BetType::Lay => {
                            let potential_liability = self.calculate_potential_liability(bet_type, &match_amount, odds);
                            if potential_liability > selection.back_liquidity {
                                match_amount = self.calculate_stake_from_liability(&selection.back_liquidity, odds);
                            }
                        }
                    }
    
                    matched_amount += &match_amount;
                    unmatched_amount -= &match_amount;
                    existing_bet.matched_amount += &match_amount;
    
                    match bet_type {
                        BetType::Back => {
                            let win_amount = self.calculate_potential_profit(bet_type, &match_amount, odds);
                            selection.lay_liquidity -= &win_amount;
                            market.lay_liquidity -= &win_amount;
                        },
                        BetType::Lay => {
                            let liability = self.calculate_potential_liability(bet_type, &match_amount, odds);
                            selection.back_liquidity -= &liability;
                            market.back_liquidity -= &liability;
                        }
                    }
    
                    if existing_bet.matched_amount == existing_bet.stake_amount {
                        existing_bet.status = BetStatus::Matched;
                    } else {
                        existing_bet.status = BetStatus::PartiallyMatched;
                    }
    
                    let _ = market.bets.set(i, &existing_bet);
    
                    if unmatched_amount == BigUint::zero() {
                        break;
                    }
                }
            }
        }
    
        let status = if matched_amount == *stake {
            BetStatus::Matched
        } else if matched_amount > BigUint::zero() {
            BetStatus::PartiallyMatched
        } else {
            BetStatus::Unmatched
        };
    
        (status, matched_amount, unmatched_amount)
    }
    
    fn calculate_stake_from_win(&self, win_amount: &BigUint, odds: &BigUint) -> BigUint {
        (win_amount * &BigUint::from(100u32)) / (odds - &BigUint::from(100u32))
    }
    
    fn calculate_stake_from_liability(&self, liability: &BigUint, odds: &BigUint) -> BigUint {
        (liability * &BigUint::from(100u32)) / (odds - &BigUint::from(100u32))
    }

    fn calculate_potential_profit(&self, bet_type: &BetType, stake: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => {
                let profit = (odds - &BigUint::from(100u32)) * stake / BigUint::from(100u32);
                profit
            },
            BetType::Lay => {
                stake.clone()
            }
        }
    }
    
    fn calculate_potential_liability(&self, bet_type: &BetType, stake: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => {
                stake.clone()
            },
            BetType::Lay => {
                let liability = (odds - &BigUint::from(100u32)) * stake / BigUint::from(100u32);
                liability
            }
        }
    }
    
    fn calculate_win_amount(&self, bet_type: &BetType, stake_amount: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => self.calculate_potential_profit(bet_type, stake_amount, odds),
            BetType::Lay => self.calculate_potential_liability(bet_type, stake_amount, odds),
        }
    }
           
}