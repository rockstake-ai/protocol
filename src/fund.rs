use crate::types::{Bet, BetStatus, BetType, MarketStatus, MarketType, PriceLevel, Selection};
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
        score_home: u32,
        score_away: u32
    ) -> SCResult<()> {
        let market_type = MarketType::from_u64(market_type_id)?;
        let market_id = self.find_market_by_type(event_id, &market_type)?;
        
        // Validăm și salvăm rezultatul
        self.save_market_result(market_id, score_home, score_away)?;
        
        Ok(())
    }

    // Step 2: Găsim market-ul și validăm starea
    fn find_market_by_type(
        &self,
        event_id: u64,
        market_type: &MarketType,
    ) -> SCResult<u64> {
        let markets = self.markets_by_event(event_id).get();
        for market_id in markets.iter() {
            let market = self.markets(market_id).get();
            require!(
                market.market_status == MarketStatus::Closed,
                "Market not closed"
            );

            let current_type = self.get_market_type(&market.description)?;
            if current_type == *market_type {
                return Ok(market_id);
            }
        }
        sc_error!("Market not found")
    }

    // Step 3: Salvăm rezultatul pentru procesare ulterioară
    fn save_market_result(
        &self,
        market_id: u64,
        score_home: u32,
        score_away: u32
    ) -> SCResult<()> {
        let mut market = self.markets(market_id).get();
        let market_type = self.get_market_type(&market.description)?;
        
        let winning_selection = self.determine_winner(
            &market_type,
            score_home,
            score_away
        )?;

        // Salvăm rezultatul pentru procesare ulterioară
        self.market_results(market_id).set(winning_selection);
        
        // Marcăm market-ul pentru procesare
        market.market_status = MarketStatus::Settled;
        self.markets(market_id).set(&market);
        
        // Adăugăm market-ul la coada de procesare
        self.markets_to_process().push(&market_id);
        
        Ok(())
    }

    // Step 4: Procesăm un singur nivel de pariuri
    #[endpoint(processBetLevel)]
    fn process_bet_level(
        &self,
        market_id: u64,
        selection_id: u64,
        is_back: bool
    ) -> SCResult<()> {
        require!(
            self.markets(market_id).get().market_status == MarketStatus::Settled,
            "Market not settled"
        );

        let winning_selection = self.market_results(market_id).get();
        let is_winning = selection_id == winning_selection;

        let levels = if is_back {
            self.selection_back_levels(market_id, selection_id).get()
        } else {
            self.selection_lay_levels(market_id, selection_id).get()
        };

        if !levels.is_empty() {
            // Procesăm doar primul nivel
            let level = levels.get(0);
            self.process_single_level(
                market_id,
                selection_id,
                &level,
                is_winning,
                is_back
            )?;

            // Actualizăm nivelurile rămase
            let mut remaining_levels = ManagedVec::new();
            for i in 1..levels.len() {
                remaining_levels.push(levels.get(i));
            }

            if is_back {
                self.selection_back_levels(market_id, selection_id).set(&remaining_levels);
            } else {
                self.selection_lay_levels(market_id, selection_id).set(&remaining_levels);
            }
        }

        Ok(())
    }

    // Step 5: Procesăm pariurile dintr-un singur nivel
    fn process_single_level(
        &self,
        market_id: u64,
        selection_id: u64,
        level: &PriceLevel<Self::Api>,
        is_winning: bool,
        is_back: bool
    ) -> SCResult<()> {
        for bet_nonce in level.bet_nonces.iter() {
            let mut bet = self.bet_by_id(bet_nonce).get();
            if bet.matched_amount > BigUint::zero() {
                let should_win = if is_back { is_winning } else { !is_winning };

                if should_win {
                    bet.status = BetStatus::Win;
                    let payout = if is_back {
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

    // Storage mappers
    #[storage_mapper("marketResults")]
    fn market_results(&self, market_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("marketsToProcess")]
    fn markets_to_process(&self) -> VecMapper<u64>;

    // Helpers
    fn get_market_type(&self, description: &ManagedBuffer) -> SCResult<MarketType> {
        match description.to_boxed_bytes().as_slice() {
            b"FullTime Result" => Ok(MarketType::FullTimeResult),
            b"Total Goals O/U 2.5" => Ok(MarketType::TotalGoals),
            b"Both Teams To Score" => Ok(MarketType::BothTeamsToScore),
            _ => sc_error!("Invalid market type")
        }
    }

    fn determine_winner(
        &self,
        market_type: &MarketType,
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