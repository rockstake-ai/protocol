use crate::types::{Bet, BetType, BetStatus, Market, MarketStatus};
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
            let back_levels = self.selection_back_levels(market_id, selection.selection_id).get();
            for level in back_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    let bet = self.bet_by_id(bet_nonce).get();
                    if bet.status == BetStatus::Win && bet.matched_amount > BigUint::zero() {
                        self.distribute_bet_reward(&bet)?;
                    }
                }
            }

            // Procesăm lay bets câștigătoare
            let lay_levels = self.selection_lay_levels(market_id, selection.selection_id).get();
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
            let back_levels = self.selection_back_levels(market_id, selection.selection_id).get();
            for level in back_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.process_unmatched_bet(bet_nonce)?;
                }
            }

            let lay_levels = self.selection_lay_levels(market_id, selection.selection_id).get();
            for level in lay_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.process_unmatched_bet(bet_nonce)?;
                }
            }

            self.selection_back_levels(market_id, selection.selection_id)
                .set(&ManagedVec::new());
            self.selection_lay_levels(market_id, selection.selection_id)
                .set(&ManagedVec::new());
            self.selection_back_liquidity(market_id, selection.selection_id)
                .set(&BigUint::zero());
            self.selection_lay_liquidity(market_id, selection.selection_id)
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
}