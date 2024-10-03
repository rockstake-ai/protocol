use crate::storage::BetType;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("bet_placed")]
    fn bet_placed_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] betslip_token_id: &TokenIdentifier,
        #[indexed] market_id: &u64,
        #[indexed] selection_id: &u64,
        #[indexed] total_amount: &BigUint,
        #[indexed] odds: &BigUint,
        #[indexed] bet_type: BetType,
        #[indexed] token_identifier: &EgldOrEsdtTokenIdentifier,
        #[indexed] token_nonce: u64,
        #[indexed] matched_amount: &BigUint,
        #[indexed] remaining_amount: &BigUint
    );

    #[event("bet_closed")]
fn bet_closed_event(
    &self,
    #[indexed] bettor: &ManagedAddress,
    #[indexed] bet_id: &u64,
    #[indexed] market_id: &u64,
    #[indexed] selection_id: &u64,
    #[indexed] refund_amount: &BigUint,
    #[indexed] token_identifier: &EgldOrEsdtTokenIdentifier,
    #[indexed] token_nonce: u64,
);

    #[event("debug")]
    fn debug_event(&self, #[indexed] msg: ManagedBuffer, #[indexed] value: BigUint);

    #[event("market_closed")]
    fn market_closed_event(&self, #[indexed] market_id: BigUint, #[indexed] winning_selection_id: BigUint);

    #[event("expired_markets_closed")]
    fn expired_markets_closed_event(&self, #[indexed] market_ids: ManagedVec<u64>);
    
    #[event("refund_unmatched_bet")]
    fn refund_unmatched_bet_event(
        &self,
        #[indexed] bet_id: u64,
        #[indexed] amount: &BigUint,
        #[indexed] bettor: &ManagedAddress,
    );

    #[event("claimFromBetslip")]
    fn claim_from_betslip_event(
        &self,
        #[indexed] betslip_id: u64,
        #[indexed] payout: &BigUint,
        #[indexed] recipient: &ManagedAddress,
    );

}