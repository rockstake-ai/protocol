multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone, ManagedVecItem)]
pub enum Status {
    InProgress,
    Win,
    Lost,
    Nul //odd 1
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct Bet<M:ManagedTypeApi>{
    pub event: BigUint<M>, //1231312 -> Real Madrid vs Barcelona
    pub option: BigUint<M>, //21 - Total Goals
    pub value: BigUint<M>, //5 - Over 2.5
    pub odd: BigUint<M>, //2.15
    pub status: Status //pending
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
    pub status: Status, //Status
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
    pub status: Status, //Status
    pub is_paid: bool,
}

pub type BetslipAttributesAsMultiValue<M> =
MultiValue7<ManagedVec<M, Bet<M>>, BigUint<M>, BigUint<M>, BigUint<M>, EgldOrEsdtTokenIdentifier<M>, Status, bool>;

//p2p - TODO()
#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct BetParticipant<M: ManagedTypeApi> {
    pub address: ManagedAddress<M>,
    pub option_chosen: ManagedBuffer<M>,
    pub stake: BigUint<M>,
    pub nft_id: Option<u64>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct P2PBet<M: ManagedTypeApi> {
    pub bet_id: ManagedBuffer<M>,
    pub creator: ManagedAddress<M>,
    pub event_details: ManagedBuffer<M>,
    pub options: ManagedVec<M, ManagedBuffer<M>>,
    pub odds: ManagedVec<M, BigUint<M>>,
    pub total_pool: BigUint<M>,
    pub participants: ManagedVec<M, BetParticipant<M>>,
    pub is_active: bool,
    pub result_declared: bool,
    pub winning_option: ManagedBuffer<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct ParticipationNFT<M: ManagedTypeApi> {
    pub bet_id: ManagedBuffer<M>,
    pub option_chosen: ManagedBuffer<M>,
    pub stake: BigUint<M>,
}

#[multiversx_sc::module]
pub trait StorageModule {
    #[view]
    #[storage_mapper("ticker")]
    fn token_manager(&self) -> NonFungibleTokenMapper<Self::Api>;

    fn get_last_betslip_id(&self) -> u64 {
        self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            self.betslip_nft_token().get_token_id_ref(),
        )
    }

    #[storage_mapper("betslipById")]
    fn betslip_by_id(&self, stream_id: u64) -> SingleValueMapper<Betslip<Self::Api>>;

    #[storage_mapper("betslipNftToken")]
    fn betslip_nft_token(&self) -> NonFungibleTokenMapper<Self::Api>;

    #[storage_mapper("betslipNftBaseUri")]
    fn betslip_nft_base_uri(&self) -> SingleValueMapper<ManagedBuffer>;

    #[storage_mapper("uniqueHashes")]
    fn used_hashes(&self, hash: &ManagedBuffer) -> SingleValueMapper<bool>;

    #[storage_mapper("deposit")]
    fn deposit(&self, address: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[storage_mapper("p2pBets")]
    fn p2p_bets(&self, bet_id: &ManagedBuffer) -> SingleValueMapper<P2PBet<Self::Api>>;

    #[storage_mapper("activeBets")]
    fn active_bets(&self) -> MapMapper<ManagedBuffer, bool>;

    #[view(getCrowdfundingTokenIdentifier)]
    #[storage_mapper("tokenIdentifier")]
    fn cf_token_identifier(&self) -> SingleValueMapper<EgldOrEsdtTokenIdentifier>;

    #[storage_mapper("ticket_counter")]
    fn ticket_counter(&self) -> SingleValueMapper<u64>;
}

