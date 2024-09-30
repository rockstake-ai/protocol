use crate::{errors::ERR_ZERO_DEPOSIT, storage::{self, Bet, BetType, Betslip, Market, Selection, Status}};
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
    
        require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
        let mut market = self.markets(&market_id).get();

        require!(current_timestamp < market.close_timestamp, "Market is closed");

        
        let mut selection = market.selections.iter_mut()
            .find(|s| s.selection_id == selection_id)
            .expect("Selection not found in this market");
        let (initial_status, matched_amount) = self.try_match_bet(&mut market, &selection, &bet_type, &odds, &token_amount);

        let remaining_amount = &token_amount - &matched_amount;
        let win_amount = self.calculate_win_amount(bet_type, &token_amount, &odds);
    
        let bet = Bet {
            event: market_id.clone(),
            selection: selection.clone(),
            stake_amount: token_amount.clone(),
            win_amount,
            odd: odds,
            bet_type,
            status: initial_status,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
        };
    
        let bet_nft_nonce = self.mint_bet_nft(&bet);
    
        self.bet_by_id(bet_id).set(&bet);
        market.bets.push(bet.clone());
    
        // Actualizăm lichiditatea
        match bet_type {
            BetType::Back => {
                selection.back_liquidity += &remaining_amount;
                if odds > selection.best_back_odds {
                    selection.best_back_odds = odds;
                }
            },
            BetType::Lay => {
                selection.lay_liquidity += &remaining_amount;
                if odds < selection.best_lay_odds || selection.best_lay_odds == BigUint::zero() {
                    selection.best_lay_odds = odds;
                }
            }
        }
    
        self.markets(&market_id).set(&market);
    
        // Blocăm doar suma rămasă nepotrivită
        if remaining_amount > BigUint::zero() {
            self.locked_funds(&caller).update(|current_locked| *current_locked += &remaining_amount);
        }
    
        self.send().direct_esdt(&caller, self.betslip_nft_token().get_token_id_ref(), bet_nft_nonce, &BigUint::from(1u64));
    
        self.bet_placed_event(
            &caller,
            self.betslip_nft_token().get_token_id_ref(),
            &market_id,
            &selection_id,
            &token_amount,
            &odds,
            bet_type,
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
        selection: &Selection<Self::Api>,
        bet_type: &BetType,
        odds: &BigUint,
        amount: &BigUint
    ) -> (Status, BigUint) {
        let mut matched_amount = BigUint::zero();
        let mut remaining_amount = amount.clone();
    
        for existing_bet in market.bets.iter_mut() {
            if existing_bet.selection.selection_id == selection.selection_id &&
               existing_bet.status == Status::Unmatched &&
               existing_bet.bet_type != *bet_type {
                if (*bet_type == BetType::Back && odds >= &existing_bet.odd) ||
                   (*bet_type == BetType::Lay && odds <= &existing_bet.odd) {
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

    fn calculate_win_amount(&self, bet_type: BetType, stake_amount: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => (stake_amount * (odds - BigUint::from(1000u32))) / BigUint::from(1000u32),
            BetType::Lay => (stake_amount * (odds - BigUint::from(1000u32))) / BigUint::from(1000u32),
        }
    }        

    
}