use crate::storage::{Market, Selection, Status};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketManagerModule: 
    crate::storage::StorageModule+
    crate::events::EventsModule +
    crate::fund_manager::FundManagerModule
    + crate::nft_manager::NftManagerModule{
    
    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(
        &self,
        event_id: u64,
        description: ManagedBuffer,
        selections: ManagedVec<Selection<Self::Api>>,
        close_timestamp: u64
    ) -> u64{
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
        market_id
    }

    fn get_and_increment_market_counter(&self) -> u64 {
        let mut counter = self.market_counter().get();
        counter += 1;
        self.market_counter().set(&counter);
        counter
    }

    #[only_owner]
    #[endpoint(closeExpiredMarkets)]
    fn close_expired_markets(&self) {
        let current_timestamp = self.blockchain().get_block_timestamp();
        let mut closed_markets = ManagedVec::new();
        
        let total_markets = self.market_counter().get();
        
        let one = u64::from(1u32);
        let mut market_id = one.clone();

        while market_id <= total_markets {
            if !self.markets(&market_id).is_empty() {
                let mut market = self.markets(&market_id).get();
                if current_timestamp >= market.close_timestamp && !self.is_market_closed(&market) {
                    for bet in market.bets.iter() {
                        if bet.status == Status::Unmatched {
                            self.distribute_rewards(bet.nft_nonce);
                        }
                    }
                    self.markets(&market_id).set(&market);
                    closed_markets.push(market_id.clone());
                }
            }
            market_id += &one;
        }
        
        self.expired_markets_closed_event(closed_markets);
    }

    fn is_market_closed(&self, market: &Market<Self::Api>) -> bool {
        market.bets.iter().all(|bet| 
            matches!(bet.status, Status::Win | Status::Lost | Status::Canceled)
        )
    }
}