use crate::types::{Bet, Market, MarketStatus, Selection, Tracker};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketManagerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::fund_manager::FundManagerModule +
    crate::nft_manager::NftManagerModule +
    crate::tracker::TrackerModule
{
    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(
        &self,
        event_id: u64,
        description: ManagedBuffer,
        selection_descriptions: ManagedVec<ManagedBuffer>,
        close_timestamp: u64
    ) -> SCResult<u64> {
        require!(
            close_timestamp > self.blockchain().get_block_timestamp(),
            "Close timestamp must be in the future"
        );

        let market_id = self.get_and_increment_market_counter();
        require!(self.markets(market_id).is_empty(), "Market already exists");

        let selections = self.create_selections(market_id, selection_descriptions)?;

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
        
        // Emit event after successful market creation
        self.market_created_event(market_id, event_id, &self.get_current_market_counter());

        Ok(market_id)
    }

    fn get_and_increment_market_counter(&self) -> u64 {
        let mut counter = self.market_counter().get();
        counter += 1;
        self.market_counter().set(&counter);
        counter
    }

    fn create_selections(
        &self,
        market_id: u64,
        descriptions: ManagedVec<ManagedBuffer>
    ) -> SCResult<ManagedVec<Selection<Self::Api>>> {
        let mut selections = ManagedVec::new();
        
        for (index, desc) in descriptions.iter().enumerate() {
            let selection_id = (index + 1) as u64;
            
            // Inițializăm storage-ul pentru selection
            self.init_selection_storage(market_id, selection_id);
            
            // Obținem tracker-ul inițializat
            let tracker = self.selection_tracker(market_id, selection_id).get();

            selections.push(Selection {
                selection_id,
                description: desc.as_ref().clone_value(),
                priority_queue: tracker,
            });
        }
        
        Ok(selections)
    }

    fn init_selection_storage(&self, market_id: u64, selection_id: u64) {
        // Inițializăm un nou tracker
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

        // Salvăm tracker-ul
        self.selection_tracker(market_id, selection_id).set(&tracker);

        // Inițializăm storage-ul pentru levels
        self.selection_back_levels(market_id, selection_id)
            .set(&ManagedVec::new());
        self.selection_lay_levels(market_id, selection_id)
            .set(&ManagedVec::new());

        // Inițializăm lichiditatea
        self.selection_back_liquidity(market_id, selection_id)
            .set(&BigUint::zero());
        self.selection_lay_liquidity(market_id, selection_id)
            .set(&BigUint::zero());

        // Inițializăm contoarele
        self.selection_matched_count(market_id, selection_id).set(&0u64);
        self.selection_unmatched_count(market_id, selection_id).set(&0u64);
        self.selection_partially_matched_count(market_id, selection_id).set(&0u64);
        self.selection_win_count(market_id, selection_id).set(&0u64);
        self.selection_lost_count(market_id, selection_id).set(&0u64);
        self.selection_canceled_count(market_id, selection_id).set(&0u64);

        // Inițializăm total matched amount
        self.total_matched_amount(market_id, selection_id).set(&BigUint::zero());
    }

    fn get_selection(
        &self,
        market: &Market<Self::Api>,
        selection_id: u64
    ) -> SCResult<Selection<Self::Api>> {
        let selection = market.selections.iter()
            .find(|s| s.selection_id == selection_id)
            .ok_or("Selection not found")?;
        Ok(selection)
    }

    #[view(isMarketOpen)]
    fn is_market_open(&self, market_id: u64) -> bool {
        if self.markets(market_id).is_empty() {
            return false;
        }
        let market = self.markets(market_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
        current_timestamp < market.close_timestamp
    }

    #[view(getMarket)]
    fn get_market(&self, market_id: u64) -> SCResult<Market<Self::Api>> {
        require!(!self.markets(market_id).is_empty(), "Market does not exist");
        Ok(self.markets(market_id).get())
    }

    // Views adiționale pentru informații despre market
    #[view(getMarketSelections)]
    fn get_market_selections(&self, market_id: u64) -> SCResult<ManagedVec<Selection<Self::Api>>> {
        require!(!self.markets(market_id).is_empty(), "Market does not exist");
        let market = self.markets(market_id).get();
        Ok(market.selections)
    }

    #[view(getSelectionTracker)]
    fn get_selection_tracker(&self, market_id: u64, selection_id: u64) -> SCResult<Tracker<Self::Api>> {
        require!(!self.markets(market_id).is_empty(), "Market does not exist");
        Ok(self.selection_tracker(market_id, selection_id).get())
    }

    #[view(getCurrentMarketCounter)]
    fn get_current_market_counter(&self) -> u64 {
        if self.market_counter().is_empty() {
            return 0;
        }
        self.market_counter().get()
    }

    #[view(checkMarketExists)]
    fn check_market_exists(&self, market_id: u64) -> bool {
        !self.markets(market_id).is_empty()
    }
}
