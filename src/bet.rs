use crate::types::{Bet, BetAttributes, BetStatus, BetType, MatchedPart, Sport};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetModule: 
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::nft::NftModule +
    crate::tracker::TrackerModule +
    crate::validation::ValidationModule 
{
    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(
        &self,
        sport: Sport,
        market_id: u64,
        selection_id: u64,
        odds: BigUint,
        bet_type: BetType,
    ) {
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, total_amount) = self
            .call_value()
            .egld_or_single_esdt()
            .into_tuple();

        self.validate_bet_amount(&total_amount);
        self.validate_bet_odds(&odds);
        self.validate_market(market_id);
        self.validate_selection(market_id, selection_id);
        
        let (final_stake, final_liability) = self.calculate_stake_and_liability(
            &bet_type,
            &total_amount,
            &odds
        );

        let bet_id = self.next_bet_id().get();
        self.next_bet_id().set(bet_id + 1);

        let bet = self.create_bet(
            sport,
            market_id,
            selection_id,
            &caller,
            &final_stake,
            &final_liability,
            &total_amount, 
            &odds,
            bet_type,
            token_identifier.clone(),
            token_nonce,
            bet_id 
        );

        let (updated_bet, matched_amount, remaining) = self.process_bet(bet);

        self.bet_by_id(bet_id).set(&updated_bet);

        let final_bet = self.update_bet_status(updated_bet, matched_amount.clone(), remaining.clone());
        
        self.update_market_and_selection(
            market_id,
            selection_id,
            &matched_amount
        );

        self.handle_nft_and_locked_funds(
            &caller,
            &final_bet,
            &remaining,
            &final_liability,
            bet_type
        );

        self.emit_bet_placed_event(
            &final_bet,
            &token_identifier,
            token_nonce,
            &matched_amount,
            &remaining,
            bet_id 
        );
    }

    #[payable("*")]
    #[endpoint(cancelBet)]
    fn cancel_bet(&self, bet_id: u64) { 
        let caller = self.blockchain().get_caller();
        let mut bet = self.bet_by_id(bet_id).get(); // Folosim bet_id pentru a accesa bet-ul

        let (token_identifier, payment_nonce, _amount) = self
            .call_value()
            .egld_or_single_esdt()
            .into_tuple();

        let token_identifier_wrap = token_identifier.unwrap_esdt();

        require!(bet.bettor == caller, "Not bet owner");
        require!(
            bet.status == BetStatus::Unmatched || bet.status == BetStatus::PartiallyMatched,
            "Bet cannot be cancelled"
        );
        
        let unmatched = &bet.stake_amount - &bet.total_matched;
        
        let refund_amount = match bet.bet_type {
            BetType::Back => unmatched.clone(),
            BetType::Lay => {
                let unmatched_ratio = (&unmatched * &BigUint::from(100u64)) / &bet.stake_amount;
                &bet.total_amount * &unmatched_ratio / &BigUint::from(100u64)
            }
        };
        
        self.remove_from_orderbook(&bet);

        match &bet.status {
            BetStatus::Unmatched => {
                self.selection_unmatched_count(bet.event, bet.selection.id)
                    .update(|val| *val -= 1);

                self.send().esdt_local_burn(
                    &token_identifier_wrap,
                    payment_nonce,
                    &BigUint::from(1u64)
                );

                bet.status = BetStatus::Canceled;
            },
            BetStatus::PartiallyMatched => {
                self.selection_partially_matched_count(bet.event, bet.selection.id)
                    .update(|val| *val -= 1);
                self.selection_matched_count(bet.event, bet.selection.id)
                    .update(|val| *val += 1);

                bet.status = BetStatus::Matched;

                self.send().direct_esdt(
                    &caller,
                    &token_identifier_wrap,
                    bet.nft_nonce, 
                    &BigUint::from(1u64)
                );
            },
            _ => {}
        };
        
        bet.stake_amount = bet.total_matched.clone();
        if bet.total_matched > BigUint::zero() {
            bet.total_amount = match bet.bet_type {
                BetType::Back => bet.total_matched.clone(),
                BetType::Lay => {
                    let matched_liability = &bet.liability * &bet.total_matched / &bet.stake_amount;
                    &bet.total_matched + &matched_liability
                }
            };
        } else {
            bet.total_amount = BigUint::zero();
        }

        self.bet_by_id(bet_id).set(&bet); 
        
        self.locked_funds(&caller).update(|val| *val -= &refund_amount);
        self.send().direct(&caller, &bet.payment_token, 0, &refund_amount);
    }

    fn calculate_matched_potential_profit(&self, bet: &Bet<Self::Api>) -> BigUint {
        let mut total_potential_profit = BigUint::zero();
        
        for part in bet.matched_parts.iter() {
            let potential_profit = match bet.bet_type {
                BetType::Back => {
                    &part.amount * &part.odds / BigUint::from(100u64) - &part.amount
                },
                BetType::Lay => {
                    part.amount.clone()
                }
            };
            total_potential_profit += potential_profit;
        }
        
        total_potential_profit
    }

    fn create_bet(
        &self,
        sport: Sport,
        market_id: u64,
        selection_id: u64,
        caller: &ManagedAddress<Self::Api>,
        stake: &BigUint,
        liability: &BigUint,
        total_amount: &BigUint,
        odds: &BigUint,
        bet_type: BetType,
        token_identifier: EgldOrEsdtTokenIdentifier<Self::Api>,
        token_nonce: u64,
        bet_id: u64, 
    ) -> Bet<Self::Api> {
        let market = self.markets(market_id).get();
        let selection = market.selections
            .iter()
            .find(|s| s.id == selection_id)
            .unwrap_or_else(|| sc_panic!("Invalid selection"))
            .clone();
        
        Bet {
            bet_id,
            bettor: caller.clone(),
            sport,
            event: market_id,
            selection,
            stake_amount: stake.clone(),
            liability: liability.clone(),
            total_amount: total_amount.clone(), 
            total_matched: BigUint::zero(),
            matched_parts: ManagedVec::new(),
            potential_profit: BigUint::zero(),
            odd: odds.clone(),
            bet_type,
            status: BetStatus::Unmatched,
            payment_token: token_identifier,
            payment_nonce: token_nonce,
            nft_nonce: self.blockchain().get_current_esdt_nft_nonce(
                &self.blockchain().get_sc_address(),
                self.bet_nft_token().get_token_id_ref(),
            ),
            created_at: self.blockchain().get_block_timestamp()
        }
    }

    fn update_bet_status(
        &self,
        mut bet: Bet<Self::Api>,
        matched_amount: BigUint,
        remaining: BigUint
    ) -> Bet<Self::Api> {
        bet.total_matched = matched_amount.clone();
        bet.status = if matched_amount > BigUint::zero() {
            if remaining > BigUint::zero() {
                BetStatus::PartiallyMatched
            } else {
                BetStatus::Matched
            }
        } else {
            BetStatus::Unmatched
        };
        bet
    }

    fn update_market_and_selection(
        &self,
        market_id: u64,
        selection_id: u64,
        matched_amount: &BigUint,
    ) {
        let mut market = self.markets(market_id).get();
        let selection_index = market
            .selections
            .iter()
            .position(|s| s.id == selection_id)
            .unwrap_or_else(|| sc_panic!("Invalid selection"));
        
        let mut selection = market.selections.get(selection_index);
        selection.priority_queue = self.selection_tracker(market_id, selection_id).get();
        
        let _ = market.selections.set(selection_index, selection);
        market.total_matched_amount += matched_amount;
        self.markets(market_id).set(&market);
    }

    fn handle_nft_and_locked_funds(
        &self,
        caller: &ManagedAddress<Self::Api>,
        bet: &Bet<Self::Api>,
        remaining: &BigUint,
        liability: &BigUint,
        bet_type: BetType
    ) {
        let bet_nft_nonce = self.mint_bet_nft(bet);
        self.bet_by_id(bet.bet_id).set(bet); 
        self.market_bet_ids(bet.event).insert(bet.nft_nonce); 

        let amount_to_lock = match bet_type {
            BetType::Back => remaining.clone(),
            BetType::Lay => {
                let ratio = remaining * &BigUint::from(100u64) / &bet.stake_amount;
                (&bet.total_amount * &ratio) / &BigUint::from(100u64)
            }
        };

        if amount_to_lock > BigUint::zero() {
            self.locked_funds(&caller).update(|funds| *funds += &amount_to_lock);
        }
    
        self.send().direct_esdt(
            caller,
            self.bet_nft_token().get_token_id_ref(),
            bet_nft_nonce,
            &BigUint::from(1u64)
        );
    }

    fn emit_bet_placed_event(
        &self,
        bet: &Bet<Self::Api>,
        token_identifier: &EgldOrEsdtTokenIdentifier<Self::Api>,
        token_nonce: u64,
        matched_amount: &BigUint,
        unmatched_amount: &BigUint,
        bet_id: u64,
    ) {
        let sport_index = match bet.sport {
            Sport::Football => 1u8,
            Sport::Basketball => 2u8,
            Sport::Tennis => 3u8,
            Sport::LeagueOfLegends => 4u8,
            Sport::CounterStrike2 => 5u8,
            Sport::Dota2 => 6u8,
        };
        
        let user_hex = bet.bettor.hex_expr();
        
        let bet_type_value = match bet.bet_type {
            BetType::Back => 1u8,
            BetType::Lay => 2u8,
        };
        
        let status_value = match bet.status {
            BetStatus::Unmatched => 1u8,
            BetStatus::Matched => 2u8,
            BetStatus::PartiallyMatched => 3u8,
            BetStatus::Win => 4u8,
            BetStatus::Lost => 5u8,
            BetStatus::Canceled => 6u8,
            BetStatus::Claimed => 7u8,
        };
        
        let potential_profit = match bet.bet_type {
            BetType::Back => &bet.stake_amount * &(&bet.odd - &BigUint::from(100u64)) / &BigUint::from(100u64),
            BetType::Lay => bet.stake_amount.clone(),
        };
        
        self.place_bet_event(
            &user_hex,
            sport_index,
            &bet.event,
            &bet.selection.id,
            &bet.stake_amount,
            &bet.odd,
            bet_type_value,
            token_identifier,
            &bet.liability,
            status_value,
            &bet.matched_parts, 
            bet_id,
            &potential_profit,
        );
    }
    
    #[event("debugVecLength")]
    fn debug_vec_length(&self, #[indexed] length: usize);

    fn calculate_stake_and_liability(
        &self,
        bet_type: &BetType,
        total_amount: &BigUint, 
        odds: &BigUint
    ) -> (BigUint, BigUint) {
        match bet_type {
            BetType::Back => {
                (total_amount.clone(), BigUint::zero())
            },
            BetType::Lay => {
                
                let odds_minus_one = odds - &BigUint::from(100u64); 
                require!(odds_minus_one > 0, "Odds must be greater than 1.00");
                
                let stake = (total_amount * &BigUint::from(100u64)) / odds;
                require!(stake > 0, "Invalid stake calculation for Lay bet");
                
                let liability = total_amount - &stake;
                require!(liability >= BigUint::zero(), "Invalid liability calculation for Lay bet");
                
                (stake, liability) 
            }
        }
    }
}