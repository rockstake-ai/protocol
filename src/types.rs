multiversx_sc::imports!();
multiversx_sc::derive_imports!();
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone, ManagedVecItem)]
pub enum BetStatus {
    Matched,
    Unmatched,
    PartiallyMatched,
    Win,
    Lost,
    Canceled,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone, ManagedVecItem)]
pub enum BetType {
    Back,
    Lay
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone, PartialEq)]
pub enum MarketStatus {
    Open,    
    Closed, 
    Settled
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Bet<M: ManagedTypeApi> {
    pub bettor: ManagedAddress<M>,
    pub event: u64, 
    pub selection: Selection<M>, 
    pub stake_amount: BigUint<M>, 
    pub liability: BigUint<M>, 
    pub matched_amount: BigUint<M>, 
    pub unmatched_amount: BigUint<M>, 
    pub potential_profit: BigUint<M>, 
    pub odd: BigUint<M>, 
    pub bet_type: BetType, 
    pub status: BetStatus, 
    pub payment_token: EgldOrEsdtTokenIdentifier<M>,
    pub payment_nonce: u64,
    pub nft_nonce: u64,
    pub created_at: u64, 
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct BetAttributes<M:ManagedTypeApi>{
    pub bettor: ManagedAddress<M>,
    pub event: u64,     
    pub selection: Selection<M>,     
    pub stake_amount: BigUint<M>, 
    pub liability: BigUint<M>,  
    pub matched_amount: BigUint<M>, 
    pub unmatched_amount: BigUint<M>,    
    pub potential_profit: BigUint<M>,     
    pub odd: BigUint<M>,        
    pub bet_type: BetType,      
    pub status: BetStatus,         
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, 
    pub payment_nonce: u64,
    pub created_at: u64, 
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct Market<M: ManagedTypeApi> {
    pub market_id: u64,
    pub event_id: u64,
    pub description: ManagedBuffer<M>,
    pub selections: ManagedVec<M, Selection<M>>,
    pub close_timestamp: u64,
    pub market_status: MarketStatus,
    pub total_matched_amount: BigUint<M>,
    pub liquidity: BigUint<M>,
    pub created_at: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Selection<M: ManagedTypeApi> {
    pub selection_id: u64,
    pub description: ManagedBuffer<M>,
    pub priority_queue: Tracker<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Tracker<M: ManagedTypeApi> {
    pub back_bets: ManagedVec<M, Bet<M>>,
    pub lay_bets: ManagedVec<M, Bet<M>>,
    pub best_back_odds: BigUint<M>,
    pub best_lay_odds: BigUint<M>,
    pub back_liquidity: BigUint<M>,
    pub lay_liquidity: BigUint<M>,
    pub matched_count: usize,
    pub unmatched_count: usize,
    pub partially_matched_count: usize,
    pub win_count: usize,
    pub lost_count: usize,
    pub canceled_count: usize,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct SchedulerDebugView<M: ManagedTypeApi> {
    pub back_bets_count: usize,
    pub lay_bets_count: usize,
    pub best_back_odds: BigUint<M>,
    pub best_lay_odds: BigUint<M>,
    pub back_liquidity: BigUint<M>,
    pub lay_liquidity: BigUint<M>,
    pub matched_count: u32,
    pub unmatched_count: u32,
    pub partially_matched_count: u32,
    pub win_count: u32,
    pub lost_count: u32,
    pub canceled_count: u32
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct BetQueueView<M: ManagedTypeApi> {
    pub odd: BigUint<M>,
    pub amount: BigUint<M>,
    pub status: BetStatus
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct DetailedSchedulerView<M: ManagedTypeApi> {
    // Counters
    pub back_bets_count: usize,
    pub lay_bets_count: usize,
    
    // Liquidities
    pub total_back_liquidity: BigUint<M>,
    pub total_lay_liquidity: BigUint<M>,
    
    // Best odds
    pub best_back_odds: BigUint<M>,
    pub best_lay_odds: BigUint<M>,
    
    // Status counters
    pub matched_count: u32,
    pub unmatched_count: u32,
    pub partially_matched_count: u32,
    
    // Queue contents
    pub back_queue: ManagedVec<M, BetQueueView<M>>,
    pub lay_queue: ManagedVec<M, BetQueueView<M>>
}


