use crate::types::{Bet, BetMatchingState, BetStatus, BetType, BetView, MatchingDetails, OrderbookView, PriceLevel, PriceLevelView};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait TrackerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
{

    fn process_bet(&self, mut bet: Bet<Self::Api>) -> (BigUint, BigUint) {
        let mut matched_amount = bet.matched_amount.clone();
        let mut remaining = bet.unmatched_amount.clone();
    
        // Încercăm să facem match cu pariurile opuse
        let mut opposite_levels = match bet.bet_type {
            BetType::Back => self.selection_lay_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_back_levels(bet.event, bet.selection.id).get(),
        };
    
        let mut i = 0;
        while i < opposite_levels.len() && remaining > BigUint::zero() {
            let mut level = opposite_levels.get(i);
            
            if level.odds == bet.odd {
                let match_amount = match bet.bet_type {
                    BetType::Back => {
                        remaining.clone().min(level.total_stake.clone())
                    },
                    BetType::Lay => {
                        bet.liability.clone().min(level.total_stake.clone())
                    }
                };
                
                if match_amount > BigUint::zero() {
                    matched_amount += &match_amount;
                    remaining -= &match_amount;
                    level.total_stake -= &match_amount;
    
                    // Update matched bets
                    let mut updated_nonces = ManagedVec::new();
                    for nonce in level.bet_nonces.iter() {
                        let mut matched_bet = self.bet_by_id(nonce).get();
                        if matched_bet.unmatched_amount > BigUint::zero() {
                            let match_this_bet = matched_bet.unmatched_amount.clone().min(match_amount.clone());
                            
                            if match_this_bet > BigUint::zero() {
                                matched_bet.matched_amount += &match_this_bet;
                                matched_bet.unmatched_amount -= &match_this_bet;
                                
                                matched_bet.status = if matched_bet.unmatched_amount == BigUint::zero() {
                                    BetStatus::Matched
                                } else {
                                    BetStatus::PartiallyMatched
                                };
                                
                                self.bet_by_id(nonce).set(&matched_bet);
    
                                if matched_bet.unmatched_amount > BigUint::zero() {
                                    updated_nonces.push(nonce);
                                }
                            }
                        }
                    }
    
                    if !updated_nonces.is_empty() {
                        level.bet_nonces = updated_nonces;
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
        
        // Salvăm levels-urile opuse actualizate
        match bet.bet_type {
            BetType::Back => self.selection_lay_levels(bet.event, bet.selection.id).set(&opposite_levels),
            BetType::Lay => self.selection_back_levels(bet.event, bet.selection.id).set(&opposite_levels),
        }
    
        // Update bet state
        bet.matched_amount = matched_amount.clone();
        bet.unmatched_amount = remaining.clone();
        
        bet.status = if remaining == BigUint::zero() {
            self.selection_matched_count(bet.event, bet.selection.id)
                .update(|val| *val += 1);
            BetStatus::Matched
        } else if matched_amount > bet.matched_amount {
            self.selection_partially_matched_count(bet.event, bet.selection.id)
                .update(|val| *val += 1);
            BetStatus::PartiallyMatched
        } else {
            self.selection_unmatched_count(bet.event, bet.selection.id)
                .update(|val| *val += 1);
            BetStatus::Unmatched
        };
    
        // Actualizăm totalul matched doar pentru noile matchuri
        if matched_amount > bet.matched_amount {
            let new_matches = &matched_amount - &bet.matched_amount;
            self.update_total_matched(bet.event, bet.selection.id, &new_matches);
        }
        
        // Adăugăm în orderbook doar dacă mai avem sumă unmatched
        if remaining > BigUint::zero() {
            self.add_to_orderbook(&bet);
        }
    
        self.bet_by_id(bet.nft_nonce).set(&bet);
    
        (matched_amount, remaining)
    }

    fn add_to_orderbook(&self, bet: &Bet<Self::Api>) {
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
                level.total_stake += &bet.unmatched_amount;
                level.bet_nonces.push(bet.nft_nonce);
                let _ = levels.set(i, level);
            },
            None => {
                let new_level = PriceLevel {
                    odds: bet.odd.clone(),
                    total_stake: bet.unmatched_amount.clone(),
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
            level.total_stake -= &bet.unmatched_amount;
            
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
                        .update(|val| *val -= &bet.unmatched_amount);
                },
                BetType::Lay => {
                    self.selection_lay_levels(bet.event, bet.selection.id).set(&levels);
                    self.selection_lay_liquidity(bet.event, bet.selection.id)
                        .update(|val| *val -= &bet.unmatched_amount);
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
            .update(|total| *total += matched_amount);
    }

    fn count_valid_bets_at_level(&self, level: &PriceLevel<Self::Api>) -> u32 {
        let mut count = 0u32;
        let mut processed_bettors = ManagedVec::<Self::Api, ManagedAddress<Self::Api>>::new();
        
        for nonce in level.bet_nonces.iter() {
            let bet = self.bet_by_id(nonce).get();
            if bet.unmatched_amount > BigUint::zero() {
                let mut is_unique = true;
                for processed_bettor in processed_bettors.iter() {
                    if bet.bettor == *processed_bettor {
                        is_unique = false;
                        break;
                    }
                }
                if is_unique {
                    count += 1;
                    processed_bettors.push(bet.bettor);
                }
            }
        }
        count
    }

    #[view(getMatchingDetails)]
    fn get_matching_details(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> MatchingDetails<Self::Api> {
        let back_levels = self.selection_back_levels(market_id, selection_id).get();
        let lay_levels = self.selection_lay_levels(market_id, selection_id).get();
        let back_liquidity = self.selection_back_liquidity(market_id, selection_id).get();
        let lay_liquidity = self.selection_lay_liquidity(market_id, selection_id).get();
        let matched_count = self.selection_matched_count(market_id, selection_id).get();
        let unmatched_count = self.selection_unmatched_count(market_id, selection_id).get();
        let partially_matched_count = self.selection_partially_matched_count(market_id, selection_id).get();

        let mut back_level_views = ManagedVec::new();
        for level in back_levels.iter() {
            if level.total_stake > BigUint::zero() && !level.bet_nonces.is_empty() {
                let mut bets = ManagedVec::new();
                for nonce in level.bet_nonces.iter() {
                    let bet = self.bet_by_id(nonce).get();
                    bets.push(BetView {
                        nonce,
                        bettor: bet.bettor,
                        stake: bet.stake_amount,
                        matched: bet.matched_amount,
                        unmatched: bet.unmatched_amount,
                        status: bet.status
                    });
                }
                
                back_level_views.push(PriceLevelView {
                    odds: level.odds,
                    total_stake: level.total_stake,
                    bets
                });
            }
        }

        let mut lay_level_views = ManagedVec::new();
        for level in lay_levels.iter() {
            if level.total_stake > BigUint::zero() && !level.bet_nonces.is_empty() {
                let mut bets = ManagedVec::new();
                for nonce in level.bet_nonces.iter() {
                    let bet = self.bet_by_id(nonce).get();
                    bets.push(BetView {
                        nonce,
                        bettor: bet.bettor,
                        stake: bet.stake_amount,
                        matched: bet.matched_amount,
                        unmatched: bet.unmatched_amount,
                        status: bet.status
                    });
                }
                
                lay_level_views.push(PriceLevelView {
                    odds: level.odds,
                    total_stake: level.total_stake,
                    bets
                });
            }
        }

        MatchingDetails {
            back_levels: back_level_views,
            lay_levels: lay_level_views,
            back_liquidity,
            lay_liquidity,
            matched_count,
            unmatched_count,
            partially_matched_count
        }
    }

    #[view(getBetMatchingState)]
    fn get_bet_matching_state(
        &self,
        bet_nonce: u64
    ) -> BetMatchingState<Self::Api> {
        let bet = self.bet_by_id(bet_nonce).get();
        
        BetMatchingState {
            bet_type: bet.bet_type,
            original_stake: bet.stake_amount,
            matched_amount: bet.matched_amount,
            unmatched_amount: bet.unmatched_amount,
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
        BetView {
            nonce: bet.nft_nonce,
            bettor: bet.bettor,
            stake: bet.stake_amount,
            matched: bet.matched_amount,
            unmatched: bet.unmatched_amount,
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