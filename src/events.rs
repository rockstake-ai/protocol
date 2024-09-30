use crate::storage::{Bet, BetType};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("placeBet")]
    fn place_bet_event(
        &self,
        #[indexed] creator: &ManagedAddress,
        #[indexed] betslip_token_identifier: &TokenIdentifier,
        #[indexed] bets: &ManagedVec<Bet<Self::Api>>,
        #[indexed] total_odd: &BigUint,
        #[indexed] stake: &BigUint,
        #[indexed] payout: &BigUint,
        #[indexed] payment_token: &EgldOrEsdtTokenIdentifier,
        #[indexed] payment_nonce: u64,
        // #[indexed] status: &Status,
    );

    #[event("bet_placed")]
    fn bet_placed_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] bet_token_identifier: &TokenIdentifier,
        #[indexed] market_id: &BigUint,
        #[indexed] selection_id: &BigUint,
        #[indexed] amount: &BigUint,
        #[indexed] odds: &BigUint,
        #[indexed] bet_type: BetType,
        #[indexed] payment_token: &EgldOrEsdtTokenIdentifier,
        #[indexed] payment_nonce: u64,
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