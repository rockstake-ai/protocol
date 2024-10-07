use crate::types::{Bet, Market, MarketStatus, Selection};

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
        
        let created_timestamp = self.blockchain().get_block_timestamp();
        require!(close_timestamp > created_timestamp, "Close timestamp must be in the future");
    
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
            market_status: MarketStatus::Open,
            total_matched_amount: BigUint::zero(),
            close_timestamp,
            created_timestamp,
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
}