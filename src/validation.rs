use crate::constants::constants::{self, MAX_ODDS, MAX_STAKE, MIN_ODDS, MIN_STAKE};
use crate::types::{Bet, BetType, MarketStatus};


multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetValidationModule:
crate::storage::StorageModule +
crate::events::EventsModule +{

    fn validate_bet_placement(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        // Validare sumă mizată
        require!(
            bet.stake_amount >= BigUint::from(constants::MIN_STAKE) &&
            bet.stake_amount <= BigUint::from(constants::MAX_STAKE),
            "Stake amount outside allowed range"
        );

        // Validare cotă
        require!(
            bet.odd >= BigUint::from(constants::MIN_ODDS) &&
            bet.odd <= BigUint::from(constants::MAX_ODDS),
            "Odds outside allowed range"
        );

        // Validări specifice pentru Lay
        if bet.bet_type == BetType::Lay {
            // Verificare liability
            require!(
                bet.liability > BigUint::zero(),
                "Liability must be greater than zero for Lay bets"
            );

            // Verificare relație corectă între stake și liability
            let odds_minus_one = &bet.odd - &BigUint::from(100u64);
            let expected_stake = (&bet.liability * &BigUint::from(100u64)) / &odds_minus_one;
            require!(
                bet.stake_amount == expected_stake,
                "Invalid stake/liability ratio for Lay bet"
            );
        } else {
            // Verificare pentru Back
            require!(
                bet.liability == BigUint::zero(),
                "Back bets should not have liability"
            );
        }

        // Validare market și selection
        self.validate_market_selection(bet)?;

        // Validare limită de expunere per utilizator
        self.validate_user_exposure(&bet.bettor, &bet.stake_amount)?;

        Ok(())
    }

    fn validate_market_selection(&self, bet: &Bet<Self::Api>) -> SCResult<()> {
        let market = self.markets(bet.event).get();
        
        // Verificare existență market
        require!(self.markets(market).is_empty(), ERR_MARKET_ALREADY_EXISTS);

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

    fn validate_user_exposure(
        &self,
        user: &ManagedAddress<Self::Api>,
        stake: &BigUint
    ) -> SCResult<()> {
        let current_exposure = self.user_total_exposure(user).get();
        let new_exposure = &current_exposure + stake;
        
        // Limită maximă de expunere per utilizator
        const MAX_USER_EXPOSURE: u64 = 10_000_000_000_000_000_000; // 10 EGLD
        
        require!(
            new_exposure <= BigUint::from(MAX_USER_EXPOSURE),
            "Exceeds maximum user exposure limit"
        );

        Ok(())
    }
}