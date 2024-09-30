use crate::storage::{Bet, BetType};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("bet_placed")]
    fn bet_placed_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] betslip_token_id: &TokenIdentifier,
        #[indexed] market_id: &BigUint,
        #[indexed] selection_id: &BigUint,
        #[indexed] total_amount: &BigUint,
        #[indexed] odds: &BigUint,
        #[indexed] bet_type: BetType,
        #[indexed] token_identifier: &TokenIdentifier,
        #[indexed] token_nonce: u64,
        #[indexed] matched_amount: &BigUint,
        #[indexed] remaining_amount: &BigUint
    );

    #[event("market_closed")]
    fn market_closed_event(&self, #[indexed] market_id: BigUint, #[indexed] winning_selection_id: BigUint);

    #[event("expired_markets_closed")]
    fn expired_markets_closed_event(&self, #[indexed] market_ids: ManagedVec<BigUint>);

    #[event("claimFromBetslip")]
    fn claim_from_betslip_event(
        &self,
        #[indexed] betslip_id: u64,
        #[indexed] payout: &BigUint,
        #[indexed] recipient: &ManagedAddress,
    );

}