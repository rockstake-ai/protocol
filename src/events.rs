use crate::types::{BetStatus, BetType};

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

    #[event("market_closed")]
    fn market_closed_event(&self, #[indexed] market_id: u64, #[indexed] timestamp: u64);

  
    #[event("bet_won")]
    fn bet_won_event(
        &self,
        #[indexed] bettor: &ManagedAddress,
        #[indexed] nft_nonce: &u64,
        #[indexed] event_id: &u64,
        #[indexed] selection_id: &u64,
        #[indexed] win_amount: &BigUint,
        #[indexed] token_identifier: &EgldOrEsdtTokenIdentifier,
        #[indexed] token_nonce: u64,
    );

    #[event("reward_distributed")]
    fn reward_distributed_event(
        &self,
        #[indexed] bet_id: u64,
        #[indexed] bettor: &ManagedAddress,
        amount: &BigUint,
    );

    #[event("market_query")]
    fn market_query_event(
        &self,
        #[indexed] market_id: u64,
        #[indexed] selection_count: usize
    );

    #[event("selection_counts")]
    fn selection_counts_event(
        &self,
        #[indexed] market_id: u64,
        #[indexed] selection_id: u64,
        #[indexed] matched: &BigUint,
        #[indexed] unmatched: &BigUint,
        #[indexed] partially_matched: &BigUint,
        #[indexed] win: &BigUint,
        #[indexed] lost: &BigUint,
        #[indexed] canceled: &BigUint
    );

    #[event("total_counts")]
    fn total_counts_event(
        &self,
        #[indexed] market_id: u64,
        #[indexed] matched: &BigUint,
        #[indexed] unmatched: &BigUint,
        #[indexed] partially_matched: &BigUint,
        #[indexed] win: &BigUint,
        #[indexed] lost: &BigUint,
        #[indexed] canceled: &BigUint
    );

    #[event("bet_status_updated")]
    fn bet_status_updated_event(
        &self,
        #[indexed] market_id: u64,
        #[indexed] selection_id: u64,
        #[indexed] bet_id: u64,
        #[indexed] old_status: &BetStatus,
        #[indexed] new_status: &BetStatus,
    );

    #[event("bet_counter_update")]
    fn bet_counter_update_event(
        &self,
        #[indexed] old_status: &BetStatus,
        #[indexed] new_status: &BetStatus,
        #[indexed] matched_count: u64,
        #[indexed] unmatched_count: u64,
        #[indexed] partially_matched_count: u64,
        #[indexed] win_count: u64,
        #[indexed] lost_count: u64,
        #[indexed] canceled_count: u64,
    );

    #[event("counter_debug")]
    fn counter_debug_event(
        &self,
        #[indexed] matched: &usize,
        #[indexed] unmatched: &usize,
        #[indexed] partially_matched: &usize,
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

}