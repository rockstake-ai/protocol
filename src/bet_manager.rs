use crate::{errors::{ERR_MARKET_CLOSED, ERR_INVALID_MARKET, ERR_MARKET_NOT_OPEN, ERR_BET_ODDS, ERR_SELECTION}, bet_scheduler::BetScheduler, types::{Bet, BetStatus, BetType, MarketStatus}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: crate::storage::StorageModule 
    + crate::events::EventsModule 
    + crate::nft_manager::NftManagerModule {

    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(&self, market_id: u64, selection_id: u64, odds: BigUint, bet_type: BetType) -> SCResult<(u64, BigUint, BigUint)> {
        let mut market = self.markets(&market_id).get();
        let created_at = self.blockchain().get_block_timestamp();

        require!(!self.markets(&market_id).is_empty(), ERR_INVALID_MARKET);
        require!(market.market_status == MarketStatus::Open, ERR_MARKET_NOT_OPEN);
        require!(created_at < market.close_timestamp, ERR_MARKET_CLOSED);
        require!(odds >= BigUint::from(101u32) && odds <= BigUint::from(100000u32), ERR_BET_ODDS);
    
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, total_amount) = self.call_value().egld_or_single_esdt().into_tuple();
        let bet_id = self.get_last_bet_id() + 1;
    
        let (stake, liability) = match bet_type {
            BetType::Back => {
                (total_amount.clone(), BigUint::zero())
            },
            BetType::Lay => {
                let stake = self.calculate_stake_from_total(&total_amount, &odds);
                let calculated_liability = self.calculate_potential_liability(&bet_type, &stake, &odds);
                let required_total = stake.clone() + &calculated_liability;
                
                require!(total_amount.clone() >= required_total, "Insufficient total amount");
                
                (stake, calculated_liability)
            }
        };
        
        let selection_index = market.selections.iter()
            .position(|s| &s.selection_id == &selection_id)
            .expect(ERR_SELECTION);
        let mut selection = market.selections.get(selection_index);
    
        let mut bet = Bet {
            bettor: caller.clone(),
            event: market_id,
            selection: selection.clone(),
            stake_amount: stake.clone(),
            liability: match bet_type {
                BetType::Back => BigUint::zero(),
                BetType::Lay => liability.clone(),
            },
            matched_amount: BigUint::zero(),
            unmatched_amount: stake.clone(),
            potential_profit: self.calculate_potential_profit(&bet_type, &stake, &odds),
            odd: odds.clone(),
            bet_type: bet_type.clone(),
            status: BetStatus::Unmatched,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
            created_at: created_at
        };
    
        let (matched_amount, unmatched_amount) = 
        selection.priority_queue.match_bet(&mut bet);
            
        let bet_nft_nonce = self.mint_bet_nft(&bet);
        self.bet_by_id(bet_id).set(&bet);
    
        let _ = market.selections.set(selection_index, &selection);
        let _ = market.total_matched_amount += &matched_amount;
        self.markets(&market_id).set(&market);
    
        let total_locked = match bet_type {
            BetType::Back => unmatched_amount.clone(),
            BetType::Lay => liability.clone(),
        };
        self.locked_funds(&caller).update(|current_locked| *current_locked += &total_locked);

        self.send().direct_esdt(&caller, self.bet_nft_token().get_token_id_ref(), bet_nft_nonce, &BigUint::from(1u64));
        
        self.bet_placed_event(
            &caller,
            self.bet_nft_token().get_token_id_ref(),
            &market_id,
            &selection_id,
            &stake,
            &odds,
            bet_type.clone(),
            &token_identifier,
            token_nonce,
            &matched_amount,
            &unmatched_amount,
            &liability
        );
    
        Ok((bet_id, odds, stake))
    }
    

    fn calculate_potential_profit(&self, bet_type: &BetType, stake: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => {
                (odds - &BigUint::from(100u32)) * stake / &BigUint::from(100u32)
            },
            BetType::Lay => stake.clone()  // Pentru Lay, profitul este stake-ul
        }
    }
    
    fn calculate_stake_from_total(&self, total: &BigUint, odds: &BigUint) -> BigUint {
        total * &BigUint::from(100u32) / odds
    }
    
    fn calculate_potential_liability(&self, bet_type: &BetType, stake: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => stake.clone(),
            BetType::Lay => {
                let odds_minus_100 = odds - &BigUint::from(100u32);
                let result = (stake * &odds_minus_100) / &BigUint::from(100u32);
                // sc_panic!(
                //     "Liability calculation: stake={}, odds={}, odds_minus_100={}, calculated_liability={}", 
                //     stake, 
                //     odds,
                //     odds_minus_100,
                //     result
                // );
                result
            }
        }
    }
    
    fn calculate_win_amount(&self, bet_type: &BetType, stake_amount: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => self.calculate_potential_profit(bet_type, stake_amount, odds),
            BetType::Lay => self.calculate_potential_liability(bet_type, stake_amount, odds),
        }
    }
           
}