use crate::{types::BetScheduler, types::{Bet, BetType, Market}};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(getBetslipData)]
    fn get_bet(&self, bet_id: u64) -> Bet<Self::Api> {
        let bet_mapper = self.bet_by_id(bet_id);
        require!(!bet_mapper.is_empty(), "Invalid");
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
    fn unmatched_bets(&self, market_id: u64) -> SingleValueMapper<BetScheduler<Self::Api>>;
    #[storage_mapper("market_queues")]
    fn market_queues(&self, market_id: u64) -> SingleValueMapper<BetScheduler<Self::Api>>;

    #[view(getBetSchedulerStorage)]
    #[storage_mapper("bet_scheduler")]
    fn bet_scheduler(&self) -> SingleValueMapper<BetScheduler<Self::Api>>;


    #[view(getBackBets)]
    fn get_back_bets(&self) -> ManagedVec<Self::Api, Bet<Self::Api>> {
        let scheduler = self.bet_scheduler().get();
        scheduler.back_bets
    }

    #[view(getLayBets)]
    fn get_lay_bets(&self) -> ManagedVec<Self::Api, Bet<Self::Api>> {
        let scheduler = self.bet_scheduler().get();
        scheduler.lay_bets
    }

    #[view(getBestBackOdds)]
    fn get_best_back_odds(&self) -> BigUint {
        let scheduler = self.bet_scheduler().get();
        scheduler.best_back_odds
    }

    #[view(getBestLayOdds)]
    fn get_best_lay_odds(&self) -> BigUint {
        let scheduler = self.bet_scheduler().get();
        scheduler.best_lay_odds
    }

    #[view(getBackLiquidity)]
    fn get_back_liquidity(&self) -> BigUint {
        let scheduler = self.bet_scheduler().get();
        scheduler.back_liquidity
    }

    #[view(getLayLiquidity)]
    fn get_lay_liquidity(&self) -> BigUint {
        let scheduler = self.bet_scheduler().get();
        scheduler.lay_liquidity
    }

    #[view(getTopNBets)]
    fn get_top_n_bets(
        &self,
        bet_type: BetType,
        n: usize
    ) -> ManagedVec<Self::Api, Bet<Self::Api>> {
        let scheduler = self.bet_scheduler().get();
        let source = match bet_type {
            BetType::Back => &scheduler.back_bets,
            BetType::Lay => &scheduler.lay_bets,
        };
        let mut result = ManagedVec::new();
        for i in 0..n.min(source.len()) {
            result.push(source.get(i).clone());
        }
        result
    }

}

