use crate::{types::Tracker, types::{Bet, Market}};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait StorageModule {
    #[storage_mapper("betById")]
    fn bet_by_id(&self, bet_id: u64) -> SingleValueMapper<Bet<Self::Api>>;

    #[storage_mapper("betNftToken")]
    fn bet_nft_token(&self) -> NonFungibleTokenMapper<Self::Api>;

    #[storage_mapper("betNftBaseUri")]
    fn bet_nft_base_uri(&self) -> SingleValueMapper<ManagedBuffer>;

    #[storage_mapper("market_counter")]
    fn market_counter(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("markets")]
    fn markets(&self, market_id: &u64) -> SingleValueMapper<Market<Self::Api>>;

    #[storage_mapper("lockedFunds")]
    fn locked_funds(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint<Self::Api>>;

    #[view(getUnmatchedBets)]
    #[storage_mapper("unmatched_bets")]
    fn unmatched_bets(&self, market_id: u64) -> SingleValueMapper<Tracker<Self::Api>>;

    #[storage_mapper("market_queues")]
    fn market_queues(&self, market_id: u64) -> SingleValueMapper<Tracker<Self::Api>>;

    #[view(getBetScheduler)]
    #[storage_mapper("bet_scheduler")]
    fn bet_scheduler(&self) -> SingleValueMapper<Tracker<Self::Api>>;

    #[storage_mapper("selection_scheduler")]
    fn selection_scheduler(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> SingleValueMapper<Tracker<Self::Api>>;

}

