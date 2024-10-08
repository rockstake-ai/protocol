use crate::priority_queue::PriorityQueue;

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
    pub timestamp: u64, 
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
    pub timestamp: u64, 
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
    pub created_timestamp: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Selection<M: ManagedTypeApi> {
    pub selection_id: u64,
    pub description: ManagedBuffer<M>,
    pub priority_queue: PriorityQueue<M>,
}



// #[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
// pub struct Selection<M: ManagedTypeApi> {
//     pub selection_id: u64,              
//     pub description: ManagedBuffer<M>,         
//     pub back_liquidity: BigUint<M>,            
//     pub lay_liquidity: BigUint<M>,             
//     pub best_back_odds: BigUint<M>,            
//     pub best_lay_odds: BigUint<M>,            
// }

// #[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
// pub struct Market<M: ManagedTypeApi> {
//     pub market_id: u64,
//     pub event_id: u64,
//     pub description: ManagedBuffer<M>,
//     pub selections: ManagedVec<M, Selection<M>>,
//     pub priority_queue: PriorityQueue<M>, // Adăugați acest câmp nou
//     pub back_liquidity: BigUint<M>,
//     pub lay_liquidity: BigUint<M>,
//     pub best_back_odds: BigUint<M>,
//     pub best_lay_odds: BigUint<M>,
//     pub bets: ManagedVec<M, Bet<M>>,
//     pub close_timestamp: u64, 
//     pub market_status: MarketStatus, 
//     pub total_matched_amount: BigUint<M>, 
//     pub created_timestamp: u64, 
// }