use crate::types::{Bet, BetStatus, BetType, MarketStatus, MarketType, ProcessingProgress, ProcessingStatus};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FundModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft::NftModule
{
    // Handling un-matched bets when market expires/closes
    fn handle_expired_market(&self, market_id: u64) -> SCResult<()> {
        let mut market = self.markets(market_id).get();
        
        market.market_status = MarketStatus::Closed;
        self.markets(market_id).set(&market);
        self.process_unmatched_bets(market_id)?;
        self.market_closed_event(
            market_id,
            self.blockchain().get_block_timestamp()
        );

        Ok(())
    }

    fn process_unmatched_bets(&self, market_id: u64) -> SCResult<()> {
        let market = self.markets(market_id).get();
        
        for selection in market.selections.iter() {
            let back_levels = self.selection_back_levels(market_id, selection.id).get();
            for level in back_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.process_unmatched_bet(bet_nonce)?;
                }
            }

            let lay_levels = self.selection_lay_levels(market_id, selection.id).get();
            for level in lay_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.process_unmatched_bet(bet_nonce)?;
                }
            }

            // Clear storage after processing
            self.selection_back_levels(market_id, selection.id).clear();
            self.selection_lay_levels(market_id, selection.id).clear();
            self.selection_back_liquidity(market_id, selection.id).set(&BigUint::zero());
            self.selection_lay_liquidity(market_id, selection.id).set(&BigUint::zero());
        }

        Ok(())
    }

    fn process_unmatched_bet(&self, bet_nonce: u64) -> SCResult<()> {
        let mut bet = self.bet_by_id(bet_nonce).get();
        
        if bet.unmatched_amount > BigUint::zero() {
            let refund_amount = match bet.bet_type {
                BetType::Back => bet.unmatched_amount.clone(),
                BetType::Lay => bet.unmatched_amount.clone()
            };
    
            if refund_amount > BigUint::zero() {
                let payment = EgldOrEsdtTokenPayment::new(
                    bet.payment_token.clone(),
                    bet.payment_nonce,
                    refund_amount.clone(),
                );
    
                self.send().direct(
                    &bet.bettor,
                    &payment.token_identifier,
                    payment.token_nonce,
                    &payment.amount,
                );
    
                bet.status = BetStatus::Canceled;
                bet.unmatched_amount = BigUint::zero();
                self.bet_by_id(bet_nonce).set(&bet);
    
                self.bet_refunded_event(
                    bet_nonce,
                    &bet.bettor,
                    &refund_amount
                );
            }
        }
    
        Ok(())
    }

    #[only_owner]
    #[endpoint(setMarketResult)]
    fn set_market_result(
        &self,
        event_id: u64,
        market_type_id: u64,
        score_home: u32,
        score_away: u32
    ) -> SCResult<()> {
        let market_type = MarketType::from_u64(market_type_id)?;
        let market_id = self.get_market_id(event_id, market_type_id)?;
        let mut market = self.markets(market_id).get();
        
        require!(market.market_status == MarketStatus::Closed, "Market not closed");

        // Determinăm selecția câștigătoare și o salvăm
        let winning_selection = self.determine_winner(market_type, score_home, score_away)?;
        self.winning_selection(market_id).set(winning_selection);

        // Inițializăm indexul de procesare pentru acest market
        self.current_processing_index(market_id).set(0u64);

        // Setăm statusul și numărul total de pariuri pentru tracking
        market.market_status = MarketStatus::Settled;
        self.markets(market_id).set(&market);

        Ok(())
    }

    // Pasul 2: Procesăm pariurile în loturi
    #[endpoint(processBatchBets)]
    fn process_batch_bets(
        &self,
        market_id: u64,
        batch_size: u64
    ) -> SCResult<ProcessingStatus> {
        require!(
            self.markets(market_id).get().market_status == MarketStatus::Settled,
            "Market not settled"
        );

        let winning_selection = self.winning_selection(market_id).get();
        let mut current_index = self.current_processing_index(market_id).get();
        let mut processed_in_batch = 0u64;

        // Parcurgem fiecare selecție
        let market = self.markets(market_id).get();
        for selection in market.selections.iter() {
            let is_winning = selection.id == winning_selection;

            // Procesăm back bets
            processed_in_batch += self.process_selection_bets(
                market_id,
                selection.id,
                is_winning,
                true,
                current_index,
                batch_size
            )?;

            if processed_in_batch >= batch_size {
                break;
            }

            // Procesăm lay bets
            processed_in_batch += self.process_selection_bets(
                market_id,
                selection.id,
                is_winning,
                false,
                current_index,
                batch_size - processed_in_batch
            )?;

            if processed_in_batch >= batch_size {
                break;
            }
        }

        // Actualizăm indexul de procesare
        if processed_in_batch > 0 {
            current_index += processed_in_batch;
            self.current_processing_index(market_id).set(current_index);
            Ok(ProcessingStatus::InProgress)
        } else {
            // Nu mai avem pariuri de procesat
            self.current_processing_index(market_id).clear();
            Ok(ProcessingStatus::Completed)
        }
    }

    fn process_selection_bets(
        &self,
        market_id: u64,
        selection_id: u64,
        is_winning: bool,
        is_back: bool,
        start_index: u64,
        max_to_process: u64
    ) -> SCResult<u64> {
        let levels = if is_back {
            self.selection_back_levels(market_id, selection_id).get()
        } else {
            self.selection_lay_levels(market_id, selection_id).get()
        };

        let mut processed_count = 0u64;

        for level in levels.iter() {
            for bet_nonce in level.bet_nonces.iter() {
                if processed_count >= max_to_process {
                    return Ok(processed_count);
                }

                let mut bet = self.bet_by_id(bet_nonce).get();
                if bet.matched_amount > BigUint::zero() {
                    let should_win = if is_back { is_winning } else { !is_winning };
                    
                    if should_win {
                        bet.status = BetStatus::Win;
                        
                        // Calculăm și distribuim câștigul
                        let payout = if is_back {
                            &bet.matched_amount + &bet.potential_profit
                        } else {
                            bet.matched_amount.clone()
                        };

                        // Facem plata
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
                    } else {
                        bet.status = BetStatus::Lost;
                    }
                    
                    self.bet_by_id(bet_nonce).set(&bet);
                    processed_count += 1;
                }
            }
        }

        Ok(processed_count)
    }

    // Storage mappers
    #[storage_mapper("winningSelection")]
    fn winning_selection(&self, market_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("currentProcessingIndex")]
    fn current_processing_index(&self, market_id: u64) -> SingleValueMapper<u64>;

    // View functions pentru monitorizare
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

    #[inline]
    fn get_market_id(&self, event_id: u64, market_type_id: u64) -> SCResult<u64> {
        let markets = self.markets_by_event(event_id).get();
        require!(!markets.is_empty(), "No markets found");
        Ok(markets.get(market_type_id as usize - 1))
    }

    #[inline]
    fn determine_winner(
        &self,
        market_type: MarketType,
        score_home: u32,
        score_away: u32
    ) -> SCResult<u64> {
        match market_type {
            MarketType::FullTimeResult => {
                Ok(if score_home > score_away { 1u64 }
                   else if score_home < score_away { 2u64 }
                   else { 3u64 })
            },
            MarketType::TotalGoals => {
                Ok(if score_home + score_away > 2 { 1u64 }
                   else { 2u64 })
            },
            MarketType::BothTeamsToScore => {
                Ok(if score_home > 0 && score_away > 0 { 1u64 }
                   else { 2u64 })
            }
        }
    }
}