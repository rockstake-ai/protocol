use crate::{storage::{self, Bet, BetType, Market, Selection, Status}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: storage::StorageModule 
    + crate::events::EventsModule 
    + crate::nft_manager::NftManagerModule {

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

        // Verificăm dacă odds-ul este în intervalul corect (între 1.01 și 1000.00)
        require!(odds >= BigUint::from(101u32) && odds <= BigUint::from(100000u32), "Odds must be between 1.01 and 1000.00");

        let selection_index = market.selections.iter()
            .position(|s| &s.selection_id == &selection_id)
            .expect("Selection not found in this market");
        let mut selection = market.selections.get(selection_index);

        let best_lay_odds = &selection.best_lay_odds;
        let best_back_odds = &selection.best_back_odds;

        match bet_type {
            BetType::Back => {
                if best_lay_odds == &BigUint::zero() || &odds <= best_lay_odds {
                    if best_back_odds == &BigUint::zero() || &odds > best_back_odds {
                        selection.best_back_odds = odds.clone();
                    }
                } else {
                    return sc_error!("Back odds must be less than or equal to the best Lay odds");
                }
            },
            BetType::Lay => {
                if best_back_odds == &BigUint::zero() || &odds >= best_back_odds {
                    if best_lay_odds == &BigUint::zero() || &odds > best_lay_odds {
                        selection.best_lay_odds = odds.clone();
                    }
                    // Nu adăugăm toată suma la lichiditate aici
                } else {
                    return sc_error!("Lay odds must be greater than or equal to the best Back odds");
                }
            }
        }
        
        // Folosim o referință la bet_type pentru a o putea utiliza în multiple locuri
        let (initial_status, matched_amount, unmatched_amount) = self.matching_bet(&mut market, &mut selection, &bet_type, &odds, &token_amount); 
        
        match bet_type {
            BetType::Back => {
                selection.back_liquidity += &unmatched_amount;
                market.back_liquidity += &unmatched_amount;
            },
            BetType::Lay => {
                let lay_liquidity = self.calculate_win_amount(&BetType::Lay, &unmatched_amount, &odds);
                selection.lay_liquidity += &lay_liquidity;
                market.lay_liquidity += &lay_liquidity;
            }
        }        
        let win_amount = self.calculate_win_amount(&bet_type, &token_amount, &odds);
        
        let bet = Bet {
            bettor: caller.clone(),
            event: market_id.clone(),
            selection: selection.clone(),
            stake_amount: token_amount.clone(),
            matched_amount: matched_amount.clone(),
            unmatched_amount: unmatched_amount.clone(),
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
    
        if unmatched_amount > BigUint::zero() {
            self.locked_funds(&caller).update(|current_locked| *current_locked += &unmatched_amount);
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
            &unmatched_amount
        );
    
        Ok((bet_id, odds, token_amount))
    }

    fn bet_type_to_number(&self, bet_type: &BetType) -> u8 {
        match bet_type {
            BetType::Back => 0,
            BetType::Lay => 1,
        }
    }

    fn status_to_number(&self, status: &Status) -> u8 {
        match status {
            Status::Unmatched => 0,
            Status::PartiallyMatched => 1,
            Status::Matched => 2,
            Status::Canceled => 3,
            Status::Win => 4,
            Status::Lost => 5,
            // Adaugă alte variante dacă există
        }
    }

    fn matching_bet(
        &self,
        market: &mut Market<Self::Api>,
        selection: &mut Selection<Self::Api>,
        bet_type: &BetType,
        odds: &BigUint,
        amount: &BigUint
    ) -> (Status, BigUint, BigUint) {
        
        let mut matched_amount = BigUint::zero();
        let mut unmatched_amount = amount.clone();
    
        for i in 0..market.bets.len() {
            let mut existing_bet = market.bets.get(i);

            // sc_panic!(
            //     "Checking bet: existing_bet_type={}, existing_odds={}, existing_amount={}, existing_matched_amount={}, existing_status={}",
            //     self.bet_type_to_number(&existing_bet.bet_type),
            //     existing_bet.odd,
            //     existing_bet.stake_amount,
            //     existing_bet.matched_amount,
            //     self.status_to_number(&existing_bet.status),
            // );

            if existing_bet.selection.selection_id == selection.selection_id &&
               (existing_bet.status == Status::Unmatched || existing_bet.status == Status::PartiallyMatched) &&
               existing_bet.bet_type != *bet_type {

                if (bet_type == &BetType::Back && odds <= &existing_bet.odd) ||
                   (bet_type == &BetType::Lay && odds >= &existing_bet.odd) {
                    let existing_unmatched = &existing_bet.stake_amount - &existing_bet.matched_amount;
                    let mut match_amount = unmatched_amount.clone().min(existing_unmatched.clone());

                    // sc_panic!("Potential match found: match_amount={}, existing_unmatched={}",
                    //     match_amount,
                    //     existing_unmatched
                    // );
    
                    // Calculăm suma reală care poate fi potrivită bazată pe lichiditate
                    match bet_type {
                        BetType::Back => {
                            let potential_win = self.calculate_win_amount(bet_type, &match_amount, odds);
                            if potential_win > selection.lay_liquidity {
                                match_amount = self.calculate_stake_from_win(&selection.lay_liquidity, odds);
                            }
                        },
                        BetType::Lay => {
                            let potential_liability = self.calculate_win_amount(bet_type, &match_amount, odds);
                            if potential_liability > selection.back_liquidity {
                                match_amount = selection.back_liquidity.clone();
                            }
                        }
                    }
    
                    matched_amount += &match_amount;
                    unmatched_amount -= &match_amount;
                    existing_bet.matched_amount += &match_amount;

                    // sc_panic!(
                    //     "After liquidity check: adjusted_match_amount={}, new_matched_amount={}, new_unmatched_amount={}",
                    //     match_amount,
                    //     matched_amount + &match_amount,
                    //     unmatched_amount - &match_amount
                    // );
    
                    // Actualizăm lichiditățile
                    match bet_type {
                        BetType::Back => {
                            let win_amount = self.calculate_win_amount(bet_type, &match_amount, odds);
                            selection.lay_liquidity -= &win_amount;
                            market.lay_liquidity -= &win_amount;
                        },
                        BetType::Lay => {
                            selection.back_liquidity -= &match_amount;
                            market.back_liquidity -= &match_amount;
                        }
                    }

                    // Actualizăm statusul pariului existent
                    if existing_bet.matched_amount == existing_bet.stake_amount {
                        existing_bet.status = Status::Matched;
                    } else {
                        existing_bet.status = Status::PartiallyMatched;
                    }
    
                    let _ = market.bets.set(i, &existing_bet);
    
                    if unmatched_amount == BigUint::zero() {
                        break;
                    }
                }
            }
        }
    
        let status = if matched_amount == *amount {
            Status::Matched
        } else if matched_amount > BigUint::zero() {
            Status::PartiallyMatched
        } else {
            Status::Unmatched
        };


       
            sc_panic!(
                "Matching complete: status={}, matched_amount={}, unmatched_amount={}, selection_back_liquidity={}, selection_lay_liquidity={}",
                self.status_to_number(&status),
            matched_amount,
    unmatched_amount,
    selection.back_liquidity,
    selection.lay_liquidity
            );
    
        (status, matched_amount, unmatched_amount)
    }
    
    // Funcție auxiliară pentru a calcula miza în funcție de câștigul potențial
    fn calculate_stake_from_win(&self, win_amount: &BigUint, odds: &BigUint) -> BigUint {
        (win_amount * &BigUint::from(100u32)) / (odds - &BigUint::from(100u32))
    }
    

    #[endpoint(closeBet)]
    fn close_bet(&self, bet_id: u64) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let mut bet = self.bet_by_id(bet_id).get();

        require!(bet.bettor == caller, "Only the bettor can close the bet");
        require!(
            bet.status == Status::Matched || bet.status == Status::PartiallyMatched,
            "Bet cannot be closed in its current state"
        );

        let mut market = self.markets(&bet.event).get();
        let selection_index = market.selections.iter()
            .position(|s| s.selection_id == bet.selection.selection_id)
            .expect("Selection not found in this market");
        let mut selection = market.selections.get(selection_index);

        let refund_amount = bet.unmatched_amount.clone();

        match bet.bet_type {
            BetType::Back => {
                require!(
                    selection.back_liquidity >= refund_amount,
                    "Insufficient back liquidity"
                );
                selection.back_liquidity -= &refund_amount;
                market.back_liquidity -= &refund_amount;
            },
            BetType::Lay => {
                let lay_liquidity_reduction = self.calculate_win_amount(&BetType::Lay, &refund_amount, &bet.odd);
                require!(
                    selection.lay_liquidity >= lay_liquidity_reduction,
                    "Insufficient lay liquidity"
                );
                selection.lay_liquidity -= &lay_liquidity_reduction;
                market.lay_liquidity -= &lay_liquidity_reduction;
            }
        }

        if bet.matched_amount == BigUint::zero() {
            bet.status = Status::Canceled;
        } else {
            bet.status = Status::Matched;
        }

        bet.stake_amount = bet.matched_amount.clone();
        bet.unmatched_amount = BigUint::zero();

        self.bet_by_id(bet_id).set(&bet);
        let _ = market.selections.set(selection_index, &selection);
        self.markets(&bet.event).set(&market);

        if refund_amount > BigUint::zero() {
            self.send().direct(
                &caller,
                &bet.payment_token,
                bet.payment_nonce,
                &refund_amount
            );
        }

        self.bet_closed_event(
            &caller,
            &bet_id,
            &bet.event,
            &bet.selection.selection_id,
            &refund_amount,
            &bet.payment_token,
            bet.payment_nonce
        );

        Ok(())
    }


    fn calculate_win_amount(&self, bet_type: &BetType, stake_amount: &BigUint, odds: &BigUint) -> BigUint {
        // Ajustăm limitele pentru a reprezenta odds cu două zecimale
        let min_odds = BigUint::from(101u32); // 1.01 * 100
        let max_odds = BigUint::from(100000u32); // 1000.00 * 100
    
        require!(
            odds >= &min_odds && odds <= &max_odds,
            "Odds must be between 1.01 and 1000.00"
        );
    
        match bet_type {
            BetType::Back => {
                // Formula: stake_amount * (odds - 100) / 100
                stake_amount * &(odds - &BigUint::from(100u32)) / BigUint::from(100u32)
            },
            BetType::Lay => {
                // Formula: stake_amount * 100 / (odds - 100)
                (stake_amount * &BigUint::from(100u32)) / (odds - &BigUint::from(100u32))
            },
        }
    }
           
}