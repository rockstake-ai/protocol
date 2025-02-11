use crate::types::{BetAmounts, BetCounts, BetDetailResponse, BetMatchedInfo, BetStatus, BetType, Market, MarketStatsResponse, MarketStatus, MarketVolumes, Selection, Tracker};
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

    #[view(getMarketBetsInfo)]
fn get_market_bets_info(&self, market_id: u64) -> MarketStatsResponse<Self::Api> {
    let mut bet_counts = BetCounts {
        total: 0,
        matched: 0,
        unmatched: 0,
        partially_matched: 0
    };
    
    let mut volumes = MarketVolumes {
        back_matched: BigUint::zero(),
        lay_matched: BigUint::zero(),
        back_unmatched: BigUint::zero(),
        lay_unmatched: BigUint::zero()
    };
    
    let mut bets = ManagedVec::new();

    for bet_id in self.market_bet_ids(market_id).iter() {
        let bet = self.bet_by_id(bet_id).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
        
        // Update counts
        match bet.status {
            BetStatus::Matched => bet_counts.matched += 1,
            BetStatus::Unmatched => bet_counts.unmatched += 1,
            BetStatus::PartiallyMatched => bet_counts.partially_matched += 1,
            _ => {}
        }
        
        // Update volumes
        match bet.bet_type {
            BetType::Back => {
                volumes.back_matched += &bet.total_matched;
                volumes.back_unmatched += &unmatched;
            },
            BetType::Lay => {
                volumes.lay_matched += &bet.total_matched;
                volumes.lay_unmatched += &unmatched;
            }
        }
        
        let bet_detail = BetDetailResponse {
            nft_nonce: bet.nft_nonce,
            selection_id: bet.selection.id,
            bettor: bet.bettor,
            stake: BetAmounts {
                stake_amount: bet.stake_amount,
                matched: bet.total_matched,
                unmatched,
                liability: bet.liability
            },
            odds: bet.odd,
            status: bet.status,
            matched_info: BetMatchedInfo {
                matched_parts: bet.matched_parts,
                potential_profit: bet.potential_profit
            }
        };
        
        bets.push(bet_detail);
    }
    
    bet_counts.total = bets.len() as u32;
    
    MarketStatsResponse {
        bet_counts,
        volumes,
        bets
    }
}

#[view(getBetInfo)]
fn get_bet_info(&self, bet_id: u64) -> MarketStatsResponse<Self::Api> {
    let bet = self.bet_by_id(bet_id).get();
    let unmatched = &bet.stake_amount - &bet.total_matched;
    
    // Initialize counters
    let mut bet_counts = BetCounts {
        total: 1,
        matched: 0,
        unmatched: 0,
        partially_matched: 0
    };
    
    // Initialize volumes
    let mut volumes = MarketVolumes {
        back_matched: BigUint::zero(),
        lay_matched: BigUint::zero(),
        back_unmatched: BigUint::zero(),
        lay_unmatched: BigUint::zero()
    };
    
    // Update count based on status
    match bet.status {
        BetStatus::Matched => bet_counts.matched = 1,
        BetStatus::Unmatched => bet_counts.unmatched = 1,
        BetStatus::PartiallyMatched => bet_counts.partially_matched = 1,
        _ => {}
    }
    
    // Update volumes based on bet type
    match bet.bet_type {
        BetType::Back => {
            volumes.back_matched = bet.total_matched.clone();
            volumes.back_unmatched = unmatched.clone();
        },
        BetType::Lay => {
            volumes.lay_matched = bet.total_matched.clone();
            volumes.lay_unmatched = unmatched.clone();
        }
    }
    
    // Create bet detail
    let bet_detail = BetDetailResponse {
        nft_nonce: bet.nft_nonce,
        selection_id: bet.selection.id,
        bettor: bet.bettor,
        stake: BetAmounts {
            stake_amount: bet.stake_amount,
            matched: bet.total_matched,
            unmatched,
            liability: bet.liability
        },
        odds: bet.odd,
        status: bet.status,
        matched_info: BetMatchedInfo {
            matched_parts: bet.matched_parts,
            potential_profit: bet.potential_profit
        }
    };
    
    // Create response with single bet
    let mut bets = ManagedVec::new();
    bets.push(bet_detail);
    
    MarketStatsResponse {
        bet_counts,
        volumes,
        bets
    }
}
}