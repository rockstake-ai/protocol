use crate::types::{
    Bet, BetOrderEntry, BetScheduler, BetStatus, BetType, 
    DetailedBetEntry, Market, MarketStatus, OrderbookEntry, Selection
};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketManagerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::fund_manager::FundManagerModule +
    crate::nft_manager::NftManagerModule +
    crate::bet_scheduler::BetSchedulerModule
{
    // View Functions
    #[view(getMarket)]
    fn get_market(&self, market_id: u64) -> SCResult<Market<Self::Api>> {
        require!(!self.markets(&market_id).is_empty(), "Market does not exist");
        Ok(self.markets(&market_id).get())
    }

    #[view(getMarketStatus)]
    fn get_market_status(&self, market_id: u64) -> SCResult<MarketStatus> {
        let market = self.get_market(market_id)?;
        Ok(market.market_status)
    }

    #[view(getMarketSelections)]
    fn get_market_selections(&self, market_id: u64) -> SCResult<ManagedVec<Self::Api, Selection<Self::Api>>> {
        let market = self.get_market(market_id)?;
        Ok(market.selections)
    }

    #[view(getOrderbook)]
    fn get_orderbook(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> SCResult<MultiValue2<ManagedVec<OrderbookEntry<Self::Api>>, ManagedVec<OrderbookEntry<Self::Api>>>> {
        let market = self.get_market(market_id)?;
        let selection = self.get_selection(&market, selection_id)?;
        
        let back_orders = self.build_orderbook_entries(&selection.priority_queue.back_bets);
        let lay_orders = self.build_orderbook_entries(&selection.priority_queue.lay_bets);
        
        Ok((back_orders, lay_orders).into())
    }

    #[view(getDetailedBetQueue)]
    fn get_detailed_bet_queue(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> SCResult<MultiValue2<ManagedVec<DetailedBetEntry<Self::Api>>, ManagedVec<DetailedBetEntry<Self::Api>>>> {
        let market = self.get_market(market_id)?;
        let selection = self.get_selection(&market, selection_id)?;
        
        let back_queue = self.build_detailed_entries(&selection.priority_queue.back_bets);
        let lay_queue = self.build_detailed_entries(&selection.priority_queue.lay_bets);
        
        Ok((back_queue, lay_queue).into())
    }

    #[view(getBetCountsByStatus)]
    fn get_bet_counts_by_status(&self, market_id: u64) -> SCResult<MultiValue6<BigUint, BigUint, BigUint, BigUint, BigUint, BigUint>> {
        let market = self.get_market(market_id)?;
        
        let mut total_matched = BigUint::zero();
        let mut total_unmatched = BigUint::zero();
        let mut total_partially_matched = BigUint::zero();
        let mut total_win = BigUint::zero();
        let mut total_lost = BigUint::zero();
        let mut total_canceled = BigUint::zero();
    
        self.market_query_event(market_id, market.selections.len());
    
        for selection in market.selections.iter() {
            let (matched, unmatched, partially_matched, win, lost, canceled) = 
                self.get_bet_scheduler_counts(&selection.priority_queue).into_tuple();
    
            self.selection_counts_event(
                market_id,
                selection.selection_id,
                &matched,
                &unmatched,
                &partially_matched,
                &win,
                &lost,
                &canceled
            );
    
            total_matched += matched;
            total_unmatched += unmatched;
            total_partially_matched += partially_matched;
            total_win += win;
            total_lost += lost;
            total_canceled += canceled;
        }
    
        self.total_counts_event(
            market_id,
            &total_matched,
            &total_unmatched,
            &total_partially_matched,
            &total_win,
            &total_lost,
            &total_canceled
        );
    
        Ok((
            total_matched,
            total_unmatched,
            total_partially_matched,
            total_win,
            total_lost,
            total_canceled
        ).into())
    }

    // Public Endpoints
    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(
        &self,
        event_id: u64,
        description: ManagedBuffer,
        selection_descriptions: ManagedVec<ManagedBuffer>,
        close_timestamp: u64
    ) -> SCResult<u64> {
        require!(close_timestamp > self.blockchain().get_block_timestamp(), 
            "Close timestamp must be in the future");

        let market_id = self.get_and_increment_market_counter();
        require!(self.markets(&market_id).is_empty(), "Market already exists");

        let selections = self.create_selections(selection_descriptions)?;
        let market = self.create_market_struct(
            market_id,
            event_id,
            description,
            selections,
            close_timestamp
        );

        self.markets(&market_id).set(&market);
        
        self.market_created_event(
            market_id,
            event_id,
            &description,
            close_timestamp,
            market.created_at
        );

        Ok(market_id)
    }

    #[only_owner]
    #[endpoint(closeMarket)]
    fn close_market(
        &self,
        market_id: u64,
        winning_selection_id: Option<u64>
    ) -> SCResult<()> {
        let mut market = self.get_market(market_id)?;
        require!(market.market_status == MarketStatus::Open, "Market not open");
        
        market.market_status = MarketStatus::Closed;
        self.markets(&market_id).set(&market);

        if let Some(winner_id) = winning_selection_id {
            self.process_market_outcome(market_id, winner_id)?;
        }

        self.market_closed_event(
            market_id.into(),
            winning_selection_id.unwrap_or_default().into()
        );

        Ok(())
    }

    // Internal Functions
    fn get_and_increment_market_counter(&self) -> u64 {
        let mut counter = self.market_counter().get();
        counter += 1;
        self.market_counter().set(&counter);
        counter
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

    fn create_selections(
        &self,
        descriptions: ManagedVec<ManagedBuffer>
    ) -> SCResult<ManagedVec<Selection<Self::Api>>> {
        let mut selections = ManagedVec::new();
        for (index, desc) in descriptions.iter().enumerate() {
            let scheduler = self.init_bet_scheduler();
            selections.push(Selection {
                selection_id: (index + 1) as u64,
                description: desc.as_ref().clone_value(),
                priority_queue: scheduler,
            });
        }
        Ok(selections)
    }

    fn create_market_struct(
        &self,
        market_id: u64,
        event_id: u64,
        description: ManagedBuffer,
        selections: ManagedVec<Selection<Self::Api>>,
        close_timestamp: u64
    ) -> Market<Self::Api> {
        Market {
            market_id,
            event_id,
            description,
            selections,
            liquidity: BigUint::zero(),
            close_timestamp,
            market_status: MarketStatus::Open,
            total_matched_amount: BigUint::zero(),
            created_at: self.blockchain().get_block_timestamp(),
        }
    }

    fn build_orderbook_entries(
        &self,
        bets: &ManagedVec<Self::Api, Bet<Self::Api>>
    ) -> ManagedVec<OrderbookEntry<Self::Api>> {
        let mut entries = ManagedVec::new();
        for bet in bets.iter() {
            entries.push(OrderbookEntry {
                odd: bet.odd.clone(),
                amount: bet.unmatched_amount.clone()
            });
        }
        entries
    }

    fn build_detailed_entries(
        &self,
        bets: &ManagedVec<Self::Api, Bet<Self::Api>>
    ) -> ManagedVec<DetailedBetEntry<Self::Api>> {
        let mut entries = ManagedVec::new();
        for bet in bets.iter() {
            entries.push(DetailedBetEntry {
                bet_type: bet.bet_type.clone(),
                odd: bet.odd.clone(),
                unmatched_amount: bet.unmatched_amount.clone(),
                matched_amount: bet.matched_amount.clone(),
                original_stake: bet.stake_amount.clone(),
                liability: bet.liability.clone(),
                status: bet.status.clone(),
                nft_nonce: bet.nft_nonce,
                created_at: bet.created_at
            });
        }
        entries
    }

    #[view(isMarketOpen)]
    fn is_market_open(&self, market_id: u64) -> bool {
        if self.markets(&market_id).is_empty() {
            return false;
        }
        
        let market = self.markets(&market_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
        
        current_timestamp < market.close_timestamp
    }
}