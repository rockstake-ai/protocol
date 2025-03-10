use crate::{errors::{ERR_MARKET_ALREADY_EXISTS, ERR_MARKET_NOT_OPEN, ERR_NO_MARKETS_FOUND}, types::{Market, MarketSelectionInfo, MarketStatus, MarketType, Selection, SelectionInfo, SelectionType, Sport, Tracker}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait MarketModule:
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::fund::FundModule + 
    crate::nft::NftModule +
    crate::orderbook::OrderbookModule +
    crate::validation::ValidationModule
{
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Markets Creation -------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Creates markets for a specific event and sport, restricted to the contract owner.
    /// Parameters:
    /// - sport: The type of sport (e.g., Football, Basketball).
    /// - event_id: The unique ID of the event.
    /// - close_timestamp: The timestamp when betting on the market closes.
    #[only_owner]
    #[endpoint(createMarkets)]
    fn create_markets(
        &self,
        sport: Sport,
        event_id: u64,
        close_timestamp: u64
    ){
        self.validate_market_creation(close_timestamp);
        
        let existing_markets = self.markets_by_event_and_sport(sport, event_id).get();
        require!(
            existing_markets.is_empty(),
            ERR_MARKET_ALREADY_EXISTS
        );
        
        let mut market_ids = ManagedVec::new();
        let mut markets_info = ManagedVec::new();
        
        // Create markets based on sport type
        if sport == Sport::Football {
            // Football specific markets
            
            // 1X2 market (Home, Draw, Away)
            let ft_result_selections = [
                SelectionType::One,
                SelectionType::Draw,
                SelectionType::Two
            ];
            
            let (market_id_1x2, selections_1x2) = self.create_single_market(
                sport,
                event_id,
                &ft_result_selections,
                close_timestamp,
                MarketType::FullTimeResult
            );
            market_ids.push(market_id_1x2);
            markets_info.push(MarketSelectionInfo {
                market_id: market_id_1x2,
                market_type: MarketType::FullTimeResult,
                selections: selections_1x2
            });
            
            // Over/Under market
            let selection_types = [
                SelectionType::Over,
                SelectionType::Under
            ];
            let (market_id_ou, selections_ou) = self.create_single_market(
                sport,
                event_id,
                &selection_types,
                close_timestamp,
                MarketType::TotalGoals
            );
            market_ids.push(market_id_ou);
            markets_info.push(MarketSelectionInfo {
                market_id: market_id_ou,
                market_type: MarketType::TotalGoals,
                selections: selections_ou
            });
            
            // Both teams to score market
            let selection_types = [
                SelectionType::Yes,
                SelectionType::No
            ];
            let (market_id_ggng, selections_ggng) = self.create_single_market(
                sport,
                event_id,
                &selection_types,
                close_timestamp,
                MarketType::BothTeamsToScore
            );
            market_ids.push(market_id_ggng);
            markets_info.push(MarketSelectionInfo {
                market_id: market_id_ggng,
                market_type: MarketType::BothTeamsToScore,
                selections: selections_ggng
            });
        } else {
            // For all other sports, create only Winner market
            let winner_selections = [
                SelectionType::One,
                SelectionType::Two
            ];
            
            let (market_id_winner, selections_winner) = self.create_single_market(
                sport,
                event_id,
                &winner_selections,
                close_timestamp,
                MarketType::Winner
            );
            market_ids.push(market_id_winner);
            markets_info.push(MarketSelectionInfo {
                market_id: market_id_winner,
                market_type: MarketType::Winner,
                selections: selections_winner
            });
        }
        
        self.markets_by_event_and_sport(sport, event_id).set(&market_ids);

        let sport_index = match sport {
            Sport::Football => 1u8,
            Sport::Basketball => 2u8,
            Sport::CounterStrike => 3u8,
            Sport::Dota => 4u8,
            Sport::LeagueOfLegends => 5u8,
        };
        
        self.create_market_event(sport_index, event_id, &markets_info);
    }


    /// Creates a single market with specified selections for an event.
    /// Parameters:
    /// - sport: The type of sport.
    /// - event_id: The unique ID of the event.
    /// - selection_types: Array of selection types for the market.
    /// - close_timestamp: The timestamp when the market closes.
    /// - market_type: The type of market (e.g., FullTimeResult, TotalGoals).
    /// Returns: A tuple containing the market ID and a vector of selection info.
    fn create_single_market(
        &self,
        sport: Sport,
        event_id: u64,
        selection_types: &[SelectionType],
        close_timestamp: u64,
        market_type: MarketType,
    ) -> (u64, ManagedVec<Self::Api, SelectionInfo>) {
        let market_type_index = match market_type {
            MarketType::FullTimeResult => 1,
            MarketType::TotalGoals => 2,
            MarketType::BothTeamsToScore => 3,
            MarketType::Winner => 4,
        };
        
        let sport_index = match sport {
            Sport::Football => 1,
            Sport::Basketball => 2,
            Sport::CounterStrike => 3,
            Sport::Dota => 4,
            Sport::LeagueOfLegends => 5,
        };
        let market_id = (sport_index * 1_000_000) + (event_id * 1000) + market_type_index;

        let selections = self.create_selections(market_id, selection_types);
        
        let market = Market {
            market_id,
            event_id,
            market_type,
            description: ManagedBuffer::new_from_bytes(market_type.to_description()),
            selections: selections.clone(),
            liquidity: BigUint::zero(),
            close_timestamp,
            market_status: MarketStatus::Open,
            total_matched_amount: BigUint::zero(),
            created_at: self.blockchain().get_block_timestamp(),
        };

        self.markets(market_id).set(&market);

        let mut selection_infos = ManagedVec::new();
        for (index, selection_type) in selection_types.iter().enumerate() {
            selection_infos.push(SelectionInfo {
                selection_id: (index + 1) as u64,
                selection_type: *selection_type,
            });
        }

        (market_id, selection_infos)
    }

    /// Creates selections for a market based on provided selection types.
    /// Parameters:
    /// - market_id: The ID of the market.
    /// - selection_types: Array of selection types to create.
    /// Returns: A vector of Selection objects.
    fn create_selections(
        &self,
        market_id: u64,
        selection_types: &[SelectionType],
    ) -> ManagedVec<Selection<Self::Api>> {
        let mut selections = ManagedVec::new();
        for (index, selection_type) in selection_types.iter().enumerate() {
            let id = market_id * 10 + (index + 1) as u64;
            self.init_selection_storage(market_id, id);
            let tracker = self.selection_tracker(market_id, id).get();
            selections.push(Selection { 
                id,
                selection_type: *selection_type,
                priority_queue: tracker,
            });
        }
        selections
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Markets Closing -----------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Processes all markets for a given event, closing them if they are open.
    /// Parameters:
    /// - sport: The type of sport.
    /// - event_id: The unique ID of the event.
    #[only_owner]
    #[endpoint(closeMarkets)]
    fn close_markets(&self, sport: Sport, event_id: u64) {
        let market_ids = self.markets_by_event_and_sport(sport, event_id).get();
        require!(
            !market_ids.is_empty(),
            ERR_NO_MARKETS_FOUND
        );
    
        for market_id in market_ids.iter() {
            let mut market = self.markets(market_id).get();
            require!(
                market.market_status == MarketStatus::Open,
               ERR_MARKET_NOT_OPEN
            );
            self.handle_expired_market(sport, event_id, market_id);
            market.market_status = MarketStatus::Closed;
            self.markets(market_id).set(&market);
        }
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Helper Functions ------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Initializes storage for a selection in a market.
    /// Parameters:
    /// - market_id: The ID of the market.
    /// - selection_id: The ID of the selection.
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

    /// Retrieves a specific selection from a market.
    /// Parameters:
    /// - market: The market object containing selections.
    /// - selection_id: The ID of the selection to retrieve.
    /// Returns: The Selection object if found, otherwise panics.
    fn get_selection(
        &self,
        market: &Market<Self::Api>,
        selection_id: u64
    ) -> Selection<Self::Api> {
        market.selections.iter()
            .find(|s| s.id == selection_id)
            .unwrap_or_else(|| sc_panic!(crate::errors::ERR_INVALID_SELECTION))
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- View Functions --------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    #[view(getMarketStatus)]
    fn get_market_status(&self, market_id: u64) -> MarketStatus {
        self.markets(market_id).get().market_status
    }

    #[view(areEventMarketsClosed)]
    fn are_event_markets_closed(&self, event_id: u64) -> bool {
        let market_ids = self.markets_by_event(event_id).get();
        require!(
            !market_ids.is_empty(),
            ERR_NO_MARKETS_FOUND
        );

        for market_id in market_ids.iter() {
            let market = self.markets(market_id).get();
            if market.market_status != MarketStatus::Closed {
                return false;
            }
        }

        true
    }
}