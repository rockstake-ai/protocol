use crate::{constants::precision_factor, storage::{self, Bet, BetType, Market, Status}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: storage::StorageModule 
    + crate::events::EventsModule 
    + crate::nft_manager::NftManagerModule{

    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(&self, market_id: u64, selection_id: u64, odds: BigUint, bet_type: BetType) -> SCResult<(u64, BigUint, BigUint)> {
        let current_timestamp = self.blockchain().get_block_timestamp();
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, token_amount) = self.call_value().egld_or_single_esdt().into_tuple();
    
        let bet_id = self.get_last_bet_id() + 1;
    
        let mut market = self.markets(&market_id).get();
        require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
        require!(current_timestamp < market.close_timestamp, "Market is closed");
    
        let selection_index = market.selections.iter()
            .position(|s| &s.selection_id == &selection_id)
            .expect("Selection not found in this market");
        let mut selection = market.selections.get(selection_index);

        let precision_factor = precision_factor::<Self::Api>();
        let odds_decimal = &odds / &precision_factor;
        let best_lay_decimal = &selection.best_lay_odds / &precision_factor;
        let best_back_decimal = &selection.best_back_odds / &precision_factor;
        
        match bet_type {
            BetType::Back => {
                if best_lay_decimal == BigUint::zero() || odds_decimal <= best_lay_decimal {
                    if best_back_decimal == BigUint::zero() || odds_decimal > best_back_decimal {
                        selection.best_back_odds = odds.clone();
                    }
                    selection.back_liquidity += &token_amount;
                } else {
                    return sc_error!("Back odds must be less than or equal to the best Lay odds");
                }
            },
            BetType::Lay => {
                if best_back_decimal == BigUint::zero() || odds_decimal > best_back_decimal {
                    if best_lay_decimal == BigUint::zero() || odds_decimal < best_lay_decimal {
                        selection.best_lay_odds = odds.clone();
                    }
                    selection.lay_liquidity += &token_amount;
                } else {
                    return sc_error!("Lay odds must be greater than the best Back odds");
                }
            }
        }
        // Folosim o referință la bet_type pentru a o putea utiliza în multiple locuri
        let (initial_status, matched_amount) = self.try_match_bet(&mut market, &selection_id, &bet_type, &odds, &token_amount);
    
        let remaining_amount = &token_amount - &matched_amount;
        
        let win_amount = self.calculate_win_amount(&bet_type, &token_amount, &odds);
        
        let bet = Bet {
            bettor: caller.clone(),
            event: market_id.clone(),
            selection: selection.clone(),
            stake_amount: token_amount.clone(),
            win_amount,
            odd: odds.clone(),
            bet_type: bet_type.clone(),
            status: initial_status,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
        };
        
    
        let bet_nft_nonce = self.mint_bet_nft(&bet);
        self.bet_by_id(bet_id).set(&bet);
        market.bets.push(bet.clone());

        let _ = market.selections.set(selection_index, &selection);
        self.markets(&market_id).set(&market);
    
        if remaining_amount > BigUint::zero() {
            self.locked_funds(&caller).update(|current_locked| *current_locked += &remaining_amount);
        }
    
        self.send().direct_esdt(&caller, self.bet_nft_token().get_token_id_ref(), bet_nft_nonce, &BigUint::from(1u64));
        self.bet_placed_event(
            &caller,
            self.bet_nft_token().get_token_id_ref(),
            &market_id,
            &selection_id,
            &token_amount,
            &odds,
            bet_type, // Folosim bet_type aici fără referință sau clonare
            &token_identifier,
            token_nonce,
            &matched_amount,
            &remaining_amount
        );
    
        Ok((bet_id, odds, token_amount))
    }

    fn try_match_bet(
        &self,
        market: &mut Market<Self::Api>,
        selection_id: &u64,
        bet_type: &BetType,
        odds: &BigUint,
        amount: &BigUint
    ) -> (Status, BigUint) {
        let mut matched_amount = BigUint::zero();
        let mut remaining_amount = amount.clone();
    
        for i in 0..market.bets.len() {
            let mut existing_bet = market.bets.get(i);
            if existing_bet.selection.selection_id == selection_id.clone() &&
                existing_bet.status == Status::Unmatched &&
                existing_bet.bet_type != bet_type.clone() {
                // Corectăm condițiile de potrivire pentru Back și Lay
                if (bet_type == &BetType::Back && odds <= &existing_bet.odd) ||
                   (bet_type == &BetType::Lay && odds >= &existing_bet.odd) {
                    let match_amount = if remaining_amount < existing_bet.stake_amount {
                        remaining_amount.clone()
                    } else {
                        existing_bet.stake_amount.clone()
                    };
    
                    matched_amount += &match_amount;
                    remaining_amount -= &match_amount;
                    existing_bet.stake_amount -= &match_amount;
    
                    if existing_bet.stake_amount == BigUint::zero() {
                        existing_bet.status = Status::Matched;
                    }
    
                    let _ = market.bets.set(i, &existing_bet);
    
                    if remaining_amount == BigUint::zero() {
                        break;
                    }
                }
            }
        }
    
        let status = if matched_amount == *amount {
            Status::Matched
        } else {
            Status::Unmatched
        };
    
        (status, matched_amount)
    }

    fn calculate_win_amount(&self, bet_type: &BetType, stake_amount: &BigUint, odds: &BigUint) -> BigUint {
        let precision_factor = precision_factor::<Self::Api>();
        let thousand = BigUint::from(1000u32);
        
        // Ajustăm limitele pentru a ține cont de precizia înaltă
        let min_odds = BigUint::from(1010u32) * &precision_factor / &thousand;
        let max_odds = BigUint::from(1000000u32) * &precision_factor / &thousand;
    
        // Verificăm dacă cotele sunt în intervalul valid
        require!(
            odds >= &min_odds && odds <= &max_odds,
            "Odds must be between 1.01 and 1000.000"
        );
    
        match bet_type {
            BetType::Back => {
                // Formula: stake_amount * (odds - precision_factor) / precision_factor
                stake_amount * &(odds - &precision_factor) / &precision_factor
            },
            BetType::Lay => {
                // Formula: stake_amount * precision_factor / (odds - precision_factor)
                (stake_amount * &precision_factor) / (odds - &precision_factor)
            },
        }
    }
           
}