use crate::{errors::ERR_INVALID_MARKET, types::{Market, MarketStatus, Selection, Tracker}};
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
    ) -> SCResult<u64> {
        // Validări
        self.validate_market_creation(close_timestamp)?;
        
        // Obținem următorul ID valid
        let market_id = self.get_and_validate_next_market_id()?;
        
        // Creăm selecțiile
        let selections = self.create_selections(market_id, selection_values)?;

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
        
        // Emit event
        self.market_created_event(market_id, event_id, &self.get_current_market_counter());

        Ok(market_id)
    }

    #[endpoint(processMarketClose)]
    fn process_market_close(&self, market_id: u64) -> SCResult<()> {
        let market = self.markets(market_id).get();
        
        require!(
            market.market_status == MarketStatus::Open,
            "Market is not in open state"
        );
        
        require!(
            self.blockchain().get_block_timestamp() >= market.close_timestamp,
            "Market has not reached close timestamp yet"
        );

        self.handle_expired_market(market_id)
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
        descriptions: ManagedVec<u64>
    ) -> SCResult<ManagedVec<Selection<Self::Api>>> {
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
            .find(|s| s.id == selection_id)
            .ok_or("Selection not found")?;
        Ok(selection)
    }

    #[view(isMarketOpen)]
    fn is_market_open(&self, market_id: u64) -> bool {
        if self.markets(market_id).is_empty() {
            return false;
        }
        let market = self.markets(market_id).get();
        market.market_status == MarketStatus::Open
    }

    #[view(getMarket)]
    fn get_market(&self, market_id: u64) -> SCResult<Market<Self::Api>> {
        require!(!self.markets(market_id).is_empty(), ERR_INVALID_MARKET);
        Ok(self.markets(market_id).get())
    }

    #[view(getMarketSelections)]
    fn get_market_selections(&self, market_id: u64) -> SCResult<ManagedVec<Selection<Self::Api>>> {
        require!(!self.markets(market_id).is_empty(), ERR_INVALID_MARKET);
        let market = self.markets(market_id).get();
        Ok(market.selections)
    }

    #[view(getSelectionTracker)]
    fn get_selection_tracker(&self, market_id: u64, selection_id: u64) -> SCResult<Tracker<Self::Api>> {
        require!(!self.markets(market_id).is_empty(), ERR_INVALID_MARKET);
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