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

    #[view(getMarketCounter)]
    #[storage_mapper("market_counter")]
    fn market_counter(&self) -> SingleValueMapper<u64>;

    // Levels and liquidity
    #[storage_mapper("selection_back_levels")]
    fn selection_back_levels(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<ManagedVec<Self::Api, PriceLevel<Self::Api>>>;

    #[storage_mapper("selection_lay_levels")]
    fn selection_lay_levels(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<ManagedVec<Self::Api, PriceLevel<Self::Api>>>;

    #[storage_mapper("selection_back_liquidity")]
    fn selection_back_liquidity(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<BigUint<Self::Api>>;

    #[storage_mapper("selection_lay_liquidity")]
    fn selection_lay_liquidity(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<BigUint<Self::Api>>;

    // Counters
    #[storage_mapper("selection_matched_count")]
    fn selection_matched_count(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<u64>;

    #[storage_mapper("selection_unmatched_count")]
    fn selection_unmatched_count(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<u64>;

    #[storage_mapper("selection_partially_matched_count")]
    fn selection_partially_matched_count(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<u64>;

    #[storage_mapper("selection_win_count")]
    fn selection_win_count(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<u64>;

    #[storage_mapper("selection_lost_count")]
    fn selection_lost_count(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<u64>;

    #[storage_mapper("selection_canceled_count")]
    fn selection_canceled_count(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<u64>;

    // Tracker
    #[storage_mapper("selection_tracker")]
    fn selection_tracker(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<Tracker<Self::Api>>;

    // Markets
    #[storage_mapper("markets")]
    fn markets(&self, market_id: u64) -> SingleValueMapper<Market<Self::Api>>;

    #[storage_mapper("total_matched_amount")]
    fn total_matched_amount(&self, market_id: u64, selection_id: u64)
        -> SingleValueMapper<BigUint<Self::Api>>;

    #[storage_mapper("locked_funds")]
    fn locked_funds(&self, address: &ManagedAddress) -> SingleValueMapper<BigUint<Self::Api>>;

}

