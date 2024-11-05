use crate::constants::constants;
use crate::types::{Bet, BetType, Market, MarketStatus};

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
    
    fn validate_bet_placement(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        self.validate_bet_amount(bet)?;
        self.validate_bet_odds(bet)?;
        self.validate_bet_type_specifics(bet)?;
        self.validate_market_selection(bet)?;
        self.validate_user_exposure(&bet.bettor, &bet.stake_amount)?;
        Ok(())
    }

    fn validate_bet_amount(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        // Convertim la tokens împărțind la 10^18
        let one_token = BigUint::from(1_000_000_000_000_000_000u64);
        let tokens = &bet.stake_amount / &one_token;

        require!(
            tokens >= BigUint::from(1u32) && tokens <= BigUint::from(10000u32),
            "Stake amount outside allowed range"
        );
        Ok(())
    }

    fn validate_bet_odds(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        require!(
            bet.odd >= BigUint::from(constants::MIN_ODDS) &&
            bet.odd <= BigUint::from(constants::MAX_ODDS),
            "Odds outside allowed range"
        );
        Ok(())
    }

    fn validate_bet_type_specifics(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        match bet.bet_type {
            BetType::Lay => self.validate_lay_bet(bet),
            BetType::Back => self.validate_back_bet(bet),
        }
    }

    fn validate_lay_bet(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        require!(
            bet.liability > BigUint::zero(),
            "Liability must be greater than zero for Lay bets"
        );

        let odds_minus_one = &bet.odd - &BigUint::from(100u64);
        let expected_stake = (&bet.liability * &BigUint::from(100u64)) / &odds_minus_one;
        require!(
            bet.stake_amount == expected_stake,
            "Invalid stake/liability ratio for Lay bet"
        );
        Ok(())
    }

    fn validate_back_bet(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        require!(
            bet.liability == BigUint::zero(),
            "Back bets should not have liability"
        );
        Ok(())
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Market Validation -----------------------------------------//
    //--------------------------------------------------------------------------------------------//
    
    fn validate_market_creation(
        &self,
        close_timestamp: u64,
        selection_descriptions: &ManagedVec<ManagedBuffer>,
    ) -> SCResult<()> {
        self.validate_market_timestamp(close_timestamp)?;
        self.validate_selection_descriptions(selection_descriptions)?;
        Ok(())
    }

    fn validate_market_timestamp(&self, close_timestamp: u64) -> SCResult<()> {
        require!(
            close_timestamp > self.blockchain().get_block_timestamp(),
            "Invalid closing timestamp"
        );
        Ok(())
    }

    fn validate_selection_descriptions(
        &self,
        selection_descriptions: &ManagedVec<ManagedBuffer>
    ) -> SCResult<()> {
        require!(!selection_descriptions.is_empty(), "No selections provided");
        require!(
            selection_descriptions.len() <= constants::MAX_SELECTIONS,
            "Too many selections"
        );

        for desc in selection_descriptions.iter() {
            require!(
                desc.len() >= constants::MIN_DESCRIPTION_LENGTH &&
                desc.len() <= constants::MAX_DESCRIPTION_LENGTH,
                "Invalid selection description length"
            );
        }
        Ok(())
    }

    fn validate_market_selection(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        // Verificăm dacă există market-ul în storage
        require!(!self.markets(bet.event).is_empty(), "Market does not exist");
    
        let market = self.markets(bet.event).get();
        
        // Verificare status market
        require!(
            market.market_status == MarketStatus::Open,
            "Market is not open for betting"
        );
    
        // Verificare timpii de închidere
        let current_timestamp = self.blockchain().get_block_timestamp();
        require!(
            current_timestamp < market.close_timestamp,
            "Market is closed for betting"
        );
    
        // Verificare selection validă
        let selection_exists = market
            .selections
            .iter()
            .any(|s| s.selection_id == bet.selection.selection_id);
        require!(selection_exists, "Invalid selection ID");
    
        Ok(())
    }

    fn validate_market_open_status(&self, market: &Market<Self::Api>) -> SCResult<()> {
        require!(
            market.market_status == MarketStatus::Open,
            "Market is not open for betting"
        );
        require!(
            self.blockchain().get_block_timestamp() < market.close_timestamp,
            "Market is closed for betting"
        );
        Ok(())
    }

    fn validate_selection_exists(&self, bet: &Bet<Self::Api>, market: &Market<Self::Api>) -> SCResult<()> {
        let selection_exists = market
            .selections
            .iter()
            .any(|s| s.selection_id == bet.selection.selection_id);
        require!(selection_exists, "Invalid selection ID");
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