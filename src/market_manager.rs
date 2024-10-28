use crate::types::{Bet, BetOrderEntry, BetScheduler, BetStatus, BetType, DetailedBetEntry, Market, MarketStatus, OrderbookEntry, Selection};
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
        
        let created_at = self.blockchain().get_block_timestamp();
        require!(close_timestamp > created_at, "Close timestamp must be in the future");
        
        let mut selections = ManagedVec::new();
        for (index, desc) in selection_descriptions.iter().enumerate() {
            let selection = Selection {
                selection_id: (index + 1) as u64,
                description: desc.as_ref().clone_value(),
                priority_queue: BetScheduler::new(),
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
            created_at,
        };
        
        self.markets(&market_id).set(&market);
        market_id
    }

    #[only_owner]
#[endpoint(getBetCountsByStatus)]
fn get_bet_counts_by_status(&self, market_id: u64) -> SCResult<(BigUint, BigUint, BigUint, BigUint, BigUint, BigUint)> {
    require!(!self.markets(&market_id).is_empty(), "Market does not exist");
    let market = self.markets(&market_id).get();
    
    let mut total_matched = BigUint::zero();
    let mut total_unmatched = BigUint::zero();
    let mut total_partially_matched = BigUint::zero();
    let mut total_win = BigUint::zero();
    let mut total_lost = BigUint::zero();
    let mut total_canceled = BigUint::zero();

    for selection in market.selections.iter() {
        let (matched, unmatched, partially, win, lost, canceled) = selection.priority_queue.get_status_counts();
        total_matched += matched;
        total_unmatched += unmatched;
        total_partially_matched += partially;
        total_win += win;
        total_lost += lost;
        total_canceled += canceled;
    }

    Ok((
        total_matched,
        total_unmatched,
        total_partially_matched,
        total_win,
        total_lost,
        total_canceled
    ))
}


    fn get_and_increment_market_counter(&self) -> u64 {
        let mut counter = self.market_counter().get();
        counter += 1;
        self.market_counter().set(&counter);
        counter
    }

    // #[view(getOrderbook)]
    #[endpoint(getOrderbook)]
    fn get_orderbook(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> SCResult<MultiValue2<ManagedVec<OrderbookEntry<Self::Api>>, ManagedVec<OrderbookEntry<Self::Api>>>> {
        require!(!self.markets(&market_id).is_empty(), "Market does not exist");
        let market = self.markets(&market_id).get();
        
        let selection_index = market.selections.iter()
            .position(|s| s.selection_id == selection_id)
            .ok_or("Selection not found")?;
        
        let selection = market.selections.get(selection_index);
        
        let mut back_orders = ManagedVec::new();
        for bet in selection.priority_queue.get_back_bets().iter() {
            back_orders.push(OrderbookEntry {
                odd: bet.odd.clone(),
                amount: bet.unmatched_amount.clone()
            });
        }
        
        let mut lay_orders = ManagedVec::new();
        for bet in selection.priority_queue.get_lay_bets().iter() {
            lay_orders.push(OrderbookEntry {
                odd: bet.odd.clone(),
                amount: bet.unmatched_amount.clone()
            });
        }
        
        Ok((back_orders, lay_orders).into())
    }

    #[endpoint(getBetQueueStatus)]
    fn get_detailed_bet_queue(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> SCResult<MultiValue2<ManagedVec<DetailedBetEntry<Self::Api>>, ManagedVec<DetailedBetEntry<Self::Api>>>> {
        require!(!self.markets(&market_id).is_empty(), "Market does not exist");
        let market = self.markets(&market_id).get();
        
        let selection_index = market.selections.iter()
            .position(|s| s.selection_id == selection_id)
            .ok_or("Selection not found")?;
        
        let selection = market.selections.get(selection_index);
        
        let mut back_queue = ManagedVec::new();
        for bet in selection.priority_queue.get_back_bets().iter() {
            back_queue.push(DetailedBetEntry {
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
        
        let mut lay_queue = ManagedVec::new();
        for bet in selection.priority_queue.get_lay_bets().iter() {
            lay_queue.push(DetailedBetEntry {
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
        
        Ok((back_queue, lay_queue).into())
    }
}