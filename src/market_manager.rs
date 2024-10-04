use crate::{storage::{BetType, Market, MarketStatus, Selection, SelectionStatus, BetStatus}};

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
                        if bet.status == BetStatus::Unmatched {
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
            matches!(bet.status, BetStatus::Win | BetStatus::Lost | BetStatus::Canceled)
        )
    }

    #[only_owner]
    #[endpoint(checkMarketStatus)]
    fn check_market_status(&self, market_id: u64) -> SCResult<MarketStatus<Self::Api>> {
        require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
        
        let market = self.markets(&market_id).get();
        
        let mut selections_status = ManagedVec::new();
        for selection in market.selections.iter() {
            let back_bets = self.count_bets_by_type(&market, &selection.selection_id, &BetType::Back);
            let lay_bets = self.count_bets_by_type(&market, &selection.selection_id, &BetType::Lay);
            
            selections_status.push(SelectionStatus {
                selection_id: selection.selection_id,
                description: selection.description.clone(),
                best_back_odds: selection.best_back_odds,
                best_lay_odds: selection.best_lay_odds,
                back_liquidity: selection.back_liquidity.clone(),
                lay_liquidity: selection.lay_liquidity.clone(),
                back_bets,
                lay_bets,
            });
        }
        
        let total_bets = market.bets.len();
        let matched_bets = self.count_bets_by_status(&market, &BetStatus::Matched);
        let unmatched_bets = self.count_bets_by_status(&market, &BetStatus::Unmatched);
        
        Ok(MarketStatus {
            market_id: market.market_id,
            description: market.description,
            total_back_liquidity: market.back_liquidity,
            total_lay_liquidity: market.lay_liquidity,
            total_bets,
            matched_bets,
            unmatched_bets,
            selections: selections_status,
            close_timestamp: market.close_timestamp,
        })
    }

    fn count_bets_by_type(&self, market: &Market<Self::Api>, selection_id: &u64, bet_type: &BetType) -> usize {
        market.bets.iter()
            .filter(|bet| bet.selection.selection_id == *selection_id && bet.bet_type == *bet_type)
            .count()
    }

    fn count_bets_by_status(&self, market: &Market<Self::Api>, status: &BetStatus) -> usize {
        market.bets.iter()
            .filter(|bet| bet.status == *status)
            .count()
    }
}