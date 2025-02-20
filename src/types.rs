multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, ManagedVecItem, Copy)]
pub enum BetStatus {
    Matched,
    Unmatched,
    PartiallyMatched,
    Win,
    Lost,
    Canceled,
    Claimed,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, ManagedVecItem, Copy)]
pub enum BetType {
    Back,
    Lay
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, ManagedVecItem, Copy)]
pub enum Sport {
    Football,
    Tennis,
    Basketball,
    LeagueOfLegends,
    CounterStrike2,
    Dota2
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq)]
pub enum MarketStatus {
    Open,    
    Closed, 
    Settled
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct MatchedPart<M: ManagedTypeApi> {
    pub amount: BigUint<M>,
    pub odds: BigUint<M>
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Bet<M: ManagedTypeApi> {
    pub bettor: ManagedAddress<M>,
    pub sport: Sport,
    pub event: u64,
    pub selection: Selection<M>,
    pub stake_amount: BigUint<M>,
    pub liability: BigUint<M>,
    pub total_amount: BigUint<M>,
    pub total_matched: BigUint<M>,
    pub matched_parts: ManagedVec<M, MatchedPart<M>>,
    pub potential_profit: BigUint<M>,
    pub odd: BigUint<M>,
    pub bet_type: BetType,
    pub status: BetStatus,
    pub payment_token: EgldOrEsdtTokenIdentifier<M>,
    pub payment_nonce: u64,
    pub nft_nonce: u64,
    pub created_at: u64,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct BetAttributes<M:ManagedTypeApi>{
    pub event: u64,     
    pub selection: Selection<M>,     
    pub stake: BigUint<M>, 
    pub potential_win: BigUint<M>,     
    pub odd: BigUint<M>,        
    pub bet_type: BetType,      
    pub status: BetStatus,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Market<M: ManagedTypeApi> {
    pub market_id: u64,
    pub event_id: u64,
    pub description: ManagedBuffer<M>,
    pub market_type: MarketType, 
    pub selections: ManagedVec<M, Selection<M>>,
    pub close_timestamp: u64,
    pub market_status: MarketStatus,
    pub total_matched_amount: BigUint<M>,
    pub liquidity: BigUint<M>,
    pub created_at: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, ManagedVecItem, Copy)]
pub enum SelectionType {
    One,    // "1"
    Draw,   // "X"
    Two,    // "2"
    Over,   // "OVER"
    Under,  // "UNDER"
    Yes,    // "YES"
    No      // "NO"
}

impl SelectionType {
    pub fn to_string(&self) -> &'static str {
        match self {
            SelectionType::One => "1",
            SelectionType::Draw => "X",
            SelectionType::Two => "2",
            SelectionType::Over => "OVER",
            SelectionType::Under => "UNDER",
            SelectionType::Yes => "YES",
            SelectionType::No => "NO",
        }
    }

    pub fn from_market_type_and_index(market_type: &MarketType, index: usize) -> Self {
        match market_type {
            MarketType::FullTimeResult => match index {
                0 => SelectionType::One,
                1 => SelectionType::Draw,
                2 => SelectionType::Two,
                _ => panic!("Invalid selection index for FullTimeResult")
            },
            MarketType::TotalGoals => match index {
                0 => SelectionType::Over,
                1 => SelectionType::Under,
                _ => panic!("Invalid selection index for TotalGoals")
            },
            MarketType::BothTeamsToScore => match index {
                0 => SelectionType::Yes,
                1 => SelectionType::No,
                _ => panic!("Invalid selection index for BothTeamsToScore")
            }
        }
    }
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct SelectionInfo {
    pub selection_id: u64,
    pub selection_type: SelectionType,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Selection<M: ManagedTypeApi> {
    pub id: u64,
    pub selection_type: SelectionType,
    pub priority_queue: Tracker<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct MarketSelectionInfo<M: ManagedTypeApi> {
    pub market_id: u64,
    pub market_type: MarketType,
    pub selections: ManagedVec<M, SelectionInfo>
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct EventMarketsCreationResponse<M: ManagedTypeApi> {
    pub event_id: u64,
    pub markets: ManagedVec<M, MarketSelectionInfo<M>>
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct PriceLevel<M: ManagedTypeApi> {
    pub odds: BigUint<M>,
    pub total_stake: BigUint<M>,
    pub bet_nonces: ManagedVec<M, u64>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Tracker<M: ManagedTypeApi> {
    pub back_levels: ManagedVec<M, PriceLevel<M>>,
    pub lay_levels: ManagedVec<M, PriceLevel<M>>,
    pub back_liquidity: BigUint<M>,
    pub lay_liquidity: BigUint<M>,
    pub matched_count: u64,
    pub unmatched_count: u64,
    pub partially_matched_count: u64,
    pub win_count: u64,
    pub lost_count: u64,
    pub canceled_count: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, ManagedVecItem, Copy)]
pub enum MarketType {
    FullTimeResult,
    TotalGoals,
    BothTeamsToScore,
}

impl MarketType {
    pub fn from_u64(value: u64) -> Self {
        match value {
            1 => MarketType::FullTimeResult,
            2 => MarketType::TotalGoals,
            3 => MarketType::BothTeamsToScore,
            _ => panic!("Invalid market type")
        }
    }

    pub fn to_u64(&self) -> u64 {
        match self {
            MarketType::FullTimeResult => 1,
            MarketType::TotalGoals => 2,
            MarketType::BothTeamsToScore => 3,
        }
    }

    pub fn to_description(&self) -> &[u8] {
        match self {
            MarketType::FullTimeResult => b"Fulltime Result",
            MarketType::TotalGoals => b"Over/Under 2.5 Goals",
            MarketType::BothTeamsToScore => b"Both Teams To Score",
        }
    }
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone)]
pub enum ProcessingStatus {
    InProgress,
    Completed
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone)]
pub struct ProcessingProgress {
    pub market_id: u64,
    pub processed_bets: u64,
    pub status: ProcessingStatus
}
