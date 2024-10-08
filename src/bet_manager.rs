use crate::{priority_queue::PriorityQueue, types::{Bet, BetStatus, BetType, MarketStatus}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: crate::storage::StorageModule 
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

        let (initial_status, matched_amount, unmatched_amount) = self.matching_bet(&mut selection.priority_queue, &bet_type, &odds, &stake);

        match bet_type {
            BetType::Back => {
                if unmatched_amount > BigUint::zero() {
                    selection.priority_queue.add(Bet {
                        bettor: caller.clone(),
                        event: market_id,
                        selection: selection.clone(),
                        stake_amount: unmatched_amount.clone(),
                        liability: BigUint::zero(),
                        matched_amount: BigUint::zero(),
                        unmatched_amount: unmatched_amount.clone(),
                        potential_profit: self.calculate_potential_profit(&bet_type, &unmatched_amount, &odds),
                        odd: odds.clone(),
                        bet_type: bet_type.clone(),
                        status: BetStatus::Unmatched,
                        payment_token: token_identifier.clone(),
                        payment_nonce: token_nonce,
                        nft_nonce: bet_id,
                        timestamp: current_timestamp
                    });
                }
            },
            BetType::Lay => {
                if unmatched_amount > BigUint::zero() {
                    let lay_liability = self.calculate_potential_liability(&bet_type, &unmatched_amount, &odds);
                    selection.priority_queue.add(Bet {
                        bettor: caller.clone(),
                        event: market_id,
                        selection: selection.clone(),
                        stake_amount: unmatched_amount.clone(),
                        liability: lay_liability,
                        matched_amount: BigUint::zero(),
                        unmatched_amount: unmatched_amount.clone(),
                        potential_profit: unmatched_amount.clone(),
                        odd: odds.clone(),
                        bet_type: bet_type.clone(),
                        status: BetStatus::Unmatched,
                        payment_token: token_identifier.clone(),
                        payment_nonce: token_nonce,
                        nft_nonce: bet_id,
                        timestamp: current_timestamp
                    });
                }
            }
        }

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
            potential_profit: self.calculate_potential_profit(&bet_type, &stake, &odds),
            odd: odds.clone(),
            bet_type: bet_type.clone(),
            status: initial_status,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
            timestamp: current_timestamp
        };

        let bet_nft_nonce = self.mint_bet_nft(&bet);
        self.bet_by_id(bet_id).set(&bet);

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
        priority_queue: &mut PriorityQueue<Self::Api>,
        bet_type: &BetType,
        odds: &BigUint,
        stake: &BigUint
    ) -> (BetStatus, BigUint, BigUint) {
        let mut matched_amount = BigUint::zero();
        let mut unmatched_amount = stake.clone();

        let matching_bets = priority_queue.get_matching_bets(bet_type, odds);

        for mut existing_bet in matching_bets.iter() {
            let existing_unmatched = &existing_bet.stake_amount - &existing_bet.matched_amount;
            let mut match_amount = unmatched_amount.clone().min(existing_unmatched.clone());

            matched_amount += &match_amount;
            unmatched_amount -= &match_amount;
            existing_bet.matched_amount += &match_amount;

            if existing_bet.matched_amount == existing_bet.stake_amount {
                existing_bet.status = BetStatus::Matched;
            } else {
                existing_bet.status = BetStatus::PartiallyMatched;
            }

            priority_queue.remove(existing_bet.nft_nonce);
            if existing_bet.unmatched_amount > BigUint::zero() {
                priority_queue.add(existing_bet);
            }

            if unmatched_amount == BigUint::zero() {
                break;
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