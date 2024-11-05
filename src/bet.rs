use crate::{
    errors::{
        ERR_INVALID_SELECTION, ERR_LIABILITY_BACK_BET, ERR_LIABILITY_TOTAL_AMOUNT, ERR_LIABILITY_ZERO
    }, 
    types::{Bet, BetStatus, BetType}
};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetModule: 
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::nft::NftModule +
    crate::tracker::TrackerModule +
    crate::validation::ValidationModule 
{
    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(
        &self,
        market_id: u64,
        selection_id: u64,
        odds: BigUint,
        bet_type: BetType,
        liability: BigUint
    ) -> SCResult<(u64, BigUint, BigUint)> {
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, total_amount) = self
            .call_value()
            .egld_or_single_esdt()
            .into_tuple();

        self.validate_bet_amount(&total_amount)?;
        self.validate_bet_odds(&odds)?;
        self.validate_market(market_id)?;
        self.validate_selection(market_id, selection_id)?;
        
        // Calculăm stake și liability
        let (final_stake, final_liability) = self.calculate_stake_and_liability(
            &bet_type,
            &total_amount,
            &liability,
            &odds
        )?;

        // Creăm pariul
        let bet = self.create_bet(
            market_id,
            selection_id,
            &caller,
            &final_stake,
            &final_liability,
            &odds,
            bet_type,
            token_identifier.clone(),
            token_nonce
        )?;

        // Procesăm pariul prin tracker
        let (matched_amount, unmatched_amount) = self.process_bet(bet.clone());
        
        // Actualizăm starea pariului
        let updated_bet = self.update_bet_status(bet, matched_amount.clone(), unmatched_amount.clone())?;

        // Actualizăm market și selection
        self.update_market_and_selection(
            market_id,
            selection_id,
            &matched_amount,
        )?;

        // Gestionăm NFT și fonduri blocate
        self.handle_nft_and_locked_funds(
            &caller,
            &updated_bet,
            &unmatched_amount,
            &final_liability,
            bet_type
        )?;

        // Emitem eveniment
        self.emit_bet_placed_event(
            &updated_bet,
            &token_identifier,
            token_nonce,
            &matched_amount,
            &unmatched_amount
        );

        Ok((updated_bet.nft_nonce, odds, final_stake))
    }

    fn calculate_stake_and_liability(
        &self,
        bet_type: &BetType,
        total_amount: &BigUint,
        liability: &BigUint,
        odds: &BigUint
    ) -> SCResult<(BigUint, BigUint)> {
        match bet_type {
            BetType::Back => {
                require!(liability == &BigUint::zero(), ERR_LIABILITY_BACK_BET);
                Ok((total_amount.clone(), BigUint::zero()))
            },
            BetType::Lay => {
                require!(liability > &BigUint::zero(), ERR_LIABILITY_ZERO);
                let stake = total_amount - liability;    
                let odds_minus_one = odds - &BigUint::from(100u32);
                let stake_check = (liability * &BigUint::from(100u32)) / odds_minus_one;
                require!(stake == stake_check, ERR_LIABILITY_TOTAL_AMOUNT);
                Ok((stake, liability.clone()))
            }
        }
    }

    fn create_bet(
        &self,
        market_id: u64,
        selection_id: u64,
        caller: &ManagedAddress<Self::Api>,
        stake: &BigUint,
        liability: &BigUint,
        odds: &BigUint,
        bet_type: BetType,
        token_identifier: EgldOrEsdtTokenIdentifier<Self::Api>,
        token_nonce: u64
    ) -> SCResult<Bet<Self::Api>> {
        let market = self.markets(market_id).get();
        let selection = market.selections
            .iter()
            .find(|s| s.selection_id == selection_id)
            .ok_or(ERR_INVALID_SELECTION)?
            .clone();
        let bet_id = self.get_last_bet_id() + 1;
        
        Ok(Bet {
            bettor: caller.clone(),
            event: market_id,
            selection,
            stake_amount: stake.clone(),
            liability: liability.clone(),
            matched_amount: BigUint::zero(),
            unmatched_amount: stake.clone(),
            potential_profit: self.calculate_potential_profit(&bet_type, stake, odds),
            odd: odds.clone(),
            bet_type,
            status: BetStatus::Unmatched,
            payment_token: token_identifier,
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
            created_at: self.blockchain().get_block_timestamp()
        })
    }

    fn update_bet_status(
        &self,
        mut bet: Bet<Self::Api>,
        matched_amount: BigUint,
        unmatched_amount: BigUint
    ) -> SCResult<Bet<Self::Api>> {
        bet.matched_amount = matched_amount.clone();
        bet.unmatched_amount = unmatched_amount.clone();
        bet.status = if matched_amount > BigUint::zero() {
            if unmatched_amount > BigUint::zero() {
                BetStatus::PartiallyMatched
            } else {
                BetStatus::Matched
            }
        } else {
            BetStatus::Unmatched
        };
        Ok(bet)
    }

    fn update_market_and_selection(
        &self,
        market_id: u64,
        selection_id: u64,
        matched_amount: &BigUint,
    ) -> SCResult<()> {
        let mut market = self.markets(market_id).get();
        let selection_index = market
            .selections
            .iter()
            .position(|s| s.selection_id == selection_id)
            .ok_or("Invalid selection")?;
        
        let mut selection = market.selections.get(selection_index);
        selection.priority_queue = self.selection_tracker(market_id, selection_id).get();
        
        let _ = market.selections.set(selection_index, &selection);
        market.total_matched_amount += matched_amount;
        self.markets(market_id).set(&market);

        Ok(())
    }

    fn handle_nft_and_locked_funds(
        &self,
        caller: &ManagedAddress<Self::Api>,
        bet: &Bet<Self::Api>,
        unmatched_amount: &BigUint,
        liability: &BigUint,
        bet_type: BetType
    ) -> SCResult<()> {
        let bet_nft_nonce = self.mint_bet_nft(bet);
        self.bet_by_id(bet.nft_nonce).set(bet);

        let total_locked = match bet_type {
            BetType::Back => unmatched_amount.clone(),
            BetType::Lay => liability.clone(),
        };
        self.locked_funds(caller).update(|current_locked| *current_locked += &total_locked);

        self.send().direct_esdt(
            caller,
            self.bet_nft_token().get_token_id_ref(),
            bet_nft_nonce,
            &BigUint::from(1u64)
        );
        
        Ok(())
    }

    fn emit_bet_placed_event(
        &self,
        bet: &Bet<Self::Api>,
        token_identifier: &EgldOrEsdtTokenIdentifier<Self::Api>,
        token_nonce: u64,
        matched_amount: &BigUint,
        unmatched_amount: &BigUint
    ) {
        self.place_bet_event(
            &bet.bettor,
            self.bet_nft_token().get_token_id_ref(),
            &bet.event,
            &bet.selection.selection_id,
            &bet.stake_amount,
            &bet.odd,
            bet.bet_type,
            token_identifier,
            token_nonce,
            matched_amount,
            unmatched_amount,
            &bet.liability
        );
    }

    fn calculate_potential_profit(
        &self, 
        bet_type: &BetType, 
        stake: &BigUint, 
        odds: &BigUint
    ) -> BigUint {
        match bet_type {
            BetType::Back => {
                (odds - &BigUint::from(100u32)) * stake / &BigUint::from(100u32)
            },
            BetType::Lay => stake.clone()
        }
    }

    fn get_last_bet_id(&self) -> u64 {
        self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            self.bet_nft_token().get_token_id_ref(),
        )
    }
}