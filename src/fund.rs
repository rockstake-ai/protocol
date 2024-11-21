use crate::types::{Bet, BetStatus, BetType, MarketStatus, MarketType, Selection};
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
    #[endpoint(validateMatchResults)]
    fn validate_match_results(
        &self,
        event_id: u64,
        market_type_id: u64,
        score_home: u64,
        score_away: u64
    ) -> SCResult<()> {
        let market_type = MarketType::from_u64(market_type_id)?;
        let market_id = self.get_market_id_for_event_and_type(event_id, &market_type)?;
        let mut market = self.markets(market_id).get();
        require!(market.market_status == MarketStatus::Closed, "Market not closed");

        let winning_selection = self.determine_winner(
            &market_type,
            score_home,
            score_away
        )?;

        self.process_market_selections(market_id, &market.selections, winning_selection)?;

        market.market_status = MarketStatus::Settled;
        self.markets(market_id).set(&market);
        
        Ok(())
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
                        let payout = if bet.bet_type == BetType::Back {
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
        }
    
        Ok(())
    }

    fn determine_winner(
        &self,
        market_type: &MarketType,
        score_home: u64,
        score_away: u64
    ) -> SCResult<u64> {
        match market_type {
            MarketType::FullTimeResult => {
                Ok(if score_home > score_away {
                    1u64 // Home Win
                } else if score_home < score_away {
                    2u64 // Away Win
                } else {
                    3u64 // Draw
                })
            },
            MarketType::TotalGoals => {
                Ok(if score_home + score_away > 2 {
                    1u64 // Over
                } else {
                    2u64 // Under
                })
            },
            MarketType::BothTeamsToScore => {
                Ok(if score_home > 0 && score_away > 0 {
                    1u64 // Yes
                } else {
                    2u64 // No
                })
            }
        }
    }

    fn get_market_id_for_event_and_type(
        &self,
        event_id: u64,
        market_type: &MarketType,
    ) -> SCResult<u64> {
        let markets = self.markets_by_event(event_id).get();
        
        for market_id in markets.iter() {
            let market = self.markets(market_id).get();
            if market.description.to_boxed_bytes().as_slice() == market_type.to_description() {
                return Ok(market_id);
            }
        }
        
        sc_error!("Market not found")
    }

    // Utility function to get market type from stored market
    #[view(getMarketType)]
    fn get_market_type(&self, market_id: u64) -> SCResult<MarketType> {
        let market = self.markets(market_id).get();
        let description = market.description.to_boxed_bytes();
        
        match description.as_slice() {
            b"FullTime Result" => Ok(MarketType::FullTimeResult),
            b"Total Goals O/U 2.5" => Ok(MarketType::TotalGoals),
            b"Both Teams To Score" => Ok(MarketType::BothTeamsToScore),
            _ => sc_error!("Invalid market type")
        }
    }
}