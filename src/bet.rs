use crate::{errors::{ERR_BET_CANNOT_BE_CANCELLED, ERR_INVALID_LIABILITY, ERR_INVALID_STAKE, ERR_NOT_BET_OWNER, ERR_ODDS_TOO_LOW}, types::{Bet, BetStatus, BetType, Sport}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetModule: 
    crate::storage::StorageModule +
    crate::events::EventsModule +
    crate::nft::NftModule +
    crate::orderbook::OrderbookModule +
    crate::validation::ValidationModule +
    crate::utils::UtilsModule
{
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Bet Placement ---------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Places a bet on a specified market and selection.
    /// Parameters:
    /// - sport: The type of sport for the bet (e.g., Football, Basketball).
    /// - market_id: The ID of the market the bet is placed on.
    /// - selection_id: The ID of the selection within the market.
    /// - odds: The odds at which the bet is placed (in BigUint format).
    /// - bet_type: The type of bet (Back or Lay).
    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(
        &self,
        sport: Sport,
        market_id: u64,
        selection_id: u64,
        odds: BigUint,
        bet_type: BetType,
    ) {
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, total_amount) = self
            .call_value()
            .egld_or_single_esdt()
            .into_tuple();
        
        self.validate_bet_amount(&total_amount);
        self.validate_bet_odds(&odds);
        self.validate_market(market_id);
        self.validate_selection(market_id, selection_id);
        
        let (final_stake, final_liability) = self.calculate_stake_and_liability(
            &bet_type,
            &total_amount,
            &odds
        );
        
        let bet_hash = self.generate_unique_bet_hash(
            &caller,
            &sport,
            &market_id,
            &selection_id,
            &odds,
            &bet_type,
            &token_identifier,
            token_nonce,
            &total_amount
        );
        
        let bet_id = self.get_bet_id_hash(&bet_hash);
        
        let bet = self.create_bet(
            sport,
            market_id,
            selection_id,
            &caller,
            &final_stake,
            &final_liability,
            &total_amount,
            &odds,
            bet_type,
            token_identifier.clone(),
            token_nonce,
            bet_id
        );
        
        let (updated_bet, matched_amount, remaining) = self.process_bet(bet);
        self.bet_by_id(bet_id).set(&updated_bet);
        let final_bet = self.update_bet_status(updated_bet, matched_amount.clone(), remaining.clone());
        self.update_market_and_selection(
            market_id,
            selection_id,
            &matched_amount
        );
        
        let real_nft_nonce = self.mint_bet_nft(&final_bet);
        
        let mut final_bet_with_nonce = final_bet.clone();
        final_bet_with_nonce.nft_nonce = real_nft_nonce;
        self.bet_by_id(bet_id).set(&final_bet_with_nonce);
        
        self.bet_nonce_to_id(real_nft_nonce).set(bet_id);
        
        self.market_bet_ids(market_id).insert(bet_id);
        
        let amount_to_lock = match bet_type {
            BetType::Back => remaining.clone(),
            BetType::Lay => {
                let ratio = &remaining.clone() * &BigUint::from(100u64) / &final_bet.stake_amount;
                (&final_bet.total_amount * &ratio) / &BigUint::from(100u64)
            }
        };
    
        if amount_to_lock > BigUint::zero() {
            self.locked_funds(&caller).update(|funds| *funds += &amount_to_lock);
        }
    
        self.send().direct_esdt(
            &caller,
            self.bet_nft_token().get_token_id_ref(),
            real_nft_nonce,
            &BigUint::from(1u64)
        );
        
        self.emit_bet_placed_event(
            &final_bet_with_nonce,
            &token_identifier,
            token_nonce,
            &matched_amount,
            &remaining.clone(),  
            real_nft_nonce,
            bet_id
        );
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Bet Cancellation ------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Cancels a bet if the caller is the bettor and the bet is cancellable.
    /// Parameters:
    /// - bet_id: The ID of the bet to be canceled.
    #[payable("*")]
#[endpoint(cancelBet)]
fn cancel_bet(&self, bet_id: u64) { 
    let caller = self.blockchain().get_caller();
    
    // Verifică dacă bet-ul există
    require!(!self.bet_by_id(bet_id).is_empty(), "Bet does not exist");
    
    let mut bet = self.bet_by_id(bet_id).get();

    let (token_identifier, payment_nonce, amount) = self
        .call_value()
        .egld_or_single_esdt()
        .into_tuple();
    let token_identifier_wrap = token_identifier.unwrap_esdt();

    require!(
        token_identifier_wrap == self.bet_nft_token().get_token_id(),
        "Must send the bet NFT to cancel"
    );
    require!(
        payment_nonce == bet.nft_nonce,
        "Invalid NFT nonce"
    );
    require!(
        amount == BigUint::from(1u64),
        "Must send exactly 1 NFT"
    );

    require!(bet.bettor == caller, "Only the bet owner can cancel the bet");
    require!(
        bet.status == BetStatus::Unmatched || bet.status == BetStatus::PartiallyMatched,
        "Bet cannot be cancelled in this state"
    );

    let unmatched = &bet.stake_amount - &bet.total_matched;
    let refund_amount = match bet.bet_type {
        BetType::Back => {
            if bet.status == BetStatus::Unmatched {
                bet.stake_amount.clone()
            } else {
                unmatched.clone()
            }
        },
        BetType::Lay => {
            let unmatched_ratio = (&unmatched * &BigUint::from(100u64)) / &bet.stake_amount;
            let refund = &bet.total_amount * &unmatched_ratio / &BigUint::from(100u64);
            if bet.status == BetStatus::Unmatched {
                bet.total_amount.clone()
            } else {
                refund
            }
        }
    };

    let locked_funds = self.locked_funds(&caller).get();
    require!(locked_funds >= refund_amount, "Insufficient locked funds to refund");

    let status_for_event: u8 = match &bet.status {
        BetStatus::Unmatched => {
            self.selection_unmatched_count(bet.event, bet.selection.id)
                .update(|val| *val -= 1);

            self.send().esdt_local_burn(
                &token_identifier_wrap,
                payment_nonce,
                &BigUint::from(1u64)
            );

            // Utilizează noua funcție de ștergere
            self.delete_bet(bet_id);
            
            BetStatus::Unmatched as u8 
        },
        BetStatus::PartiallyMatched => {
            self.selection_partially_matched_count(bet.event, bet.selection.id)
                .update(|val| *val -= 1);
            self.selection_matched_count(bet.event, bet.selection.id)
                .update(|val| *val += 1);

            bet.status = BetStatus::Matched;

            let original_stake_amount = bet.stake_amount.clone();
            bet.stake_amount = bet.total_matched.clone();
            let matched_liability = if bet.bet_type == BetType::Lay {
                (&bet.liability * &bet.total_matched) / &original_stake_amount
            } else {
                BigUint::zero()
            };
            if bet.total_matched > BigUint::zero() {
                bet.total_amount = match bet.bet_type {
                    BetType::Back => bet.total_matched.clone(),
                    BetType::Lay => &bet.total_matched + &matched_liability,
                };
            } else {
                bet.total_amount = BigUint::zero();
            }
            bet.liability = matched_liability.clone();
            bet.potential_profit = self.calculate_total_potential_profit(&bet);

            self.send().direct_esdt(
                &caller,
                &token_identifier_wrap,
                bet.nft_nonce, 
                &BigUint::from(1u64)
            );

            self.bet_by_id(bet_id).set(&bet);
            BetStatus::Matched as u8 
        },
        _ => sc_panic!("Invalid bet status for cancellation"),
    };

    self.locked_funds(&caller).update(|val| {
        if *val >= refund_amount {
            *val -= &refund_amount;
        } else {
            *val = BigUint::zero();
        }
    });
    self.send().direct(&caller, &bet.payment_token, 0, &refund_amount);

    let sport_index = match bet.sport {
        Sport::Football => 1u8,
        Sport::Basketball => 2u8,
        Sport::CounterStrike => 3u8,
        Sport::Dota => 4u8,
        Sport::LeagueOfLegends => 5u8,
    };

    self.cancel_bet_event(
        &caller,
        bet_id,
        status_for_event,
        &refund_amount,
        &bet.total_matched,
        &bet.total_amount,
        &bet.potential_profit,
        &bet.liability,
        sport_index,
        bet.nft_nonce
    );
}
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Helper Functions ------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Calculates the potential profit for a matched bet.
    /// Parameters:
    /// - bet: The bet object containing matched parts and bet type.
    /// Returns: The total potential profit as BigUint.
    fn calculate_matched_potential_profit(&self, bet: &Bet<Self::Api>) -> BigUint {
        let mut total_potential_profit = BigUint::zero();
        
        for part in bet.matched_parts.iter() {
            let potential_profit = match bet.bet_type {
                BetType::Back => {
                    &part.amount * &part.odds / BigUint::from(100u64) - &part.amount
                },
                BetType::Lay => {
                    part.amount.clone()
                }
            };
            total_potential_profit += potential_profit;
        }
        
        total_potential_profit
    }

    /// Creates a new bet object with the provided details.
    /// Parameters:
    /// - sport: The sport type.
    /// - market_id: The market ID.
    /// - selection_id: The selection ID.
    /// - caller: The address of the bettor.
    /// - stake: The stake amount.
    /// - liability: The liability amount.
    /// - total_amount: The total amount paid.
    /// - odds: The odds of the bet.
    /// - bet_type: The type of bet (Back or Lay).
    /// - token_identifier: The token used for payment.
    /// - token_nonce: The nonce of the token.
    /// - bet_id: The unique ID of the bet.
    /// Returns: A new Bet object.
    fn create_bet(
        &self,
        sport: Sport,
        market_id: u64,
        selection_id: u64,
        caller: &ManagedAddress<Self::Api>,
        stake: &BigUint,
        liability: &BigUint,
        total_amount: &BigUint,
        odds: &BigUint,
        bet_type: BetType,
        token_identifier: EgldOrEsdtTokenIdentifier<Self::Api>,
        token_nonce: u64,
        bet_id: u64, 
    ) -> Bet<Self::Api> {
        let market = self.markets(market_id).get();
        let selection = market.selections
            .iter()
            .find(|s| s.id == selection_id)
            .unwrap_or_else(|| sc_panic!(crate::errors::ERR_INVALID_SELECTION))
            .clone();
        
        Bet {
            bet_id,
            bettor: caller.clone(),
            sport,
            event: market_id,
            selection,
            stake_amount: stake.clone(),
            liability: liability.clone(),
            total_amount: total_amount.clone(), 
            total_matched: BigUint::zero(),
            matched_parts: ManagedVec::new(),
            potential_profit: BigUint::zero(),
            odd: odds.clone(),
            bet_type,
            status: BetStatus::Unmatched,
            payment_token: token_identifier,
            payment_nonce: token_nonce,
            nft_nonce: self.blockchain().get_current_esdt_nft_nonce(
                &self.blockchain().get_sc_address(),
                self.bet_nft_token().get_token_id_ref(),
            ),
            created_at: self.blockchain().get_block_timestamp()
        }
    }

    /// Updates the status of a bet based on matched and remaining amounts.
    /// Parameters:
    /// - bet: The bet object to update.
    /// - matched_amount: The amount that has been matched.
    /// - remaining: The remaining unmatched amount.
    /// Returns: The updated Bet object.
    fn update_bet_status(
        &self,
        mut bet: Bet<Self::Api>,
        matched_amount: BigUint,
        remaining: BigUint
    ) -> Bet<Self::Api> {
        bet.total_matched = matched_amount.clone();
        bet.status = if matched_amount > BigUint::zero() {
            if remaining > BigUint::zero() {
                BetStatus::PartiallyMatched
            } else {
                BetStatus::Matched
            }
        } else {
            BetStatus::Unmatched
        };
        bet
    }

    /// Updates the market and selection data after a bet is placed.
    /// Parameters:
    /// - market_id: The ID of the market.
    /// - selection_id: The ID of the selection.
    /// - matched_amount: The amount matched in the bet.
    fn update_market_and_selection(
        &self,
        market_id: u64,
        selection_id: u64,
        matched_amount: &BigUint,
    ) {
        let mut market = self.markets(market_id).get();
        let selection_index = market
            .selections
            .iter()
            .position(|s| s.id == selection_id)
            .unwrap_or_else(|| sc_panic!(crate::errors::ERR_INVALID_SELECTION));
        
        let mut selection = market.selections.get(selection_index);
        selection.priority_queue = self.selection_tracker(market_id, selection_id).get();
        
        let _ = market.selections.set(selection_index, selection);
        market.total_matched_amount += matched_amount;
        self.markets(market_id).set(&market);
    }

    /// Emits an event when a bet is placed.
    /// Parameters:
    /// - bet: The bet object.
    /// - token_identifier: The token used for payment.
    /// - token_nonce: The nonce of the token.
    /// - matched_amount: The matched amount.
    /// - unmatched_amount: The remaining unmatched amount.
    /// - bet_id: The ID of the bet.
    fn emit_bet_placed_event(
        &self,
        bet: &Bet<Self::Api>,
        token_identifier: &EgldOrEsdtTokenIdentifier<Self::Api>,
        _token_nonce: u64,
        _matched_amount: &BigUint,
        _unmatched_amount: &BigUint,
        nft_nonce: u64, 
        bet_id: u64,
    ) {
        let sport_index = match bet.sport {
            Sport::Football => 1u8,
            Sport::Basketball => 2u8,
            Sport::CounterStrike => 3u8,
            Sport::Dota => 4u8,
            Sport::LeagueOfLegends => 5u8,
        };
        
        let user_hex = bet.bettor.hex_expr();
        
        let bet_type_value = match bet.bet_type {
            BetType::Back => 1u8,
            BetType::Lay => 2u8,
        };
        
        let status_value = match bet.status {
            BetStatus::Unmatched => 1u8,
            BetStatus::Matched => 2u8,
            BetStatus::PartiallyMatched => 3u8,
            BetStatus::Win => 4u8,
            BetStatus::Lost => 5u8,
            BetStatus::Claimed => 6u8,
        };
        
        let potential_profit = match bet.bet_type {
            BetType::Back => &bet.stake_amount * &(&bet.odd - &BigUint::from(100u64)) / &BigUint::from(100u64),
            BetType::Lay => bet.stake_amount.clone(),
        };
        
        self.place_bet_event(
            &user_hex,
            sport_index,
            &bet.event,
            &bet.selection.id,
            &bet.stake_amount,
            &bet.odd,
            bet_type_value,
            token_identifier,
            &bet.liability,
            status_value,
            &bet.matched_parts, 
            bet_id,
            &potential_profit,
            nft_nonce, 
        );
    }

    /// Calculates the stake and liability for a bet based on its type.
    /// Parameters:
    /// - bet_type: The type of bet (Back or Lay).
    /// - total_amount: The total amount paid.
    /// - odds: The odds of the bet.
    /// Returns: A tuple of (stake, liability) as BigUint.
    fn calculate_stake_and_liability(
        &self,
        bet_type: &BetType,
        total_amount: &BigUint, 
        odds: &BigUint
    ) -> (BigUint, BigUint) {
        match bet_type {
            BetType::Back => {
                (total_amount.clone(), BigUint::zero())
            },
            BetType::Lay => {
                let odds_minus_one = odds - &BigUint::from(100u64); 
                require!(odds_minus_one > 0, ERR_ODDS_TOO_LOW);
                
                let stake = (total_amount * &BigUint::from(100u64)) / odds;
                require!(stake > 0, ERR_INVALID_STAKE);
                
                let liability = total_amount - &stake;
                require!(liability >= BigUint::zero(), ERR_INVALID_LIABILITY);
                
                (stake, liability) 
            }
        }
    }

    #[view(betExists)]
    fn bet_exists(&self, bet_id: u64) -> bool {
        self.bet_by_id(bet_id).is_empty() == false
    }
}