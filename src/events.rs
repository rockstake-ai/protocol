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

    #[event("claimFromBetslip")]
    fn claim_from_betslip_event(
        &self,
        #[indexed] betslip_id: u64,
        #[indexed] payout: &BigUint,
        #[indexed] recipient: &ManagedAddress,
    );

    #[event("event_create_p2p_bet")]
    fn event_create_p2p_bet(
        &self,
        #[indexed] bet_id: &ManagedBuffer,
        #[indexed] creator: &ManagedAddress,
        event_details: &ManagedBuffer,
    );

    #[event("event_join_p2p_bet")]
    fn event_join_p2p_bet(
        &self,
        #[indexed] bet_id: &ManagedBuffer,
        #[indexed] participant: &ManagedAddress,
        #[indexed]option_chosen: &ManagedBuffer,
        stake: &BigUint,
    );

    #[event("event_finalize_p2p_bet")]
    fn event_finalize_p2p_bet(
        &self,
        #[indexed] bet_id: &ManagedBuffer,
        winning_option: &ManagedBuffer,
    );
}