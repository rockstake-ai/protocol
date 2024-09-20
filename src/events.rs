use crate::storage::{BetGroup};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("placeBet")]
    fn place_bet_event(
        &self,
        #[indexed] creator: &ManagedAddress,
        #[indexed] betslip_token_identifier: &TokenIdentifier,
        #[indexed] bets: &ManagedVec<BetGroup<Self::Api>>,
        #[indexed] total_odd: &BigUint,
        #[indexed] stake: &BigUint,
        #[indexed] payout: &BigUint,
        #[indexed] payment_token: &EgldOrEsdtTokenIdentifier,
        #[indexed] payment_nonce: u64,
        // #[indexed] status: &Status,
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