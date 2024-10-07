use crate::{errors::ERR_INVALID_STREAM, priority_queue::PriorityQueue};

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
    pub potential_liability: BigUint<M>, 
    pub odd: BigUint<M>, 
    pub bet_type: BetType, 
    pub status: BetStatus, 
    pub payment_token: EgldOrEsdtTokenIdentifier<M>,
    pub payment_nonce: u64,
    pub nft_nonce: u64,
    pub timestamp: u64, // Adăugați acest câmp nou
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct BetAttributes<M:ManagedTypeApi>{
    pub bettor: ManagedAddress<M>,
    pub event: u64,     
    pub selection: Selection<M>,     
    pub stake_amount: BigUint<M>, 
    pub collateral: BigUint<M>,  
    pub matched_amount: BigUint<M>, 
    pub unmatched_amount: BigUint<M>,    
    pub potential_profit: BigUint<M>,     
    pub potential_liability: BigUint<M>,  
    pub odd: BigUint<M>,        
    pub bet_type: BetType,      
    pub status: BetStatus,         
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, 
    pub payment_nonce: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Selection<M: ManagedTypeApi> {
    pub selection_id: u64,              
    pub description: ManagedBuffer<M>,         
    pub back_liquidity: BigUint<M>,            
    pub lay_liquidity: BigUint<M>,             
    pub best_back_odds: BigUint<M>,            
    pub best_lay_odds: BigUint<M>,            
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct Market<M: ManagedTypeApi> {
    pub market_id: u64,
    pub event_id: u64,
    pub description: ManagedBuffer<M>,
    pub selections: ManagedVec<M, Selection<M>>,
    pub back_liquidity: BigUint<M>,
    pub lay_liquidity: BigUint<M>,
    pub best_back_odds: BigUint<M>,
    pub best_lay_odds: BigUint<M>,
    pub bets: ManagedVec<M, Bet<M>>,
    pub close_timestamp: u64, 
    pub market_status: MarketStatus, 
    pub total_matched_amount: BigUint<M>, 
    pub created_timestamp: u64, 
}

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(getBetslipData)]
    fn get_bet(&self, bet_id: u64) -> Bet<Self::Api> {
        let bet_mapper = self.bet_by_id(bet_id);
        require!(!bet_mapper.is_empty(), ERR_INVALID_STREAM);
        bet_mapper.get()
    }

    fn get_last_bet_id(&self) -> u64 {
        self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            self.bet_nft_token().get_token_id_ref(),
        )
    }

    #[view(isMarketOpen)]
    fn is_market_open(&self, market_id: u64) -> bool {
        if self.markets(&market_id).is_empty() {
            return false;
        }
        
        let market = self.markets(&market_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
        
        current_timestamp < market.close_timestamp
    }

    #[view(getMarketCounter)]
    fn get_market_counter(&self) -> u64 {
        self.market_counter().get()
    }


    #[storage_mapper("createBet")]
    fn create_bet(&self, market_id: BigUint, selection_id: BigUint, odds: BigUint, bet_type: BetType, 
        stake_amount: BigUint, token_identifier: EgldOrEsdtTokenIdentifier, 
        token_nonce: u64, bet_id: u64) -> SingleValueMapper<Bet<Self::Api>>;

    #[storage_mapper("betById")]
    fn bet_by_id(&self, bet_id: u64) -> SingleValueMapper<Bet<Self::Api>>;
    #[storage_mapper("betNftToken")]
    fn bet_nft_token(&self) -> NonFungibleTokenMapper<Self::Api>;
    #[storage_mapper("betNftBaseUri")]
    fn bet_nft_base_uri(&self) -> SingleValueMapper<ManagedBuffer>;

    #[storage_mapper("market_counter")]
    fn market_counter(&self) -> SingleValueMapper<u64>;
    #[storage_mapper("markets")]
    fn markets(&self, market_id: &u64) -> SingleValueMapper<Market<Self::Api>>;
    #[storage_mapper("lockedFunds")]
    fn locked_funds(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint<Self::Api>>;

    #[view(getPotentialLayLoss)]
    #[storage_mapper("potential_lay_loss")]
    fn potential_lay_loss(&self, bet_id: &u64) -> SingleValueMapper<BigUint>;

    #[view(getUnmatchedBets)]
    #[storage_mapper("unmatched_bets")]
    fn unmatched_bets(&self, market_id: u64) -> SingleValueMapper<PriorityQueue<Self::Api>>;

}

