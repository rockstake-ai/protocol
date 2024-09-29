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
    pub option: BigUint<M>,     // ID-ul selecției (ex: 1 = First Team Win)
    pub value: BigUint<M>,      // Suma pariată
    pub odd: BigUint<M>,        // Cota la care s-a plasat pariul
    pub bet_type: BetType,      // BACK sau LAY (adăugat)
    pub status: Status,         // Starea pariului (InProgress, Matched, etc.)
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

#[multiversx_sc::module]
pub trait StorageModule {
    #[view]
    #[storage_mapper("ticker")]
    fn token_manager(&self) -> NonFungibleTokenMapper<Self::Api>;

    #[view(getBetslipData)]
    fn get_betslip(&self, betslip_id: u64) -> Betslip<Self::Api> {
        let betslip_mapper = self.betslip_by_id(betslip_id);
        require!(!betslip_mapper.is_empty(), ERR_INVALID_STREAM);
        betslip_mapper.get()
    }

    fn get_last_betslip_id(&self) -> u64 {
        self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            self.betslip_nft_token().get_token_id_ref(),
        )
    }

    #[storage_mapper("betslipById")]
    fn betslip_by_id(&self, betslip_id: u64) -> SingleValueMapper<Betslip<Self::Api>>;
    #[storage_mapper("betslipNftToken")]
    fn betslip_nft_token(&self) -> NonFungibleTokenMapper<Self::Api>;
    #[storage_mapper("betslipNftBaseUri")]
    fn betslip_nft_base_uri(&self) -> SingleValueMapper<ManagedBuffer>;

    #[storage_mapper("markets")]
    fn markets(&self, market_id: &BigUint) -> SingleValueMapper<Market<Self::Api>>;
    #[storage_mapper("lockedFunds")]
    fn locked_funds(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint<Self::Api>>;

}

