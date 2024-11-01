use crate::{
    errors::{
        ERR_BET_ODDS, ERR_INVALID_MARKET, ERR_LIABILITY_BACK_BET, 
        ERR_LIABILITY_TOTAL_AMOUNT, ERR_LIABILITY_ZERO, ERR_MARKET_CLOSED, 
        ERR_MARKET_NOT_OPEN, ERR_SELECTION
    }, 
    types::{Bet, BetStatus, BetType, MarketStatus, Tracker}
};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: 
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::nft_manager::NftManagerModule +
    crate::tracker::TrackerModule 
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
        let mut market = self.markets(market_id).get();
        let created_at = self.blockchain().get_block_timestamp();
    
        require!(!self.markets(market_id).is_empty(), ERR_INVALID_MARKET);
        require!(market.market_status == MarketStatus::Open, ERR_MARKET_NOT_OPEN);
        require!(created_at < market.close_timestamp, ERR_MARKET_CLOSED);
        require!(
            odds >= BigUint::from(101u32) && odds <= BigUint::from(100000u32),
            ERR_BET_ODDS
        );
    
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, total_amount) = self
            .call_value()
            .egld_or_single_esdt()
            .into_tuple();
            
        let bet_id = self.get_last_bet_id() + 1;
            
        let (final_stake, final_liability) = match bet_type {
            BetType::Back => {
                require!(liability == BigUint::zero(), ERR_LIABILITY_BACK_BET);
                (total_amount.clone(), BigUint::zero())
            },
            BetType::Lay => {
                require!(liability > BigUint::zero(), ERR_LIABILITY_ZERO);
                let stake = &total_amount - &liability;    
                let odds_minus_one = &odds - &BigUint::from(100u32);
                let stake_check = (&liability * &BigUint::from(100u32)) / odds_minus_one;
                require!(
                    stake == stake_check,
                    ERR_LIABILITY_TOTAL_AMOUNT
                );
                
                (stake, liability)
            }
        };

        let selection_index = market
            .selections
            .iter()
            .position(|s| &s.selection_id == &selection_id)
            .expect(ERR_SELECTION);
        let mut selection = market.selections.get(selection_index);
        
        // Verificăm dacă tracker-ul există și îl inițializăm dacă nu
        // if self.selection_tracker(market_id, selection_id).is_empty() {
        //     self.init_tracker(market_id, selection_id);
        // }
    
        let bet = Bet {
            bettor: caller.clone(),
            event: market_id,
            selection: selection.clone(),
            stake_amount: final_stake.clone(),
            liability: final_liability.clone(),
            matched_amount: BigUint::zero(),
            unmatched_amount: final_stake.clone(),
            potential_profit: self.calculate_potential_profit(&bet_type, &final_stake, &odds),
            odd: odds.clone(),
            bet_type: bet_type.clone(),
            status: BetStatus::Unmatched,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
            created_at: created_at
        };
    
        // Process bet using tracker
        let (matched_amount, unmatched_amount) = self.process_bet(bet.clone());
        
        // Update bet with results
        let mut updated_bet = bet;
        updated_bet.matched_amount = matched_amount.clone();
        updated_bet.unmatched_amount = unmatched_amount.clone();
        updated_bet.status = if matched_amount > BigUint::zero() {
            if unmatched_amount > BigUint::zero() {
                BetStatus::PartiallyMatched
            } else {
                BetStatus::Matched
            }
        } else {
            BetStatus::Unmatched
        };
    
        // Update selection tracker
        selection.priority_queue = self.selection_tracker(market_id, selection_id).get();
        
        // Update market
        let _ = market.selections.set(selection_index, &selection);
        market.total_matched_amount += &matched_amount;
        self.markets(market_id).set(&market);
    
        // Create and save NFT
        let bet_nft_nonce = self.mint_bet_nft(&updated_bet);
        self.bet_by_id(bet_id).set(&updated_bet);
    
        // Update locked funds
        let total_locked = match bet_type {
            BetType::Back => unmatched_amount.clone(),
            BetType::Lay => final_liability.clone(),
        };
        self.locked_funds(&caller).update(|current_locked| *current_locked += &total_locked);
    
        // Send NFT to caller
        self.send().direct_esdt(
            &caller,
            self.bet_nft_token().get_token_id_ref(),
            bet_nft_nonce,
            &BigUint::from(1u64)
        );
    
        // Emit event
        self.place_bet_event(
            &caller,
            self.bet_nft_token().get_token_id_ref(),
            &market_id,
            &selection_id,
            &final_stake,
            &odds,
            bet_type,
            &token_identifier,
            token_nonce,
            &matched_amount,
            &unmatched_amount,
            &final_liability
        );
    
        Ok((bet_id, odds, final_stake))
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