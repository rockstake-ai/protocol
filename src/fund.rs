use crate::{errors::{ERR_INVALID_MARKET, ERR_MARKET_NOT_CLOSED, ERR_MARKET_NOT_SETTLED}, types::{Bet, BetAttributes, BetStatus, BetType, MarketStatus, MarketType, ProcessingProgress, ProcessingStatus, Tracker}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FundModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft::NftModule
    + crate::tracker::TrackerModule
{
    fn handle_expired_market(&self, market_id: u64) {
        let mut market = self.markets(market_id).get();
        market.market_status = MarketStatus::Closed;
        self.markets(market_id).set(&market);
        
        for selection in market.selections.iter() {
            let back_levels = self.selection_back_levels(market_id, selection.id).get();
            for level in back_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.return_unmatched_amount(bet_nonce);
                }
            }
    
            let lay_levels = self.selection_lay_levels(market_id, selection.id).get();
            for level in lay_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.return_unmatched_amount(bet_nonce);
                }
            }
    
            self.selection_back_levels(market_id, selection.id).set(&ManagedVec::new());
            self.selection_lay_levels(market_id, selection.id).set(&ManagedVec::new());
            self.selection_back_liquidity(market_id, selection.id).set(&BigUint::zero());
            self.selection_lay_liquidity(market_id, selection.id).set(&BigUint::zero());
    
            let tracker = Tracker {
                back_levels: ManagedVec::new(),
                lay_levels: ManagedVec::new(),
                back_liquidity: BigUint::zero(),
                lay_liquidity: BigUint::zero(),
                matched_count: self.selection_matched_count(market_id, selection.id).get(),
                unmatched_count: self.selection_unmatched_count(market_id, selection.id).get(),
                partially_matched_count: self.selection_partially_matched_count(market_id, selection.id).get(),
                win_count: self.selection_win_count(market_id, selection.id).get(),
                lost_count: self.selection_lost_count(market_id, selection.id).get(),
                canceled_count: self.selection_canceled_count(market_id, selection.id).get(),
            };
            
            self.selection_tracker(market_id, selection.id).set(&tracker);
        }
        
    }
    
    fn return_unmatched_amount(&self, bet_nonce: u64) {
        let mut bet = self.bet_by_id(bet_nonce).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
        
        if unmatched > BigUint::zero() {
            let refund_amount = match bet.bet_type {
                BetType::Back => unmatched.clone(),
                BetType::Lay => {
                    let unmatched_ratio = (&unmatched * &BigUint::from(100u64)) / &bet.stake_amount;
                    (&bet.total_amount * &unmatched_ratio) / &BigUint::from(100u64)
                }
            };
    
            self.send().direct(
                &bet.bettor,
                &bet.payment_token,
                bet.payment_nonce,
                &refund_amount
            );
    
            if bet.total_matched > BigUint::zero() {
                bet.stake_amount = bet.total_matched.clone();
                bet.total_amount = if bet.bet_type == BetType::Lay {
                    let matched_ratio = &bet.total_matched / &bet.stake_amount;
                    &bet.total_amount * &matched_ratio
                } else {
                    bet.total_matched.clone()
                };
                bet.potential_profit = self.calculate_total_potential_profit(&bet);
                bet.status = BetStatus::Matched;
            } else {
                bet.status = BetStatus::Canceled;
                bet.total_amount = BigUint::zero();
            }
            
            self.bet_by_id(bet_nonce).set(&bet);
            // self.bet_refunded_event(bet_nonce, &bet.bettor, &refund_amount);
        }
    }

    fn process_unmatched_bets(&self, market_id: u64) {
        let market = self.markets(market_id).get();
        
        for selection in market.selections.iter() {
            let back_levels = self.selection_back_levels(market_id, selection.id).get();
            for level in back_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.process_unmatched_bet(bet_nonce);
                }
            }

            let lay_levels = self.selection_lay_levels(market_id, selection.id).get();
            for level in lay_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.process_unmatched_bet(bet_nonce);
                }
            }

            self.selection_back_liquidity(market_id, selection.id).set(&BigUint::zero());
            self.selection_lay_liquidity(market_id, selection.id).set(&BigUint::zero());
        }
    }

    fn process_unmatched_bet(&self, bet_nonce: u64) {
        let mut bet = self.bet_by_id(bet_nonce).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
        
        if unmatched > BigUint::zero() {
            let refund_amount = match bet.bet_type {
                BetType::Back => unmatched.clone(),
                BetType::Lay => {
                    let unmatched_ratio = (&unmatched * &BigUint::from(100u64)) / &bet.stake_amount;
                    (&bet.total_amount * &unmatched_ratio) / &BigUint::from(100u64)
                }
            };
    
            self.send().direct(
                &bet.bettor,
                &bet.payment_token,
                bet.payment_nonce,
                &refund_amount,
            );
            
            bet.status = if bet.total_matched > BigUint::zero() {
                bet.stake_amount = bet.total_matched.clone();
                bet.total_amount = if bet.bet_type == BetType::Lay {
                    let matched_ratio = &bet.total_matched / &bet.stake_amount;
                    &bet.total_amount * &matched_ratio
                } else {
                    bet.total_matched.clone()
                };
                BetStatus::Matched
            } else {
                bet.total_amount = BigUint::zero();
                BetStatus::Canceled
            };
            
            self.bet_by_id(bet_nonce).set(&bet);
            // self.bet_refunded_event(bet_nonce, &bet.bettor, &refund_amount);
        }
    }
    

    #[only_owner]
    #[endpoint(setEventResult)]
    fn set_event_result(
        &self,
        event_id: u64,
        score_home: u32,
        score_away: u32
    ) {
        self.event_score(event_id).set(&(score_home, score_away));
        
        let market_ids = self.markets_by_event(event_id).get();
        require!(!market_ids.is_empty(), "No markets found for event");
        
        for market_id in market_ids.iter() {
            let mut market = self.markets(market_id).get();
            
            require!(
                market.market_status == MarketStatus::Closed,
                ERR_MARKET_NOT_CLOSED
            );
            
            let winning_selection = self.determine_winner(market.market_type, score_home, score_away, event_id);
            
            self.winning_selection(market_id).set(winning_selection);
            
            market.market_status = MarketStatus::Settled;
            self.markets(market_id).set(&market);
            
            self.mark_bets_win_loss(market_id, winning_selection);
        }
        // self.event_result_set_event(event_id, score_home, score_away);
    }

    fn mark_bets_win_loss(
        &self,
        market_id: u64,
        winning_selection: u64,
    ) {
        let bet_ids = self.market_bet_ids(market_id);
        
        for bet_id in bet_ids.iter() {
            let mut bet = self.bet_by_id(bet_id).get();
            
            if bet.status == BetStatus::Matched {
                let is_winner = match bet.bet_type {
                    BetType::Back => bet.selection.id == winning_selection,
                    BetType::Lay => bet.selection.id != winning_selection
                };
                
                bet.status = if is_winner {
                    BetStatus::Win
                } else {
                    BetStatus::Lost
                };
                
                if is_winner {
                    self.selection_win_count(market_id, bet.selection.id)
                        .update(|count| *count += 1);
                } else {
                    self.selection_lost_count(market_id, bet.selection.id)
                        .update(|count| *count += 1);
                }
                
                self.bet_by_id(bet_id).set(&bet);
                
                // Emit bet settled event
                // self.bet_settled_event(
                //     bet_id,
                //     &bet.bettor,
                //     bet.status,
                //     &bet.potential_profit
                // );
            }
        }
    }

    #[payable("*")]
    #[endpoint(claimWinnings)]
    fn claim_winnings(&self, bet_nonce: u64) {
        let caller = self.blockchain().get_caller();
        let (token_identifier, payment_nonce, _amount) = self
            .call_value()
            .egld_or_single_esdt()
            .into_tuple();
        let token_identifier_wrap = token_identifier.unwrap_esdt();
        
        let mut bet = self.bet_by_id(bet_nonce).get();
        require!(bet.bettor == caller, "Not bet owner");
        require!(bet.status == BetStatus::Win, "Bet not won");

        let payout = match bet.bet_type {
            BetType::Back => {
                &bet.stake_amount + &bet.potential_profit
            },
            BetType::Lay => {
                bet.total_amount.clone()
            }
        };

        bet.status = BetStatus::Claimed;
        self.bet_by_id(bet_nonce).set(&bet);

        self.send().direct(
            &caller,
            &bet.payment_token,
            0,
            &payout
        );

        self.send().direct_esdt(
            &caller,
            &token_identifier_wrap,
            bet_nonce,
            &BigUint::from(1u64)
        );
    }

    fn determine_winner(
        &self,
        market_type: MarketType,
        score_home: u32,
        score_away: u32,
        event_id: u64,
    ) -> u64 {
        let market_ids = self.markets_by_event(event_id).get();
        let market_id = market_ids.iter()
            .find(|&id| {
                let market = self.markets(id).get();
                market.market_type == market_type
            })
            .unwrap_or_else(|| sc_panic!("Market not found"));
            
        let market = self.markets(market_id).get();
        
        let winning_index = match market_type {
            MarketType::FullTimeResult => {
                if score_home > score_away { 0 }     
                else if score_home < score_away { 2 } 
                else { 1 }                           
            },
            MarketType::TotalGoals => {
                if score_home + score_away > 2 { 0 } 
                else { 1 }                          
            },
            MarketType::BothTeamsToScore => {
                if score_home > 0 && score_away > 0 { 0 } 
                else { 1 }                               
            }
        };
        
        market.selections.get(winning_index).id
    }

    #[view(getWinningSelection)]
    fn get_winning_selection(&self, market_id: u64) -> u64 {
        self.winning_selection(market_id).get()
    }

    #[view(getMarketSettlementDetails)]
    fn get_market_settlement_details(
        &self,
        market_id: u64
    ) -> (u64, MarketStatus) {
        let market = self.markets(market_id).get();
        let winning_selection = if self.winning_selection(market_id).is_empty() {
            0u64
        } else {
            self.winning_selection(market_id).get()
        };
        
        (winning_selection, market.market_status)
    }

    #[view(getBetStatusDetails)]
    fn get_bet_status_details(
        &self,
        bet_nonce: u64
    ) -> (BetStatus, BigUint<Self::Api>, BigUint<Self::Api>) {
        let bet = self.bet_by_id(bet_nonce).get();
        (bet.status, bet.total_matched, bet.potential_profit)
    }

    #[view(getProcessingProgress)]
    fn get_processing_progress(&self, market_id: u64) -> ProcessingProgress {
        let current_index = if self.current_processing_index(market_id).is_empty() {
            0u64
        } else {
            self.current_processing_index(market_id).get()
        };

        ProcessingProgress {
            market_id,
            processed_bets: current_index,
            status: if current_index > 0 { 
                ProcessingStatus::InProgress 
            } else { 
                ProcessingStatus::Completed 
            }
        }
    }
}