use crate::constants::constants;
use crate::types::{Market, MarketStatus};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait ValidationModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
{
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Bet Validation --------------------------------------------//
    //--------------------------------------------------------------------------------------------//    

    fn validate_bet_amount(&self, total_amount: &BigUint) {
        let one_token = BigUint::from(1_000_000_000_000_000_000u64);
        let tokens = (total_amount * &BigUint::from(10u32)) / &one_token; 
        
        require!(
            tokens >= BigUint::from(1u32) && tokens <= BigUint::from(100000u32),
            "Stake amount out of range"
        );
    }

    fn validate_bet_odds(&self, odds: &BigUint) {
        require!(
            odds >= &BigUint::from(101u32) && odds <= &BigUint::from(100000u32),
            "Odds out of range"
        );
    }
    
    
    fn validate_lay_bet(&self, total_amount: &BigUint, odds: &BigUint) -> (BigUint, BigUint) {
        let odds_minus_one = odds - &BigUint::from(100u32);
        
        let max_stake = total_amount.clone() * &BigUint::from(100u32) / odds.clone();
        let stake = max_stake.clone();
        
        let liability = (stake.clone() * odds_minus_one.clone()) / &BigUint::from(100u32);
        
        require!(
            total_amount >= &(&stake + &liability),
            "Insufficient funds for lay bet liability"
        );
        
        (stake, liability)
    }

    fn validate_back_bet(&self, total_amount: &BigUint) -> (BigUint, BigUint) {
        // Pentru back bet, total_amount = stake și liability = 0
        (total_amount.clone(), BigUint::zero())
    }
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Market Validation (FOR ADMIN) -----------------------------//
    //--------------------------------------------------------------------------------------------//
    
    fn validate_market_creation(&self, close_timestamp: u64) {
        self.validate_market_timestamp(close_timestamp);
    }

    fn validate_market_timestamp(&self, close_timestamp: u64) {
        require!(
            close_timestamp > self.blockchain().get_block_timestamp(),
            "Invalid market timestamp"
        );
    }

    fn validate_market_open_status(&self, market: &Market<Self::Api>) {
        require!(
            market.market_status == MarketStatus::Open,
            "Market not open"
        );
        require!(
            self.blockchain().get_block_timestamp() < market.close_timestamp,
            "Market already closed"
        );
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Market Validation (FOR USER) -------------------------------//
    //---------------------------------------------------------------------------------------------//

    fn validate_market(&self, market_id: u64) {
        require!(!self.markets(market_id).is_empty(), "Invalid market");
        
        let market = self.markets(market_id).get();
        let current_timestamp = self.blockchain().get_block_timestamp();
        
        require!(
            market.market_status == MarketStatus::Open,
            "Market is not open for betting"
        );
        
        require!(
            current_timestamp < market.close_timestamp,
            "Market already closed"
        );
    }
    
    fn validate_market_status(&self, market_id: u64) -> bool {
        if self.markets(market_id).is_empty() {
            return false;
        }
        
        let market = self.markets(market_id).get();
        market.market_status == MarketStatus::Open
    }

    fn validate_selection(&self, market_id: u64, selection_id: u64) {
        let market = self.markets(market_id).get();
        let selection_exists = market
            .selections
            .iter()
            .any(|s| s.id == selection_id);
            
        require!(selection_exists, "Invalid selection");
    }

    #[view(getNextMarketId)]
    fn get_next_market_id(&self) -> u64 {
        let current_counter = self.market_counter().get();
        let next_counter = current_counter + 1;
        self.market_counter().set(&next_counter);
        next_counter
    }
    
}