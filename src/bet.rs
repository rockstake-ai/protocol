use crate::types::{Bet, BetAttributes, BetStatus, BetType, DebugBetState, DebugMatchedPart};
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

        let bet = self.create_bet(
            market_id,
            selection_id,
            &caller,
            &final_stake,
            &final_liability,
            &odds,
            bet_type,
            token_identifier.clone(),
            token_nonce
        );

        let (matched_amount, remaining) = self.process_bet(bet.clone());
        let updated_bet = self.update_bet_status(bet, matched_amount.clone(), remaining.clone());
        self.update_market_and_selection(
            market_id,
            selection_id,
            &matched_amount
        );

        self.handle_nft_and_locked_funds(
            &caller,
            &updated_bet,
            &remaining,
            &final_liability,
            bet_type
        );

        self.emit_bet_placed_event(
            &updated_bet,
            &token_identifier,
            token_nonce,
            &matched_amount,
            &remaining
        );
    }

    #[payable("*")]
    #[endpoint(cancelBet)]
    fn cancel_bet(&self, bet_nonce: u64) {
        let caller = self.blockchain().get_caller();
        let mut bet = self.bet_by_id(bet_nonce).get();

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
            BetType::Back => unmatched,
            BetType::Lay => bet.liability.clone()
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
                    bet_nonce,
                    &BigUint::from(1u64)
                );
            },
            _ => {}
        }
        
        bet.stake_amount = bet.total_matched.clone();
        self.bet_by_id(bet_nonce).set(&bet);
        
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
                    part.amount
                }
            };
            total_potential_profit += potential_profit;
        }
        
        total_potential_profit
    }

    #[endpoint(updateBet)]
    #[payable("*")]
    #[allow_multiple_var_args]
    fn update_bet(
        &self,
        bet_nonce: u64,
        new_odds: OptionalValue<BigUint>,
        new_amount: OptionalValue<BigUint>,
    ) {
        let (token_identifier, payment_nonce, _amount) = self
        .call_value()
        .egld_or_single_esdt()
        .into_tuple();

        let caller = self.blockchain().get_caller();
        let mut bet = self.bet_by_id(bet_nonce).get();
        
        require!(bet.bettor == caller, "Not bet owner");
        require!(
            bet.status == BetStatus::Unmatched || bet.status == BetStatus::PartiallyMatched,
            "Bet cannot be updated"
        );
        require!(
            new_odds.is_some() || new_amount.is_some(),
            "Must provide new odds or amount"
        );

        require!(
            token_identifier == self.bet_nft_token().get_token_id(),
            "Invalid NFT token identifier"
        );
        require!(payment_nonce == bet_nonce, "Invalid NFT nonce");

        let old_unmatched = &bet.stake_amount - &bet.total_matched;
        
        let old_liability = match bet.bet_type {
            BetType::Back => BigUint::zero(),
            BetType::Lay => bet.liability.clone()
        };

        let update_odds = match &new_odds {
            OptionalValue::Some(odds) => {
                self.validate_bet_odds(odds);
                odds.clone()
            },
            OptionalValue::None => bet.odd.clone()
        };

        let new_unmatched = match &new_amount {
            OptionalValue::Some(amount) => {
                require!(
                    amount <= &old_unmatched,
                    "New amount cannot exceed unmatched amount"
                );
                amount.clone()
            },
            OptionalValue::None => old_unmatched.clone()
        };

        let refund_amount = if new_amount.is_some() {
            old_unmatched.clone() - &new_unmatched
        } else {
            BigUint::zero()
        };

        if new_amount.is_some() || new_odds.is_some() {
            self.remove_from_orderbook(&bet);
        }

        bet.odd = update_odds.clone();
        bet.stake_amount = bet.total_matched.clone() + &new_unmatched;
        bet.potential_profit = self.calculate_total_potential_profit(&bet);

        let mut total_liability = BigUint::zero();
        
        for part in bet.matched_parts.iter() {
            let (_, matched_liability) = self.calculate_stake_and_liability(
                &bet.bet_type,
                &part.amount,
                &part.odds
            );
            total_liability += matched_liability;
        }

        if new_unmatched > BigUint::zero() {
            let (_, unmatched_liability) = self.calculate_stake_and_liability(
                &bet.bet_type,
                &new_unmatched,
                &update_odds
            );
            total_liability += unmatched_liability;
        }

        bet.liability = total_liability.clone();

        let (matched_amount, remaining) = self.process_bet(bet.clone());

        let attributes = BetAttributes {
            event: bet.event.clone(),
            selection: bet.selection.clone(),
            stake: bet.stake_amount.clone(),
            potential_win: bet.potential_profit.clone(),
            odd: update_odds.clone(),
            bet_type: bet.bet_type.clone(),
            status: bet.status.clone(),
        };

        let token_identifier_wrap = token_identifier.unwrap_esdt();

        self.locked_funds(&caller).update(|val| {
            *val -= &old_liability;
            *val += &total_liability;
        });

        let updated_bet = self.update_bet_status(bet, matched_amount, remaining);
        self.bet_by_id(bet_nonce).set(&updated_bet);

        self.send().nft_update_attributes(
            &token_identifier_wrap,
            payment_nonce,
            &attributes
        );

        if refund_amount > BigUint::zero() {
            self.send().direct(&caller, &updated_bet.payment_token, 0, &refund_amount);
            self.send().direct_esdt(
                &caller,
                &token_identifier_wrap,
                bet_nonce,
                &BigUint::from(1u64)
            );
        }
    }

    fn create_bet(
        &self,
        market_id: u64,
        selection_id: u64,
        caller: &ManagedAddress<Self::Api>,
        stake: &BigUint,
        liability: &BigUint,
        odds: &BigUint,
        bet_type: BetType,
        token_identifier: EgldOrEsdtTokenIdentifier<Self::Api>,
        token_nonce: u64
    ) -> Bet<Self::Api> {
        let market = self.markets(market_id).get();
        let selection = market.selections
            .iter()
            .find(|s| s.id == selection_id)
            .unwrap_or_else(|| sc_panic!("Invalid selection"))
            .clone();
        let bet_id = self.get_last_bet_id() + 1;
        
        Bet {
            bettor: caller.clone(),
            event: market_id,
            selection,
            stake_amount: stake.clone(),
            liability: liability.clone(),
            total_matched: BigUint::zero(),
            matched_parts: ManagedVec::new(),
            potential_profit: BigUint::zero(),
            odd: odds.clone(),
            bet_type,
            status: BetStatus::Unmatched,
            payment_token: token_identifier,
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
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
        self.bet_by_id(bet.nft_nonce).set(bet);

        self.market_bet_ids(bet.event).insert(bet.nft_nonce);
        let total_locked = match bet_type {
            BetType::Back => remaining.clone(),
            BetType::Lay => liability.clone(),
        };
        self.locked_funds(caller).update(|current_locked| *current_locked += &total_locked);

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
        unmatched_amount: &BigUint
    ) {
        self.place_bet_event(
            &bet.bettor,
            self.bet_nft_token().get_token_id_ref(),
            &bet.event,
            &bet.selection.id,
            &bet.stake_amount,
            &bet.odd,
            bet.bet_type,
            token_identifier,
            token_nonce,
            matched_amount,
            unmatched_amount,
            &bet.liability
        );
    }

    fn calculate_stake_and_liability(
        &self,
        bet_type: &BetType,
        total_amount: &BigUint,
        odds: &BigUint
    ) -> (BigUint, BigUint) {
        match bet_type {
            BetType::Back => self.validate_back_bet(total_amount),
            BetType::Lay => self.validate_lay_bet(total_amount, odds)
        }
    }

    fn calculate_potential_profit(
        &self, 
        bet_type: &BetType, 
        stake: &BigUint, 
        odds: &BigUint
    ) -> BigUint {
        match bet_type {
            BetType::Back => {
                (odds - &BigUint::from(100u32)) * stake / &BigUint::from(100u32)
            },
            BetType::Lay => stake.clone()
        }
    }

    fn get_last_bet_id(&self) -> u64 {
        self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            self.bet_nft_token().get_token_id_ref(),
        )
    }

    #[view(getDebugBetState)]
    fn get_debug_bet_state(
        &self,
        bet_nonce: u64
    ) -> DebugBetState<Self::Api> {
        let bet = self.bet_by_id(bet_nonce).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
        
        let mut matched_parts = ManagedVec::new();
        for part in bet.matched_parts.iter() {
            matched_parts.push(DebugMatchedPart {
                amount: part.amount,
                odds: part.odds
            });
        }
        
        DebugBetState {
            bet_type: bet.bet_type,
            stake_amount: bet.stake_amount,
            matched_amount: bet.total_matched,
            unmatched_amount: unmatched,
            status: bet.status,
            current_odds: bet.odd,
            potential_profit: bet.potential_profit,
            matched_parts
        }
    }
}