use crate::storage::{self, Bet, BetType, Market, Status};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: storage::StorageModule 
    + crate::events::EventsModule 
    + crate::nft_manager::NftManagerModule{

    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(&self, market_id: BigUint, selection_id: BigUint, odds: BigUint, bet_type: BetType) -> u64 {
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
    
        // Folosim o referință la bet_type pentru a o putea utiliza în multiple locuri
        let (initial_status, matched_amount) = self.try_match_bet(&mut market, &selection_id, &bet_type, &odds, &token_amount);
    
        let remaining_amount = &token_amount - &matched_amount;
        let win_amount = self.calculate_win_amount(&bet_type, &token_amount, &odds);
    
        let selection = market.selections.get(selection_index);
        let bet = Bet {
            bettor: caller.clone(),
            event: market_id.clone(),
            selection: selection,
            stake_amount: token_amount.clone(),
            win_amount,
            odd: odds.clone(),
            bet_type: bet_type.clone(), // Clonăm bet_type aici
            status: initial_status,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
        };
    
        let bet_nft_nonce = self.mint_bet_nft(&bet);
    
        self.bet_by_id(bet_id).set(&bet);
        market.bets.push(bet.clone());
    
        // Actualizăm selecția
        let mut selection = market.selections.get(selection_index);
        match bet_type {
            BetType::Back => {
                selection.back_liquidity += &remaining_amount;
                if odds > selection.best_back_odds {
                    selection.best_back_odds = odds.clone();
                }
            },
            BetType::Lay => {
                selection.lay_liquidity += &remaining_amount;
                if odds < selection.best_lay_odds || selection.best_lay_odds == BigUint::zero() {
                    selection.best_lay_odds = odds.clone();
                }
            }
        }
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
    
        bet_id
    }

    fn try_match_bet(
        &self,
        market: &mut Market<Self::Api>,
        selection_id: &BigUint,
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
                if (bet_type.clone() == BetType::Back && odds >= &existing_bet.odd) ||
                    (bet_type.clone() == BetType::Lay && odds <= &existing_bet.odd) {
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
        let thousand = BigUint::from(1000u32);
        match bet_type {
            BetType::Back => {
                // (stake_amount * (odds - 1000)) / 1000
                (stake_amount * &(odds - &thousand)) / &thousand
            },
            BetType::Lay => {
                // (stake_amount * 1000) / (odds - 1000)
                (stake_amount * &thousand) / &(odds - &thousand)
            },
        }
    }    
           
}