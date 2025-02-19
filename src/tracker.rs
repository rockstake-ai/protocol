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
        
        let mut remaining_liability = match bet.bet_type {
            BetType::Back => BigUint::zero(),
            BetType::Lay => {
                let odds_minus_one = &bet.odd - &BigUint::from(100u64);
                (&remaining * &odds_minus_one) / &BigUint::from(100u64)
            }
        };

        let bet_type_value = match bet.bet_type {
            BetType::Back => 0u64,
            BetType::Lay => 1u64,
        };
    
        let mut opposite_levels = match bet.bet_type {
            BetType::Back => self.selection_lay_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_back_levels(bet.event, bet.selection.id).get(),
        };
    
        let mut i = 0;
        while i < opposite_levels.len() && remaining > BigUint::zero() {
            let mut level = opposite_levels.get(i);
    
            if level.odds == bet.odd {
                let match_amount = remaining.clone().min(level.total_stake.clone());
    
                if match_amount > BigUint::zero() {
                    if bet.bet_type == BetType::Lay {
                        let matched_liability = (&match_amount * &(&bet.odd - &BigUint::from(100u64))) 
                            / &BigUint::from(100u64);
                        remaining_liability -= &matched_liability;
                    }

                    bet.matched_parts.push(MatchedPart {
                        amount: match_amount.clone(),
                        odds: level.odds.clone(),
                    });
    
                    total_matched += &match_amount;
                    remaining -= &match_amount;
    
                    let mut updated_nonces = ManagedVec::new();
                    let mut total_level_stake = BigUint::zero();
    
                    for nonce in level.bet_nonces.iter() {
                        let mut matched_bet = self.bet_by_id(nonce).get();
                        let current_unmatched = &matched_bet.stake_amount - &matched_bet.total_matched;
    
                        if current_unmatched > BigUint::zero() {
                            let match_this_bet = current_unmatched.clone().min(match_amount.clone());
    
                            if match_this_bet > BigUint::zero() {
                                matched_bet.matched_parts.push(MatchedPart {
                                    amount: match_this_bet.clone(),
                                    odds: matched_bet.odd.clone(),
                                });
    
                                matched_bet.total_matched += &match_this_bet;

                                if matched_bet.bet_type == BetType::Lay {
                                    let matched_ratio = &match_this_bet / &matched_bet.stake_amount;
                                    let matched_total = &matched_bet.total_amount * &matched_ratio;
                                    matched_bet.total_amount -= &matched_total;
                                }
    
                                matched_bet.status = if &matched_bet.total_matched == &matched_bet.stake_amount {
                                    BetStatus::Matched
                                } else {
                                    BetStatus::PartiallyMatched
                                };
    
                                matched_bet.potential_profit = self.calculate_total_potential_profit(&matched_bet);
    
                                let remaining_unmatched = &matched_bet.stake_amount - &matched_bet.total_matched;
                                if remaining_unmatched > BigUint::zero() {
                                    updated_nonces.push(nonce);
                                    total_level_stake += remaining_unmatched;
                                }
    
                                self.bet_by_id(nonce).set(&matched_bet);
                            }
                        }
                    }
    
                    if !updated_nonces.is_empty() {
                        level.bet_nonces = updated_nonces;
                        level.total_stake = total_level_stake;
                        let _ = opposite_levels.set(i, level);
                        i += 1;
                    } else {
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
            bet.stake_amount = total_matched.clone();
            bet.total_amount = if bet.bet_type == BetType::Lay {
                let matched_ratio = &total_matched / &bet.stake_amount;
                &bet.total_amount * &matched_ratio
            } else {
                total_matched.clone()
            };
            
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
        bet.liability = remaining_liability; // Actualizăm liability-ul rămas
    
        if total_matched > BigUint::zero() {
            let new_matches = &total_matched - &bet.total_matched;
            self.update_total_matched(bet.event, bet.selection.id, &new_matches);
        }
    
        if remaining > BigUint::zero() {
            self.add_to_orderbook(&bet);
        }
    
        self.bet_by_id(bet.nft_nonce).set(&bet);
    
        (total_matched, remaining)
    }

    fn calculate_total_potential_profit(&self, bet: &Bet<Self::Api>) -> BigUint {
        match bet.bet_type {
            BetType::Back => {
                // Pentru Back: (matched_amount * (odds-1))
                &bet.total_matched * &(&bet.odd - &BigUint::from(100u64)) / &BigUint::from(100u64)
            },
            BetType::Lay => {
                // Pentru Lay: matched_amount (profitul este stake-ul)
                bet.total_matched.clone()
            }
        }
    }

    fn add_to_orderbook(&self, bet: &Bet<Self::Api>) {
        let unmatched_amount = &bet.stake_amount - &bet.total_matched;
        if unmatched_amount == BigUint::zero() {
            return; 
        }        
        
        let unmatched_liability = if bet.bet_type == BetType::Lay {
            let odds_minus_one = &bet.odd - &BigUint::from(100u64);
            (&unmatched_amount * &odds_minus_one) / &BigUint::from(100u64)
        } else {
            BigUint::zero()
        };

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
                        "Invalid back liquidity after removal"
                    );
                },
                BetType::Lay => {
                    self.selection_lay_levels(bet.event, bet.selection.id).set(&levels);
                    self.selection_lay_liquidity(bet.event, bet.selection.id)
                        .update(|val| *val -= &unmatched_amount);
                    
                    require!(
                        self.selection_lay_liquidity(bet.event, bet.selection.id).get() >= BigUint::zero(),
                        "Invalid lay liquidity after removal"
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