use crate::types::{MarketSelectionInfo, MatchedPart};

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
        #[indexed] nft_nonce: u64,
    );

    #[event("cancelBet")]
    fn cancel_bet_event(
        &self,
        #[indexed] bettor: &ManagedAddress,
        #[indexed] bet_id: u64,
        #[indexed] status: u8,           
        #[indexed] refund_amount: &BigUint,
        #[indexed] total_matched: &BigUint,
        #[indexed] total_amount: &BigUint,
        #[indexed] potential_profit: &BigUint,
        #[indexed] liability: &BigUint, 
    );

    #[event("claimWin")]
    fn claim_win_event(
        &self,
        #[indexed] bettor: &ManagedAddress,
        #[indexed] bet_id: u64,
        #[indexed] status: u8,
        #[indexed] payout: &BigUint,
    );

    #[event("create_market")]
    fn create_market_event(
        &self,
        #[indexed] sport_index: u8, 
        #[indexed] event_id: u64,                       
        markets: &ManagedVec<Self::Api, MarketSelectionInfo<Self::Api>>
    );

}