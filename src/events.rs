use crate::types::{BetType};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("place_bet")]
    fn place_bet_event(
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
        #[indexed] unmatched_amount: &BigUint,
        #[indexed] collateral: &BigUint
    );

    #[event("market_closed")]
    fn market_closed_event(&self, #[indexed] market_id: u64, #[indexed] timestamp: u64);

    #[event("reward_distributed")]
    fn reward_distributed_event(
        &self,
        #[indexed] bet_id: u64,
        #[indexed] bettor: &ManagedAddress,
        amount: &BigUint,
    );

    #[event("bet_counter_debug")]
    fn bet_counter_debug_event(
        &self,
        #[indexed] matched: &usize,
        #[indexed] unmatched: &usize,
        #[indexed] partially: &usize,
        #[indexed] win: &usize,
        #[indexed] lost: &usize,
        #[indexed] canceled: &usize,
    );

    // Event for tracking selection creation
    #[event("selection_created")]
    fn selection_created_event(
        &self,
        #[indexed] market_id: &u64,
        #[indexed] selection_id: &u64,
        #[indexed] description: &ManagedBuffer,
    );

    #[event("match")]
    fn emit_match_event(
        &self,
        #[indexed] bettor: &ManagedAddress,
        #[indexed] event_id: &u64,
        #[indexed] matched_amount: &BigUint,
        #[indexed] odds: &BigUint,
    );

    #[event("marketCreated")]
    fn market_created_event(
        &self,
        #[indexed] market_id: u64,
        #[indexed] event_id: u64,
        #[indexed] current_counter: &u64,
    );

    #[event("marketSettled")]
    fn market_settled_event(
        &self,
        #[indexed] market_id: u64,
        #[indexed] winning_selection: u64,
        #[indexed] current_counter: u64,
    );

    #[event("bet_refunded")]
    fn bet_refunded_event(
        &self,
        #[indexed] bet_id: u64,
        #[indexed] bettor: &ManagedAddress,
        #[indexed] amount: &BigUint,
    );

}