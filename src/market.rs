use crate::types::{Market, MarketStatus, Selection, Tracker};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketModule:
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::fund::FundModule +
    crate::nft::NftModule +
    crate::tracker::TrackerModule +
    crate::validation::ValidationModule
{
    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(
        &self,
        event_id: u64,
        description: ManagedBuffer,
        selection_values: ManagedVec<u64>,
        close_timestamp: u64
    ) -> u64 {
        self.validate_market_creation(close_timestamp);
        
        let market_id = self.get_next_market_id();
        let selections = self.create_selections(market_id, selection_values);
    
        let market = Market {
            market_id,
            event_id,
            description,
            selections,
            liquidity: BigUint::zero(),
            close_timestamp,
            market_status: MarketStatus::Open,
            total_matched_amount: BigUint::zero(),
            created_at: self.blockchain().get_block_timestamp(),
        };
    
        self.markets(market_id).set(&market);
        
        self.markets_by_event(event_id).update(|markets| {
            markets.push(market_id);
        });
    
        self.market_created_event(market_id, event_id, &self.get_current_market_counter());
    
        market_id
    }

    #[endpoint(processEventMarkets)]
    fn process_event_markets(&self, timestamp: u64) {
        let events = self.events_by_timestamp(timestamp).get();
        require!(!events.is_empty(), "No events found for timestamp");

        for event_id in events.iter() {
            let market_ids = self.markets_by_event(event_id).get();
            for market_id in market_ids.iter() {
                self.handle_expired_market(market_id);
            }
        }
    }

    #[endpoint(processMarketClose)]
    fn process_market_close(&self, market_id: u64) {
        let market = self.markets(market_id).get();
        
        require!(
            market.market_status == MarketStatus::Open,
            "Market not open"
        );
        
        require!(
            self.blockchain().get_block_timestamp() >= market.close_timestamp,
            "Market timestamp not reached"
        );

        self.handle_expired_market(market_id);
    }

    fn create_selections(
        &self,
        market_id: u64,
        descriptions: ManagedVec<u64>
    ) -> ManagedVec<Selection<Self::Api>> {
        let mut selections = ManagedVec::new();
        for (index, value) in descriptions.iter().enumerate() {
            let id = (index + 1) as u64;
            self.init_selection_storage(market_id, id);
            let tracker = self.selection_tracker(market_id, id).get();
            selections.push(Selection {
                id,
                value: value,
                priority_queue: tracker,
            });
        }
        selections
    }

    fn init_selection_storage(&self, market_id: u64, selection_id: u64) {
        let tracker = Tracker {
            back_levels: ManagedVec::new(),
            lay_levels: ManagedVec::new(),
            back_liquidity: BigUint::zero(),
            lay_liquidity: BigUint::zero(),
            matched_count: 0,
            unmatched_count: 0,
            partially_matched_count: 0,
            win_count: 0,
            lost_count: 0,
            canceled_count: 0,
        };

        self.selection_tracker(market_id, selection_id).set(&tracker);

        self.selection_back_levels(market_id, selection_id)
            .set(&ManagedVec::new());
        self.selection_lay_levels(market_id, selection_id)
            .set(&ManagedVec::new());

        self.selection_back_liquidity(market_id, selection_id)
            .set(&BigUint::zero());
        self.selection_lay_liquidity(market_id, selection_id)
            .set(&BigUint::zero());

        self.selection_matched_count(market_id, selection_id).set(&0u64);
        self.selection_unmatched_count(market_id, selection_id).set(&0u64);
        self.selection_partially_matched_count(market_id, selection_id).set(&0u64);
        self.selection_win_count(market_id, selection_id).set(&0u64);
        self.selection_lost_count(market_id, selection_id).set(&0u64);
        self.selection_canceled_count(market_id, selection_id).set(&0u64);

        self.total_matched_amount(market_id, selection_id).set(&BigUint::zero());
    }

    fn get_selection(
        &self,
        market: &Market<Self::Api>,
        selection_id: u64
    ) -> Selection<Self::Api> {
        market.selections.iter()
            .find(|s| s.id == selection_id)
            .unwrap_or_else(|| sc_panic!("Selection not found"))
    }

    #[view(getMarketStatus)]
    fn get_market_status(&self, market_id: u64) -> MarketStatus {
        self.markets(market_id).get().market_status
    }

    #[view(getCurrentMarketCounter)]
    fn get_current_market_counter(&self) -> u64 {
        if self.market_counter().is_empty() {
            return 0;
        }
        self.market_counter().get()
    }
}