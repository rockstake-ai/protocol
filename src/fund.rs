use crate::{errors::{ERR_INVALID_MARKET, ERR_MARKET_NOT_CLOSED, ERR_MARKET_NOT_SETTLED}, types::{Bet, BetStatus, BetType, MarketStatus, MarketType, ProcessingProgress, ProcessingStatus}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FundModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft::NftModule
{
    fn handle_expired_market(&self, market_id: u64) {
        let mut market = self.markets(market_id).get();
        market.market_status = MarketStatus::Closed;
        self.markets(market_id).set(&market);
        
        self.process_unmatched_bets(market_id);
        self.market_closed_event(
            market_id,
            self.blockchain().get_block_timestamp()
        );
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
        
        if bet.unmatched_amount > BigUint::zero() {
            let refund_amount = bet.unmatched_amount.clone();
            
            self.send().direct(
                &bet.bettor,
                &bet.payment_token,
                bet.payment_nonce,
                &refund_amount,
            );
            
            let original_matched = bet.matched_amount.clone();
            bet.unmatched_amount = BigUint::zero();
            bet.matched_amount = original_matched.clone();
            // Actualizăm stake_amount să reflecte doar partea matched
            bet.stake_amount = original_matched; 
            
            bet.status = if bet.matched_amount > BigUint::zero() {
                BetStatus::Matched
            } else {
                BetStatus::Canceled
            };
            
            self.bet_by_id(bet_nonce).set(&bet);
            self.bet_refunded_event(bet_nonce, &bet.bettor, &refund_amount);
        }
    }

    #[only_owner]
    #[endpoint(setMarketResult)]
    fn set_market_result(
        &self,
        event_id: u64,
        market_type_id: u64,
        score_home: u32,
        score_away: u32
    ) {
        let market_id = self.get_market_id(event_id, market_type_id);
        let mut market = self.markets(market_id).get();
        
        require!(market.market_status == MarketStatus::Closed, "Market not closed");
        
        let market_type = MarketType::from_u64(market_type_id);
        let winning_selection = self.determine_winner(market_type, score_home, score_away);
        
        self.winning_selection(market_id).set(winning_selection);
        self.current_processing_index(market_id).set(0u64);
        
        market.market_status = MarketStatus::Settled;
        self.markets(market_id).set(&market);
    }

    #[endpoint(processBatchBets)]
    fn process_batch_bets(
        &self,
        market_id: u64,
        batch_size: u64
    ) -> ProcessingStatus {
        let market = self.markets(market_id).get();
        require!(
            market.market_status == MarketStatus::Settled,
            "Market not settled"
        );

        let winning_selection = self.winning_selection(market_id).get();
        let mut processed_count = 0u64;

        for bet_id in self.market_bet_ids(market_id).iter() {
            if processed_count >= batch_size {
                return ProcessingStatus::InProgress;
            }

            let mut bet = self.bet_by_id(bet_id).get();
            if bet.matched_amount > BigUint::zero() {
                match bet.bet_type {
                    BetType::Back => {
                        if bet.selection.id == winning_selection {
                            self.process_winning_bet(&mut bet);
                        } else {
                            bet.status = BetStatus::Lost;
                        }
                    },
                    BetType::Lay => {
                        if bet.selection.id != winning_selection {
                            self.process_winning_bet(&mut bet);
                        } else {
                            bet.status = BetStatus::Lost;
                        }
                    }
                }
                self.bet_by_id(bet_id).set(&bet);
                processed_count += 1;
            }
        }
        ProcessingStatus::Completed
    }

    fn process_winning_bet(&self, bet: &mut Bet<Self::Api>) {
        bet.status = BetStatus::Win;
        
        let payout = match bet.bet_type {
            BetType::Back => {
                let mut total_payout = BigUint::zero();
                
                for part in bet.matched_parts.iter() {
                    let part_profit = (part.odds.clone() - BigUint::from(100u32)) * &part.amount / BigUint::from(100u32);
                    total_payout += &part.amount + &part_profit;
                }
                
                total_payout
            },
            BetType::Lay => {
                // Pentru Lay: suma tuturor părților matched
                let mut total_matched = BigUint::zero();
                for part in bet.matched_parts.iter() {
                    total_matched += &part.amount;
                }
                total_matched
            }
        };
        
        self.send().direct(
            &bet.bettor,
            &bet.payment_token,
            bet.payment_nonce,
            &payout
        );
    
        self.reward_distributed_event(
            bet.nft_nonce,
            &bet.bettor,
            &payout
        );
    }

    #[inline]
    fn get_market_id(&self, event_id: u64, market_type_id: u64) -> u64 {
        let markets = self.markets_by_event(event_id).get();
        require!(!markets.is_empty(), "Invalid market");
        markets.get(market_type_id as usize - 1)
    }

    fn determine_winner(
        &self,
        market_type: MarketType,
        score_home: u32,
        score_away: u32
    ) -> u64 {
        match market_type {
            MarketType::FullTimeResult => {
                if score_home > score_away { 1u64 }
                else if score_home < score_away { 2u64 }
                else { 3u64 }
            },
            MarketType::TotalGoals => {
                if score_home + score_away > 2 { 1u64 }
                else { 2u64 }
            },
            MarketType::BothTeamsToScore => {
                if score_home > 0 && score_away > 0 { 1u64 }
                else { 2u64 }
            }
        }
    }

    // View functions
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
        (bet.status, bet.matched_amount, bet.potential_profit)
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