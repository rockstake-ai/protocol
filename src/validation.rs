use crate::constants::constants;
use crate::errors::{ERR_INVALID_MARKET, ERR_INVALID_SELECTION, ERR_LIABILITY_BACK_BET, ERR_LIABILITY_TOTAL_AMOUNT, ERR_LIABILITY_ZERO, ERR_MARKET_CLOSED, ERR_MARKET_NOT_OPEN, ERR_MARKET_TIMESTAMP, ERR_ODDS_OUT_OF_RANGE, ERR_STAKE_OUT_OF_RANGE, ERR_TOO_MANY_SELECTIONS};
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

    fn validate_bet_amount(&self, total_amount: &BigUint) -> SCResult<()> {
        let one_token = BigUint::from(1_000_000_000_000_000_000u64);
        let tokens = total_amount / &one_token;

        require!(
            tokens >= BigUint::from(1u32) && tokens <= BigUint::from(10000u32),
            ERR_STAKE_OUT_OF_RANGE
        );
        Ok(())
    }

    fn validate_bet_odds(&self, odds: &BigUint) -> SCResult<()> {
        require!(
            odds >= &BigUint::from(101u32) && odds <= &BigUint::from(100000u32),
            ERR_ODDS_OUT_OF_RANGE
        );
        Ok(())
    }
    
    fn validate_lay_bet(&self, liability: &BigUint, total_amount: &BigUint, odds: &BigUint) -> SCResult<(BigUint, BigUint)> {
        require!(liability > &BigUint::zero(), ERR_LIABILITY_ZERO);
        
        let stake = total_amount - liability;    
        let odds_minus_one = odds - &BigUint::from(100u32);
        let stake_check = (liability * &BigUint::from(100u32)) / odds_minus_one;
        require!(&stake == &stake_check, ERR_LIABILITY_TOTAL_AMOUNT);
        
        Ok((stake, liability.clone()))
    }

    fn validate_back_bet(&self, total_amount:&BigUint, liability: &BigUint) -> SCResult<(BigUint, BigUint)> {
        require!(
            liability == &BigUint::zero(),
            ERR_LIABILITY_BACK_BET
        );
        Ok((total_amount.clone(),BigUint::zero()))
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Market Validation (FOR ADMIN) -----------------------------//
    //--------------------------------------------------------------------------------------------//
    
    fn validate_market_creation(
        &self,
        close_timestamp: u64,
        selection_descriptions: &ManagedVec<ManagedBuffer>,
    ) -> SCResult<()> {
        self.validate_market_timestamp(close_timestamp)?;
        self.validate_total_selections(selection_descriptions)?;
        Ok(())
    }

    fn validate_market_timestamp(&self, close_timestamp: u64) -> SCResult<()> {
        require!(
            close_timestamp > self.blockchain().get_block_timestamp(),
            ERR_MARKET_TIMESTAMP
        );
        Ok(())
    }

    fn validate_total_selections(
        &self,
        selection_descriptions: &ManagedVec<ManagedBuffer>
    ) -> SCResult<()> {
        require!(!selection_descriptions.is_empty(), "No selections provided");
        require!(
            selection_descriptions.len() <= constants::MAX_SELECTIONS,
            ERR_TOO_MANY_SELECTIONS
        );
        Ok(())
    }

    fn validate_market_open_status(&self, market: &Market<Self::Api>) -> SCResult<()> {
        require!(
            market.market_status == MarketStatus::Open,
            ERR_MARKET_NOT_OPEN
        );
        require!(
            self.blockchain().get_block_timestamp() < market.close_timestamp,
            ERR_MARKET_CLOSED
        );
        Ok(())
    }

     //--------------------------------------------------------------------------------------------//
    //-------------------------------- Market Validation (FOR USER) -------------------------------//
    //---------------------------------------------------------------------------------------------//

    fn validate_market(&self, market_id: u64) -> SCResult<()> {
        require!(!self.markets(market_id).is_empty(), ERR_INVALID_MARKET);
        
        let market = self.markets(market_id).get();
        // require!(market.market_status == MarketStatus::Open, ERR_MARKET_NOT_OPEN);
        
        let created_at = self.blockchain().get_block_timestamp();
        require!(created_at < market.close_timestamp, ERR_MARKET_CLOSED);
        
        Ok(())
    }

    fn validate_selection(&self, market_id: u64, selection_id: u64) -> SCResult<()> {
        let market = self.markets(market_id).get();
        let selection_exists = market
            .selections
            .iter()
            .any(|s| s.selection_id == selection_id);
        require!(selection_exists, ERR_INVALID_SELECTION);
        Ok(())
    }

    fn validate_user_exposure(
        &self,
        user: &ManagedAddress<Self::Api>,
        stake: &BigUint
    ) -> SCResult<()> {
        let current_exposure = self.user_total_exposure(user).get();
        let new_exposure = &current_exposure + stake;
        
        require!(
            new_exposure <= BigUint::from(constants::MAX_USER_EXPOSURE),
            "Exceeds maximum user exposure limit"
        );
        Ok(())
    }

    fn get_and_validate_next_market_id(&self) -> SCResult<u64> {
        let mut counter = self.market_counter().get();
        counter += 1;
        
        require!(
            counter <= constants::MAX_MARKETS,
            "Maximum number of markets reached"
        );
        
        self.market_counter().set(&counter);
        Ok(counter)
    }
}
