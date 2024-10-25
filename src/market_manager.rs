use crate::types::{Bet, BetStatus, BetType, Market, MarketStatus, Selection};
use crate::bet_scheduler::BetScheduler;
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
}