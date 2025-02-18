use crate::types::{Bet, BetMatchingState, BetStatus, BetType, BetView, MatchedPart, MatchingDetails, OrderbookView, PriceLevel, PriceLevelView};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait TrackerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
{
    fn process_bet(&self, mut bet: Bet<Self::Api>) -> (BigUint, BigUint) {
        let mut total_matched = bet.total_matched.clone();
        let mut remaining = &bet.stake_amount - &bet.total_matched;
    
        // Step 1: Initial state
        // let bet_type_value = match bet.bet_type {
        //     BetType::Back => 0u64,
        //     BetType::Lay => 1u64,
        // };
        // sc_panic!("{} {} {}", 
        //     bet_type_value,
        //     bet.stake_amount,
        //     bet.odd
        // );
    
        let mut opposite_levels = match bet.bet_type {
            BetType::Back => self.selection_lay_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_back_levels(bet.event, bet.selection.id).get(),
        };
    
        // Step 2: Opposite levels count
        // sc_panic!("{}", opposite_levels.len());
    
        let mut i = 0;
        while i < opposite_levels.len() && remaining > BigUint::zero() {
            let mut level = opposite_levels.get(i);
            
            // Step 3: Level details
            // sc_panic!("{} {} {} {}", 
            //     i,
            //     level.odds,
            //     level.total_stake,
            //     level.bet_nonces.len()
            // );
    
            if level.odds == bet.odd {
                let match_amount = remaining.clone().min(level.total_stake.clone());
                
                // Step 4: Match found
                // sc_panic!("{} {}", 
                //     match_amount,
                //     level.total_stake
                // );
    
                if match_amount > BigUint::zero() {
                    bet.matched_parts.push(MatchedPart {
                        amount: match_amount.clone(),
                        odds: level.odds.clone()
                    });
    
                    // Step 5: After push
                    // let new_total = total_matched.clone() + &match_amount;
                    // sc_panic!("{} {}", 
                    //     bet.matched_parts.len(),
                    //     new_total
                    // );
    
                    total_matched += &match_amount;
                    remaining -= &match_amount;
    
                    let mut updated_nonces = ManagedVec::new();
                    let mut total_level_stake = BigUint::zero();
    
                    for nonce in level.bet_nonces.iter() {
                        let mut matched_bet = self.bet_by_id(nonce).get();
                        let current_unmatched = &matched_bet.stake_amount - &matched_bet.total_matched;
                        
                        // Step 6: Processing opposite bet
                        // sc_panic!("{} {}", 
                        //     nonce,
                        //     current_unmatched
                        // );
                        
                        if current_unmatched > BigUint::zero() {
                            let match_this_bet = current_unmatched.clone().min(match_amount.clone());
                            
                            // Step 7.1: Before updating opposite bet
                            // let opp_matched_parts_count = matched_bet.matched_parts.len();
                            // sc_panic!("7.1: {} {}", 
                            //     opp_matched_parts_count,
                            //     matched_bet.potential_profit
                            // );
                                
                            if match_this_bet > BigUint::zero() {
                                matched_bet.matched_parts.push(MatchedPart {
                                    amount: match_this_bet.clone(),
                                    odds: matched_bet.odd.clone()
                                });
    
                                matched_bet.total_matched += &match_this_bet;
                                
                                // Update status
                                matched_bet.status = if &matched_bet.total_matched == &matched_bet.stake_amount {
                                    BetStatus::Matched
                                } else {
                                    BetStatus::PartiallyMatched
                                };
                                
                                matched_bet.potential_profit = self.calculate_total_potential_profit(&matched_bet);
    
                                // Step 7.2: After updating opposite bet
                                // let status_value = match matched_bet.status {
                                //     BetStatus::Matched => 0u64,
                                //     BetStatus::Unmatched => 1u64,
                                //     BetStatus::PartiallyMatched => 2u64,
                                //     BetStatus::Win => 3u64,
                                //     BetStatus::Lost => 4u64,
                                //     BetStatus::Canceled => 5u64,
                                //     BetStatus::Claimed => 6u64,
                                // };
                                // sc_panic!("7.2: {} {} {}", 
                                //     matched_bet.matched_parts.len(),
                                //     matched_bet.total_matched,
                                //     status_value
                                // );
                                
                                let remaining_unmatched = &matched_bet.stake_amount - &matched_bet.total_matched;
                                if remaining_unmatched > BigUint::zero() {
                                    updated_nonces.push(nonce);
                                    total_level_stake += remaining_unmatched;
                                }
                                
                                self.bet_by_id(nonce).set(&matched_bet);
    
                                // Step 7.3: Verify storage update
                                // let verified_bet = self.bet_by_id(nonce).get();
                                // let verified_status = match verified_bet.status {
                                //     BetStatus::Matched => 0u64,
                                //     BetStatus::Unmatched => 1u64,
                                //     BetStatus::PartiallyMatched => 2u64,
                                //     BetStatus::Win => 3u64,
                                //     BetStatus::Lost => 4u64,
                                //     BetStatus::Canceled => 5u64,
                                //     BetStatus::Claimed => 6u64,
                                // };
                                // sc_panic!("7.3: {} {} {}", 
                                //     verified_bet.matched_parts.len(),
                                //     verified_bet.total_matched,
                                //     verified_status
                                // );
                            }
                        }
                    }
    
                    if !updated_nonces.is_empty() {
                        level.bet_nonces = updated_nonces;
                        level.total_stake = total_level_stake;
                        let _ = opposite_levels.set(i, level);
                        i += 1;
                    } else {
                        // Eliminăm price level-ul dacă nu mai are pariuri active
                        if i < opposite_levels.len() - 1 {
                            let last = opposite_levels.get(opposite_levels.len() - 1);
                            let _ = opposite_levels.set(i, last);
                        }
                        opposite_levels.remove(opposite_levels.len() - 1);
                    }
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        
        match bet.bet_type {
            BetType::Back => self.selection_lay_levels(bet.event, bet.selection.id).set(&opposite_levels),
            BetType::Lay => self.selection_back_levels(bet.event, bet.selection.id).set(&opposite_levels),
        }
    
        bet.total_matched = total_matched.clone();
        
        bet.status = if remaining == BigUint::zero() {
            self.selection_matched_count(bet.event, bet.selection.id)
                .update(|val| *val += 1);
            BetStatus::Matched
        } else if total_matched > BigUint::zero() {
            self.selection_partially_matched_count(bet.event, bet.selection.id)
                .update(|val| *val += 1);
            BetStatus::PartiallyMatched
        } else {
            self.selection_unmatched_count(bet.event, bet.selection.id)
                .update(|val| *val += 1);
            BetStatus::Unmatched
        };
    
        bet.potential_profit = self.calculate_total_potential_profit(&bet);
        
        // Step 9: After profit calculation
        // let status_value = match bet.status {
        //     BetStatus::Matched => 0u64,
        //     BetStatus::Unmatched => 1u64,
        //     BetStatus::PartiallyMatched => 2u64,
        //     BetStatus::Win => 3u64,
        //     BetStatus::Lost => 4u64,
        //     BetStatus::Canceled => 5u64,
        //     BetStatus::Claimed => 6u64,
        // };
        // sc_panic!("9: {} {} {}", 
        //     bet.potential_profit,
        //     status_value,
        //     bet.matched_parts.len()
        // );
    
        if total_matched > BigUint::zero() {
            let new_matches = &total_matched - &bet.total_matched;
            self.update_total_matched(bet.event, bet.selection.id, &new_matches);
        }
        
        if remaining > BigUint::zero() {
            self.add_to_orderbook(&bet);
        }
    
        self.bet_by_id(bet.nft_nonce).set(&bet);
    
        // Step 10: Final state
        // let final_status_value = match bet.status {
        //     BetStatus::Matched => 0u64,
        //     BetStatus::Unmatched => 1u64,
        //     BetStatus::PartiallyMatched => 2u64,
        //     BetStatus::Win => 3u64,
        //     BetStatus::Lost => 4u64,
        //     BetStatus::Canceled => 5u64,
        //     BetStatus::Claimed => 6u64,
        // };
        // sc_panic!("10: {} {} {} {}", 
        //     bet.potential_profit,
        //     final_status_value,
        //     bet.matched_parts.len(),
        //     bet.total_matched
        // );
    
        (total_matched, remaining)
    }

    fn calculate_total_potential_profit(&self, bet: &Bet<Self::Api>) -> BigUint<Self::Api> {
        let mut total_profit = BigUint::zero();
        
        for matched_part in bet.matched_parts.iter() {
            match bet.bet_type {
                BetType::Back => {
                    let profit = (matched_part.odds.clone() - BigUint::from(100u32)) 
                        * &matched_part.amount / BigUint::from(100u32);
                    total_profit += profit;
                },
                BetType::Lay => {
                    let liability = (matched_part.odds.clone() - BigUint::from(100u32)) 
                        * &matched_part.amount / BigUint::from(100u32);
                    total_profit += liability;
                }
            }
        }
        
        total_profit
    }

    fn add_to_orderbook(&self, bet: &Bet<Self::Api>) {
        let unmatched_amount = &bet.stake_amount - &bet.total_matched;
        require!(unmatched_amount > BigUint::zero(), "No unmatched amount to add to orderbook");
        
        let mut levels = match bet.bet_type {
            BetType::Back => self.selection_back_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_lay_levels(bet.event, bet.selection.id).get(),
        };
    
        let mut level_index = Option::<usize>::None;
        
        for i in 0..levels.len() {
            let level = levels.get(i);
            if level.odds == bet.odd {
                level_index = Some(i);
                break;
            }
        }
    
        match level_index {
            Some(i) => {
                let mut level = levels.get(i);
                level.total_stake += &unmatched_amount;
                level.bet_nonces.push(bet.nft_nonce);
                let _ = levels.set(i, level);
            },
            None => {
                let new_level = PriceLevel {
                    odds: bet.odd.clone(),
                    total_stake: unmatched_amount.clone(),
                    bet_nonces: ManagedVec::from_single_item(bet.nft_nonce),
                };
    
                let mut insert_pos = levels.len();
                for i in 0..levels.len() {
                    let level = levels.get(i);
                    match bet.bet_type {
                        BetType::Back => {
                            if bet.odd > level.odds {
                                insert_pos = i;
                                break;
                            }
                        },
                        BetType::Lay => {
                            if bet.odd < level.odds {
                                insert_pos = i;
                                break;
                            }
                        },
                    }
                }
    
                if insert_pos == levels.len() {
                    levels.push(new_level);
                } else {
                    let mut temp_levels = ManagedVec::new();
                    for i in 0..insert_pos {
                        temp_levels.push(levels.get(i));
                    }
                    temp_levels.push(new_level);
                    for i in insert_pos..levels.len() {
                        temp_levels.push(levels.get(i));
                    }
                    levels = temp_levels;
                }
            }
        }
    
        match bet.bet_type {
            BetType::Back => {
                self.selection_back_levels(bet.event, bet.selection.id).set(&levels);
                self.selection_back_liquidity(bet.event, bet.selection.id)
                    .update(|val| *val = levels.iter().fold(BigUint::zero(), |acc, level| acc + &level.total_stake));
            },
            BetType::Lay => {
                self.selection_lay_levels(bet.event, bet.selection.id).set(&levels);
                self.selection_lay_liquidity(bet.event, bet.selection.id)
                    .update(|val| *val = levels.iter().fold(BigUint::zero(), |acc, level| acc + &level.total_stake));
            },
        }
    }

    fn remove_from_orderbook(&self, bet: &Bet<Self::Api>) {
        let unmatched_amount = &bet.stake_amount - &bet.total_matched;
        let mut levels = match bet.bet_type {
            BetType::Back => self.selection_back_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_lay_levels(bet.event, bet.selection.id).get(),
        };
    
        let mut level_index = Option::<usize>::None;
        
        for i in 0..levels.len() {
            let level = levels.get(i);
            if level.odds == bet.odd {
                level_index = Some(i);
                break;
            }
        }
    
        if let Some(i) = level_index {
            let mut level = levels.get(i);
            level.total_stake -= &unmatched_amount;
            
            let mut updated_nonces = ManagedVec::new();
            for nonce in level.bet_nonces.iter() {
                if nonce != bet.nft_nonce {
                    updated_nonces.push(nonce);
                }
            }
            
            if updated_nonces.is_empty() {
                if i < levels.len() - 1 {
                    let last = levels.get(levels.len() - 1);
                    let _ = levels.set(i, last);
                }
                levels.remove(levels.len() - 1);
            } else {
                level.bet_nonces = updated_nonces;
                let _ = levels.set(i, level);
            }
            
            match bet.bet_type {
                BetType::Back => {
                    self.selection_back_levels(bet.event, bet.selection.id).set(&levels);
                    self.selection_back_liquidity(bet.event, bet.selection.id)
                        .update(|val| *val -= &unmatched_amount);
                    
                    require!(
                        self.selection_back_liquidity(bet.event, bet.selection.id).get() >= BigUint::zero(),
                        "Invalid liquidity after removal"
                    );
                },
                BetType::Lay => {
                    self.selection_lay_levels(bet.event, bet.selection.id).set(&levels);
                    self.selection_lay_liquidity(bet.event, bet.selection.id)
                        .update(|val| *val -= &unmatched_amount);
                    
                    require!(
                        self.selection_lay_liquidity(bet.event, bet.selection.id).get() >= BigUint::zero(),
                        "Invalid liquidity after removal"
                    );
                },
            }
        }
    }

    fn update_total_matched(
        &self,
        market_id: u64,
        selection_id: u64,
        matched_amount: &BigUint
    ) {
        self.total_matched_amount(market_id, selection_id)
            .update(|total| {
                *total += matched_amount;
                require!(
                    *total <= self.selection_back_liquidity(market_id, selection_id).get(),
                    "Invalid matched amount"
                );
            });
    }

    #[view(getBetMatchingState)]
    fn get_bet_matching_state(
        &self,
        bet_nonce: u64
    ) -> BetMatchingState<Self::Api> {
        let bet = self.bet_by_id(bet_nonce).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
        
        BetMatchingState {
            bet_type: bet.bet_type,
            original_stake: bet.stake_amount,
            matched_amount: bet.total_matched,
            unmatched_amount: unmatched,
            status: bet.status,
            odds: bet.odd
        }
    }

    #[view(getBetDetails)]
    fn get_bet_details(
        &self,
        bet_nonce: u64
    ) -> BetView<Self::Api> {
        let bet = self.bet_by_id(bet_nonce).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
        
        BetView {
            nonce: bet.nft_nonce,
            bettor: bet.bettor,
            stake: bet.stake_amount,
            matched: bet.total_matched,
            unmatched: unmatched,
            status: bet.status
        }
    }

    #[view(getMatchingStats)]
    fn get_matching_stats(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> (u32, BigUint<Self::Api>) {
        let matched_count = self.selection_matched_count(market_id, selection_id).get() as u32;
        let total_matched = self.total_matched_amount(market_id, selection_id).get();
        (matched_count, total_matched)
    }
}