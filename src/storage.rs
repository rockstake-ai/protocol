use crate::errors::ERR_INVALID_STREAM;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone, ManagedVecItem)]
pub enum Status {
    InProgress,
    Matched,
    Unmatched,
    Win,
    Lost,
    Canceled,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone, ManagedVecItem)]
pub enum BetType {
    Back,
    Lay
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Bet<M: ManagedTypeApi> {
    pub event: BigUint<M>,      // ID-ul evenimentului (ex: Real Madrid vs Barcelona)
    pub selection: Selection<M>,     // ID-ul selecției (ex: 1 = First Team Win)
    pub stake_amount: BigUint<M>,      // Suma pariată
    pub win_amount: BigUint<M>,      // Suma pariată
    pub odd: BigUint<M>,        // Cota la care s-a plasat pariul
    pub bet_type: BetType,      // BACK sau LAY (adăugat)
    pub status: Status,         // Starea pariului (InProgress, Matched, etc.)
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, //e.g BOBER
    pub payment_nonce: u64,
    pub nft_nonce: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Selection<M: ManagedTypeApi> {
    pub selection_id: BigUint<M>,              // ID-ul unic al selecției
    pub description: ManagedBuffer<M>,         // Descrierea selecției (de ex. "Real Sociedad câștigă")
    pub back_liquidity: BigUint<M>,            // Lichiditatea disponibilă pentru BACK pe această selecție
    pub lay_liquidity: BigUint<M>,             // Lichiditatea disponibilă pentru LAY pe această selecție
    pub best_back_odds: BigUint<M>,            // Cele mai bune cote pentru BACK
    pub best_lay_odds: BigUint<M>,             // Cele mai bune cote pentru LAY
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct Market<M:ManagedTypeApi>{
    pub market_id: BigUint<M>,              
    pub event_id: BigUint<M>,                  
    pub description: ManagedBuffer<M>,   
    pub selections: ManagedVec<M,Selection<M>>,
    pub back_liquidity: BigUint<M>,        
    pub lay_liquidity: BigUint<M>,          
    pub best_back_odds: BigUint<M>,         
    pub best_lay_odds: BigUint<M>,          
    pub bets: ManagedVec<M, Bet<M>>,
    pub close_timestamp: u64, // Timestamp când piața se închide
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Betslip<M:ManagedTypeApi>{
    pub creator: ManagedAddress<M>,
    pub bets: ManagedVec<M, Bet<M>>, //Bet
    pub total_odd: BigUint<M>, //132.55
    pub stake: BigUint<M>, //123.55
    pub payout: BigUint<M>, //stake * total_odd
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, //e.g BOBER
    pub payment_nonce: u64,
    // pub status: Status, //Status
    pub nft_nonce: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct BetslipAttributes<M:ManagedTypeApi>{
    pub creator: ManagedAddress<M>,
    pub bets: ManagedVec<M, Bet<M>>, //Bet
    pub total_odd: BigUint<M>, //132.55
    pub stake: BigUint<M>, //123.55
    pub payout: BigUint<M>, //stake * total_odd
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, //e.g BOBER
    pub payment_nonce: u64,
    // pub status: Status, //Status
    pub is_paid: bool,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct BetAttributes<M:ManagedTypeApi>{
    pub event: BigUint<M>,      // ID-ul evenimentului (ex: Real Madrid vs Barcelona)
    pub option: BigUint<M>,     // ID-ul selecției (ex: 1 = First Team Win)
    pub stake_amount: BigUint<M>,      // Suma pariată
    pub win_amount: BigUint<M>,      // Suma pariată
    pub odd: BigUint<M>,        // Cota la care s-a plasat pariul
    pub bet_type: BetType,      // BACK sau LAY (adăugat)
    pub status: Status,         // Starea pariului (InProgress, Matched, etc.)
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, //e.g BOBER
    pub payment_nonce: u64,
}

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(getBetslipData)]
    fn get_betslip(&self, betslip_id: u64) -> Betslip<Self::Api> {
        let betslip_mapper = self.betslip_by_id(betslip_id);
        require!(!betslip_mapper.is_empty(), ERR_INVALID_STREAM);
        betslip_mapper.get()
    }

    fn get_last_bet_id(&self) -> u64 {
        self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            self.betslip_nft_token().get_token_id_ref(),
        )
    }

    #[view(isMarketOpen)]
    fn is_market_open(&self, market_id: BigUint) -> bool {
        if self.markets(&market_id).is_empty() {
            return false;
        }
        
        let market = self.markets(&market_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
        
        current_timestamp < market.close_timestamp
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
    fn market_counter(&self) -> SingleValueMapper<BigUint>;
    #[storage_mapper("markets")]
    fn markets(&self, market_id: &BigUint) -> SingleValueMapper<Market<Self::Api>>;
    #[storage_mapper("lockedFunds")]
    fn locked_funds(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint<Self::Api>>;

}

