use crate::{errors::ERR_INVALID_STREAM, priority_queue::PriorityQueue, types::{Bet, BetType, Market}};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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
    #[storage_mapper("market_queues")]
    fn market_queues(&self, market_id: u64) -> SingleValueMapper<PriorityQueue<Self::Api>>;

}

