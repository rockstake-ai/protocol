use crate::types::{Market, MarketStatus, Selection};
use crate::priority_queue::PriorityQueue;
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketManagerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::fund_manager::FundManagerModule +
    crate::nft_manager::NftManagerModule
{
    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(
        &self,
        event_id: u64,
        description: ManagedBuffer,
        selection_descriptions: ManagedVec<ManagedBuffer>,
        close_timestamp: u64
    ) -> u64 {
        let market_id = self.get_and_increment_market_counter();
        require!(self.markets(&market_id).is_empty(), "Market already exists");
        
        let created_timestamp = self.blockchain().get_block_timestamp();
        require!(close_timestamp > created_timestamp, "Close timestamp must be in the future");
        
        let mut selections = ManagedVec::new();
        for (index, desc) in selection_descriptions.iter().enumerate() {
            let selection = Selection {
                selection_id: (index + 1) as u64,
                description: desc.as_ref().clone_value(),
                priority_queue: PriorityQueue::new(),
            };
            selections.push(selection);
        }
        
        let market = Market {
            market_id,
            event_id,
            description,
            selections,
            liquidity: BigUint::zero(),
            close_timestamp,
            market_status: MarketStatus::Open,
            total_matched_amount: BigUint::zero(),
            created_timestamp,
        };
        
        self.markets(&market_id).set(&market);
        market_id
    }


    #[view(getMarket)]
    fn get_market(&self, market_id: u64) -> OptionalValue<Market<Self::Api>> {
        if !self.markets(&market_id).is_empty() {
            OptionalValue::Some(self.markets(&market_id).get())
        } else {
            OptionalValue::None
        }
    }


    #[endpoint(updateMarketStatus)]
    fn update_market_status(&self, market_id: u64, new_status: MarketStatus) -> bool {
        if self.markets(&market_id).is_empty() {
            return false;
        }
        
        let mut market = self.markets(&market_id).get();
        market.market_status = new_status;
        self.markets(&market_id).set(&market);
        true
    }

    #[view(getMarketLiquidity)]
    fn get_market_liquidity(&self, market_id: u64) -> MultiValue3<bool, BigUint<Self::Api>, BigUint<Self::Api>> {
        if self.markets(&market_id).is_empty() {
            return (false, BigUint::zero(), BigUint::zero()).into();
        }
        
        let market = self.markets(&market_id).get();
        let mut total_back_liquidity = BigUint::zero();
        let mut total_lay_liquidity = BigUint::zero();
        
        for selection in market.selections.iter() {
            total_back_liquidity += &selection.priority_queue.get_back_liquidity();
            total_lay_liquidity += &selection.priority_queue.get_lay_liquidity();
        }
        
        (true, total_back_liquidity, total_lay_liquidity).into()
    }

    #[view(getBestOdds)]
    fn get_best_odds(&self, market_id: u64, selection_id: u64) -> MultiValue3<bool, BigUint<Self::Api>, BigUint<Self::Api>> {
        if self.markets(&market_id).is_empty() {
            return (false, BigUint::zero(), BigUint::zero()).into();
        }
        
        let market = self.markets(&market_id).get();
        for selection in market.selections.iter() {
            if selection.selection_id == selection_id {
                return (true, 
                        selection.priority_queue.get_best_back_odds(), 
                        selection.priority_queue.get_best_lay_odds()).into();
            }
        }
        
        (false, BigUint::zero(), BigUint::zero()).into()
    }

    fn get_and_increment_market_counter(&self) -> u64 {
        let mut counter = self.market_counter().get();
        counter += 1;
        self.market_counter().set(&counter);
        counter
    }
}