use crate::types::{BetStatus, BetType, MarketSelectionInfo, MatchedPart, Sport};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("placeBet")]
    fn place_bet_event(
        &self,
        #[indexed] user_id: &ManagedBuffer,
        #[indexed] sport_index: u8,
        #[indexed] market_id: &u64,
        #[indexed] selection_id: &u64,
        #[indexed] stake_amount: &BigUint,
        #[indexed] odd: &BigUint,
        #[indexed] bet_type: u8,
        #[indexed] payment_token: &EgldOrEsdtTokenIdentifier<Self::Api>,
        #[indexed] liability: &BigUint,
        #[indexed] status: u8,
        #[indexed] matched_parts: &ManagedVec<Self::Api, MatchedPart<Self::Api>>,
        #[indexed] bet_id: u64,
        #[indexed] potential_profit: &BigUint,
    );
    #[event("create_market")]
    fn create_market_event(
        &self,
        #[indexed] sport_index: u8, 
        #[indexed] event_id: u64,                       
        markets: &ManagedVec<Self::Api, MarketSelectionInfo<Self::Api>>
    );

    #[event("debugProcessBet")]
    fn debug_process_bet_event(
        &self,
        #[indexed] bet_type: u8,
        #[indexed] odds: &BigUint,
        #[indexed] event_id: u64,
        #[indexed] selection_id: u64,
        #[indexed] opposite_levels_len: u64
    );

    #[event("debugOrderbook")]
    fn debug_orderbook_event(
        &self,
        #[indexed] bet_type: u8,
        #[indexed] odds: &BigUint,
        #[indexed] event_id: u64,
        #[indexed] selection_id: u64,
        #[indexed] remaining: &BigUint
    );

    #[event("debugMatchedParts")]
    fn debug_matched_parts_event(&self, #[indexed] nonce: u64, #[indexed] amount: &BigUint);

}