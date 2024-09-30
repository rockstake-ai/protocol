use crate::storage::{self, Bet, Market, Selection, Status};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketManagerModule: storage::StorageModule{
    
    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(
        &self,
        event_id: BigUint,
        description: ManagedBuffer,
        selections: ManagedVec<Selection<Self::Api>>,
        close_timestamp: u64
    ) {
        let market_id = self.get_and_increment_market_counter();
        require!(self.markets(&market_id).is_empty(), "Market already exists");
        
        let current_timestamp = self.blockchain().get_block_timestamp();
        require!(close_timestamp > current_timestamp, "Close timestamp must be in the future");

        let market = Market {
            market_id: market_id.clone(),
            event_id,
            description,
            selections,
            back_liquidity: BigUint::zero(),
            lay_liquidity: BigUint::zero(),
            best_back_odds: BigUint::zero(),
            best_lay_odds: BigUint::zero(),
            bets: ManagedVec::new(),
            close_timestamp,
        };
        self.markets(&market_id).set(&market);
    }

    fn get_and_increment_market_counter(&self) -> BigUint {
        let mut counter = self.market_counter().get();
        counter += 1u32;
        self.market_counter().set(&counter);
        counter
    }

    #[only_owner]
    #[endpoint(closeMarket)]
    fn close_market(&self, market_id: BigUint, winning_selection_id: BigUint) {
        let mut market = self.markets(&market_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
        
        require!(current_timestamp >= market.close_timestamp, "Market is not yet closed");

        for mut bet in market.bets.iter_mut() {
            match bet.status {
                Status::Unmatched => {
                    self.refund_unmatched_bet(&bet);
                    bet.status = Status::Canceled;
                },
                Status::Matched => {
                    if bet.selection.selection_id == winning_selection_id {
                        bet.status = Status::Win;
                    } else {
                        bet.status = Status::Lost;
                    }
                },
                _ => {}, // Alte statusuri rămân neschimbate
            }
        }
        
        self.markets(&market_id).set(&market);
        self.market_closed_event(market_id, winning_selection_id);
    }

    #[endpoint(closeExpiredMarkets)]
    fn close_expired_markets(&self) {
        let current_timestamp = self.blockchain().get_block_timestamp();
        let mut closed_markets = ManagedVec::new();

        let total_markets = self.market_counter().get();
        for market_id in 1..=total_markets {
            let market_id = BigUint::from(market_id);
            if !self.markets(&market_id).is_empty() {
                let mut market = self.markets(&market_id).get();
                if current_timestamp >= market.close_timestamp && !self.is_market_closed(&market) {
                    for bet in market.bets.iter_mut() {
                        if bet.status == Status::Unmatched {
                            self.refund_unmatched_bet(bet);
                            bet.status = Status::Canceled;
                        }
                    }
                    self.markets(&market_id).set(&market);
                    closed_markets.push(market_id);
                }
            }
        }

        self.expired_markets_closed_event(closed_markets);
    }


    //TODO - move to FundManager
    fn refund_unmatched_bet(&self, bet: &Bet<Self::Api>) {
        self.send().direct_esdt(
            &bet.bettor,
            &bet.payment_token,
            bet.payment_nonce,
            &bet.stake_amount,
        );
    }

    fn is_market_closed(&self, market: &Market<Self::Api>) -> bool {
        market.bets.iter().all(|bet| 
            matches!(bet.status, Status::Win | Status::Lost | Status::Canceled)
        )
    }


}