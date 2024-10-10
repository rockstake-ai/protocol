use crate::types::{Bet, BetStatus, BetType, Market, MarketStatus, Selection};
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
        
        let created_at = self.blockchain().get_block_timestamp();
        require!(close_timestamp > created_at, "Close timestamp must be in the future");
        
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
            created_at,
        };
        
        self.markets(&market_id).set(&market);
        market_id
    }

    #[only_owner]
    #[endpoint(getBetCountsByStatus)]
    fn get_bet_counts_by_status(&self, market_id: u64) ->  SCResult<(BigUint, BigUint, BigUint)> {
        require!(!self.markets(&market_id).is_empty(), "Market does not exist");
        
        let market = self.markets(&market_id).get();
        let mut matched_count = 0;
        let mut unmatched_count = 0;
        let mut partially_matched_count = 0;

        for selection in market.selections.iter() {
            for bet_type in [BetType::Back, BetType::Lay].iter() {
                let bets = match bet_type {
                    BetType::Back => selection.priority_queue.get_back_bets(),
                    BetType::Lay => selection.priority_queue.get_lay_bets(),
                };

                for bet in bets.iter() {
                    match bet.status {
                        BetStatus::Matched => matched_count += 1u32,
                        BetStatus::Unmatched => unmatched_count += 1u32,
                        BetStatus::PartiallyMatched => partially_matched_count += 1u32,
                        _ => (), // Ignorăm alte statusuri
                    }
                }
            }
        }

        Ok((matched_count.into(), unmatched_count.into(), partially_matched_count.into()))
    }


    fn get_and_increment_market_counter(&self) -> u64 {
        let mut counter = self.market_counter().get();
        counter += 1;
        self.market_counter().set(&counter);
        counter
    }
}