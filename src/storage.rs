use crate::errors::ERR_INVALID_STREAM;

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
    Open,    // Piața este deschisă pentru pariuri
    Closed,  // Piața este închisă pentru pariuri noi, dar încă nu s-a stabilit rezultatul
    Settled  // Rezultatul a fost stabilit și pariurile au fost decontate
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Bet<M: ManagedTypeApi> {
    pub bettor: ManagedAddress<M>,
    pub event: u64, // ID-ul evenimentului (ex: Real Madrid vs Barcelona)
    pub selection: Selection<M>, // ID-ul selecției (ex: 1 = First Team Win)
    pub stake_amount: BigUint<M>, // Miza efectivă
    pub collateral: BigUint<M>, // Garanția blocată (pentru pariuri LAY)
    pub matched_amount: BigUint<M>, // Suma potrivită
    pub unmatched_amount: BigUint<M>, // Suma nepotrivită
    pub potential_profit: BigUint<M>, // Profitul potențial (înlocuiește win_amount)
    pub potential_liability: BigUint<M>, // Pierderea potențială maximă (pentru pariuri LAY)
    pub odd: BigUint<M>, // Cota la care s-a plasat pariul
    pub bet_type: BetType, // BACK sau LAY
    pub status: BetStatus, // Starea pariului (Unmatched, PartiallyMatched, Matched, etc.)
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, // e.g BOBER
    pub payment_nonce: u64,
    pub nft_nonce: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Selection<M: ManagedTypeApi> {
    pub selection_id: u64,              // ID-ul unic al selecției
    pub description: ManagedBuffer<M>,         // Descrierea selecției (de ex. "Real Sociedad câștigă")
    pub back_liquidity: BigUint<M>,            // Lichiditatea disponibilă pentru BACK pe această selecție
    pub lay_liquidity: BigUint<M>,             // Lichiditatea disponibilă pentru LAY pe această selecție
    pub best_back_odds: BigUint<M>,            // Cele mai bune cote pentru BACK
    pub best_lay_odds: BigUint<M>,             // Cele mai bune cote pentru LAY
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
    pub close_timestamp: u64, // Timestamp când piața se închide
    pub market_status: MarketStatus, // Adăugat: starea curentă a pieței
    pub total_matched_amount: BigUint<M>, // Adăugat: suma totală potrivită în această piață
    pub created_timestamp: u64, // Adăugat: timestamp-ul creării pieței
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct BetAttributes<M:ManagedTypeApi>{
    pub bettor: ManagedAddress<M>,
    pub event_id: u64,     // ID-ul evenimentului (ex: Real Madrid vs Barcelona)
    pub selection: Selection<M>,     // ID-ul selecției (ex: 1 = First Team Win)
    pub stake_amount: BigUint<M>,      // Suma pariată
    pub win_amount: BigUint<M>,      // Suma pariată
    pub odd: BigUint<M>,        // Cota la care s-a plasat pariul
    pub bet_type: BetType,      // BACK sau LAY (adăugat)
    pub status: BetStatus,         // Starea pariului (InProgress, Matched, etc.)
    pub payment_token: EgldOrEsdtTokenIdentifier<M>, //e.g BOBER
    pub payment_nonce: u64,
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

}

