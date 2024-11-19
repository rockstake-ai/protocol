use crate::types::{Bet, BetStatus, BetType, MarketStatus, PriceLevel, Selection};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FundModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft::NftModule
{
    fn handle_expired_market(&self, market_id: u64) -> SCResult<()> {
        let mut market = self.markets(market_id).get();
        
        market.market_status = MarketStatus::Closed;
        self.markets(market_id).set(&market);
        self.process_winning_bets(market_id)?;
        self.process_unmatched_bets(market_id)?;
        self.market_closed_event(
            market_id,
            self.blockchain().get_block_timestamp()
        );

        Ok(())
    }

    fn process_winning_bets(&self, market_id: u64) -> SCResult<()> {
        let market = self.markets(market_id).get();
        
        for selection in market.selections.iter() {
            let back_levels = self.selection_back_levels(market_id, selection.id).get();
            for level in back_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    let bet = self.bet_by_id(bet_nonce).get();
                    if bet.status == BetStatus::Win && bet.matched_amount > BigUint::zero() {
                        self.distribute_bet_reward(&bet)?;
                    }
                }
            }

            // Procesăm lay bets câștigătoare
            let lay_levels = self.selection_lay_levels(market_id, selection.id).get();
            for level in lay_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    let bet = self.bet_by_id(bet_nonce).get();
                    if bet.status == BetStatus::Win && bet.matched_amount > BigUint::zero() {
                        self.distribute_bet_reward(&bet)?;
                    }
                }
            }
        }

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

            self.selection_back_levels(market_id, selection.id)
                .set(&ManagedVec::new());
            self.selection_lay_levels(market_id, selection.id)
                .set(&ManagedVec::new());
            self.selection_back_liquidity(market_id, selection.id)
                .set(&BigUint::zero());
            self.selection_lay_liquidity(market_id, selection.id)
                .set(&BigUint::zero());
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

    fn distribute_bet_reward(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        let amount_to_distribute = match bet.bet_type {
            BetType::Back => bet.potential_profit.clone(),
            BetType::Lay => &bet.liability - &bet.potential_profit,
        };

        if amount_to_distribute > BigUint::zero() {
            let payment = EgldOrEsdtTokenPayment::new(
                bet.payment_token.clone(),
                bet.payment_nonce,
                amount_to_distribute.clone(),
            );

            self.send().direct(
                &bet.bettor,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );

            self.reward_distributed_event(
                bet.nft_nonce,
                &bet.bettor,
                &amount_to_distribute
            );
        }

        Ok(())
    }

    #[only_owner]
    #[endpoint(validateMatchResults)]
    fn validate_match_results(
        &self,
        event_id: u64,
        score_home: u32,
        score_away: u32
    ) -> SCResult<()> {
        // Get all markets for this event
        let markets = self.markets_by_event(event_id).get();
        
        for market_id in markets.iter() {
            let mut market = self.markets(market_id).get();
            require!(
                market.market_status == MarketStatus::Closed, 
                "Market not closed"
            );

            // Determine winning selections based on market type
            let winning_selections = self.get_winning_selections(
                &market.description,
                score_home,
                score_away
            )?;

            // Process each selection
            for selection in market.selections.iter() {
                let is_winning = winning_selections.contains(&selection.id);
                
                // Process back bets first
                let back_levels = self.selection_back_levels(market_id, selection.id).get();
                for level in back_levels.iter() {
                    self.process_bets_at_level(
                        &level,
                        is_winning,
                        BetType::Back
                    )?;
                }
                self.selection_back_levels(market_id, selection.id).clear();

                // Then process lay bets
                let lay_levels = self.selection_lay_levels(market_id, selection.id).get();
                for level in lay_levels.iter() {
                    self.process_bets_at_level(
                        &level,
                        is_winning,
                        BetType::Lay
                    )?;
                }
                self.selection_lay_levels(market_id, selection.id).clear();
            }

            market.market_status = MarketStatus::Settled;
            self.markets(market_id).set(&market);
        }
        
        Ok(())
    }

    fn get_winning_selections(
        &self,
        market_type: &ManagedBuffer,
        score_home: u32,
        score_away: u32
    ) -> SCResult<ManagedVec<u64>> {
        let mut winning_selections = ManagedVec::new();

        match market_type.to_boxed_bytes().as_slice() {
            b"FullTime Result" => {
                if score_home > score_away {
                    winning_selections.push(1u64); // Home Win
                } else if score_home < score_away {
                    winning_selections.push(2u64); // Away Win
                } else {
                    winning_selections.push(3u64); // Draw
                }
            },
            b"Total Goals O/U 2.5" => {
                if (score_home + score_away) > 2 {
                    winning_selections.push(1u64); // Over
                } else {
                    winning_selections.push(2u64); // Under
                }
            },
            b"Both Teams To Score" => {
                if score_home > 0 && score_away > 0 {
                    winning_selections.push(1u64); // Yes
                } else {
                    winning_selections.push(2u64); // No
                }
            },
            _ => return sc_error!("Invalid market type")
        }

        Ok(winning_selections)
    }

    fn process_bets_at_level(
        &self,
        level: &PriceLevel<Self::Api>,
        is_winning: bool,
        bet_type: BetType
    ) -> SCResult<()> {
        for bet_nonce in level.bet_nonces.iter() {
            let mut bet = self.bet_by_id(bet_nonce).get();
            
            if bet.matched_amount > BigUint::zero() {
                let should_win = match bet_type {
                    BetType::Back => is_winning,
                    BetType::Lay => !is_winning
                };

                if should_win {
                    bet.status = BetStatus::Win;
                    // Calculate payout based on bet type
                    let payout = if bet_type == BetType::Back {
                        &bet.matched_amount + &bet.potential_profit
                    } else {
                        bet.matched_amount.clone()
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
                } else {
                    bet.status = BetStatus::Lost;
                }
                
                self.bet_by_id(bet_nonce).set(&bet);
            }
        }

        Ok(())
    }

    // Utility function to check if a market exists for an event
    #[view(getMarketIdForEventAndType)]
    fn get_market_id_for_event_and_type(
        &self,
        event_id: u64,
        market_type: ManagedBuffer
    ) -> Option<u64> {
        let markets = self.markets_by_event(event_id).get();
        for market_id in markets.iter() {
            let market = self.markets(market_id).get();
            if market.description == market_type {
                return Some(market_id);
            }
        }
        None
    }

    // View function to get market status
    #[view(getMarketStatus)]
    fn get_market_status(&self, market_id: u64) -> MarketStatus {
        self.markets(market_id).get().market_status
    }

    fn get_winning_selection(
        &self,
        market_type: &ManagedBuffer,
        score_home: u32,
        score_away: u32
    ) -> SCResult<u64> {
        match market_type.to_boxed_bytes().as_slice() {
            b"FullTime Result" => {
                if score_home > score_away {
                    Ok(1) // Home win
                } else if score_home < score_away {
                    Ok(2) // Away win
                } else {
                    Ok(3) // Draw
                }
            },
            b"Total Goals O/U 2.5" => {
                if (score_home + score_away) > 2 {
                    Ok(1) // Over
                } else {
                    Ok(2) // Under
                }
            },
            b"Both Teams To Score" => {
                if score_home > 0 && score_away > 0 {
                    Ok(1) // Yes
                } else {
                    Ok(2) // No
                }
            },
            _ => sc_error!("Invalid market type")
        }
    }

    fn process_market_selections(
        &self,
        market_id: u64,
        selections: &ManagedVec<Selection<Self::Api>>,
        winning_selection: u64
    ) -> SCResult<()> {
        for selection in selections.iter() {
            let is_winning = selection.id == winning_selection;
            self.process_all_bets(market_id, selection.id, is_winning)?;
            
            // Cleanup
            self.selection_back_levels(market_id, selection.id).clear();
            self.selection_lay_levels(market_id, selection.id).clear();
        }
        Ok(())
    }

    fn process_all_bets(
        &self,
        market_id: u64,
        selection_id: u64,
        is_winning: bool
    ) -> SCResult<()> {
        let back_levels = self.selection_back_levels(market_id, selection_id).get();
        let lay_levels = self.selection_lay_levels(market_id, selection_id).get();
    
        for level in back_levels.iter().chain(lay_levels.iter()) {
            for bet_nonce in level.bet_nonces.iter() {
                let mut bet = self.bet_by_id(bet_nonce).get();
                if bet.matched_amount > BigUint::zero() {
                    let is_bet_winning = if bet.bet_type == BetType::Back {
                        is_winning
                    } else {
                        !is_winning
                    };
    
                    if is_bet_winning {
                        bet.status = BetStatus::Win;
                        let amount = if bet.bet_type == BetType::Back {
                            &bet.matched_amount + &bet.matched_amount
                        } else {
                            bet.matched_amount.clone()
                        };
    
                        self.send().direct(
                            &bet.bettor,
                            &bet.payment_token,
                            bet.payment_nonce,
                            &amount
                        );
                    } else {
                        bet.status = BetStatus::Lost;
                    }
                    
                    self.bet_by_id(bet_nonce).set(&bet);
                }
            }
        }
    
        Ok(())
    }
}