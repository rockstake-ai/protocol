use crate::types::{Bet, Market, PriceLevel, Tracker};

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

    //
    #[storage_mapper("selection_back_levels")]
    fn selection_back_levels(&self, market_id: u64, selection_id: u64) 
        -> SingleValueMapper<ManagedVec<Self::Api, PriceLevel<Self::Api>>>;

    #[storage_mapper("selection_lay_levels")]
    fn selection_lay_levels(&self, market_id: u64, selection_id: u64) 
        -> SingleValueMapper<ManagedVec<Self::Api, PriceLevel<Self::Api>>>;

    #[storage_mapper("selection_tracker")]
    fn selection_tracker(&self, market_id: u64, selection_id: u64) 
        -> SingleValueMapper<Tracker<Self::Api>>;

    //
    #[storage_mapper("back_levels")]
    fn back_levels(&self) -> SingleValueMapper<ManagedVec<Self::Api, PriceLevel<Self::Api>>>;

    #[storage_mapper("lay_levels")]
    fn lay_levels(&self) -> SingleValueMapper<ManagedVec<Self::Api, PriceLevel<Self::Api>>>;

    #[storage_mapper("back_liquidity")]
    fn back_liquidity(&self) -> SingleValueMapper<BigUint<Self::Api>>;

    #[storage_mapper("lay_liquidity")]
    fn lay_liquidity(&self) -> SingleValueMapper<BigUint<Self::Api>>;

    #[storage_mapper("matched_count")]
    fn matched_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("unmatched_count")]
    fn unmatched_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("partially_matched_count")]
    fn partially_matched_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("win_count")]
    fn win_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("lost_count")]
    fn lost_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("canceled_count")]
    fn canceled_count(&self) -> SingleValueMapper<u64>;

}

