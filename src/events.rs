use crate::types::{BetStatus, BetType, MarketSelectionInfo, MatchedPart, Sport};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("place_bet")]
    fn place_bet_event(
        &self,
        #[indexed] user: &ManagedAddress,
        #[indexed] sport_index: u8, // Folosim u8 pentru index numeric
        #[indexed] market_id: &u64,  // Corectăm la u64, nu u32
        #[indexed] selection_id: &u64,
        #[indexed] total_amount: &BigUint,
        #[indexed] odds: &BigUint,
        #[indexed] bet_type: BetType,
        #[indexed] token_identifier: &EgldOrEsdtTokenIdentifier,
        #[indexed] liability: &BigUint,
        #[indexed] status: BetStatus,
        #[indexed] matched_parts: &ManagedVec<MatchedPart<Self::Api>>
    );
    
    #[event("create_market")]
    fn create_market_event(
        &self,
        #[indexed] sport_index: u8, 
        #[indexed] event_id: u64,                       
        markets: &ManagedVec<Self::Api, MarketSelectionInfo<Self::Api>>
    );

}