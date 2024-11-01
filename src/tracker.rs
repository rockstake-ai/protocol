use crate::types::{Bet, BetMatchingState, BetStatus, BetType, BetView, MatchingDetails, OrderbookView, PriceLevel, PriceLevelView, Tracker};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait TrackerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
{
    fn init_tracker(&self) {
        self.back_levels().set(&ManagedVec::new());
        self.lay_levels().set(&ManagedVec::new());
        self.back_liquidity().set(&BigUint::zero());
        self.lay_liquidity().set(&BigUint::zero());
        self.matched_count().set(&0u64);
        self.unmatched_count().set(&0u64);
        self.partially_matched_count().set(&0u64);
        self.win_count().set(&0u64);
        self.lost_count().set(&0u64);
        self.canceled_count().set(&0u64);
    }

    fn process_bet(&self, mut bet: Bet<Self::Api>) -> (BigUint, BigUint) {
        let mut matched_amount = BigUint::zero();
        let mut remaining = bet.stake_amount.clone();

        // Get opposite levels for matching
        let mut levels = match bet.bet_type {
            BetType::Back => self.lay_levels().get(),
            BetType::Lay => self.back_levels().get(),
        };

        // Match against existing orders
        let mut i = 0;
        while i < levels.len() && remaining > BigUint::zero() {
            let mut level = levels.get(i);
            
            // Match only when odds are exactly equal
            if level.odds == bet.odd {
                let match_amount = remaining.clone().min(level.total_stake.clone());
                
                if match_amount > BigUint::zero() {
                    matched_amount += &match_amount;
                    remaining -= &match_amount;
                    level.total_stake -= &match_amount;

                    let mut updated_nonces = ManagedVec::new();
                    let mut matched_any = false;

                    for nonce in level.bet_nonces.iter() {
                        let mut matched_bet = self.bet_by_id(nonce).get();
                        
                        if matched_bet.unmatched_amount > BigUint::zero() {
                            let bet_match = matched_bet.unmatched_amount.clone().min(match_amount.clone());
                            
                            if bet_match > BigUint::zero() {
                                matched_bet.matched_amount += &bet_match;
                                matched_bet.unmatched_amount -= &bet_match;
                                matched_any = true;
                                
                                matched_bet.status = if matched_bet.unmatched_amount == BigUint::zero() {
                                    self.matched_count().update(|val| *val += 1);
                                    BetStatus::Matched
                                } else {
                                    self.partially_matched_count().update(|val| *val += 1);
                                    BetStatus::PartiallyMatched
                                };
                                
                                self.bet_by_id(nonce).set(&matched_bet);

                                if matched_bet.unmatched_amount > BigUint::zero() {
                                    updated_nonces.push(nonce);
                                }
                            }
                        }
                    }

                    // Only keep the level if there were actual matches and remaining unmatched amount
                    if matched_any && !updated_nonces.is_empty() {
                        level.bet_nonces = updated_nonces;
                        levels.set(i, &level);
                        i += 1;
                    } else {
                        // Remove the level if no valid matches remain
                        if i < levels.len() - 1 {
                            let last = levels.get(levels.len() - 1);
                            levels.set(i, &last);
                        }
                        levels.remove(levels.len() - 1);
                    }
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        // Update opposite side levels
        match bet.bet_type {
            BetType::Back => self.lay_levels().set(&levels),
            BetType::Lay => self.back_levels().set(&levels),
        }

        // Update bet state
        bet.matched_amount = matched_amount.clone();
        bet.unmatched_amount = remaining.clone();
        
        bet.status = if remaining == BigUint::zero() {
            self.matched_count().update(|val| *val += 1);
            BetStatus::Matched
        } else if matched_amount > BigUint::zero() {
            self.partially_matched_count().update(|val| *val += 1);
            BetStatus::PartiallyMatched
        } else {
            self.unmatched_count().update(|val| *val += 1);
            BetStatus::Unmatched
        };

        if matched_amount > BigUint::zero() {
            self.update_total_matched(bet.event, bet.selection.selection_id, &matched_amount);
        }
        
        // Add remaining amount to orderbook if any exists
        if remaining > BigUint::zero() {
            self.add_to_orderbook(&bet);
        }

        self.bet_by_id(bet.nft_nonce).set(&bet);

        (matched_amount, remaining)
    }

    fn add_to_orderbook(&self, bet: &Bet<Self::Api>) {
        let mut levels = match bet.bet_type {
            BetType::Back => self.back_levels().get(),
            BetType::Lay => self.lay_levels().get(),
        };

        // Find existing level or insertion position
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
                // Add to existing level
                let mut level = levels.get(i);
                level.total_stake += &bet.unmatched_amount;
                level.bet_nonces.push(bet.nft_nonce);
                levels.set(i, &level);
            },
            None => {
                // Create new level
                let new_level = PriceLevel {
                    odds: bet.odd.clone(),
                    total_stake: bet.unmatched_amount.clone(),
                    bet_nonces: ManagedVec::from_single_item(bet.nft_nonce),
                };

                // Find insert position to maintain proper ordering
                let mut insert_pos = levels.len();
                for i in 0..levels.len() {
                    let level = levels.get(i);
                    match bet.bet_type {
                        BetType::Back => {
                            if bet.odd >= level.odds {
                                insert_pos = i;
                                break;
                            }
                        },
                        BetType::Lay => {
                            if bet.odd <= level.odds {
                                insert_pos = i;
                                break;
                            }
                        },
                    }
                }

                // Insert new level
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

        // Update storage
        match bet.bet_type {
            BetType::Back => {
                self.back_levels().set(&levels);
                self.back_liquidity().update(|val| {
                    *val = levels.iter().fold(BigUint::zero(), |acc, level| acc + &level.total_stake)
                });
            },
            BetType::Lay => {
                self.lay_levels().set(&levels);
                self.lay_liquidity().update(|val| {
                    *val = levels.iter().fold(BigUint::zero(), |acc, level| acc + &level.total_stake)
                });
            },
        }
    }

    #[view(getMatchingDetails)]
    fn get_matching_details(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> MultiValueEncoded<Self::Api, OrderbookView<Self::Api>> {
        let mut result = MultiValueEncoded::new();
        
        // Process back levels (descending order)
        let back_levels = self.back_levels().get();
        for level in back_levels.iter() {
            if level.total_stake > BigUint::zero() && !level.bet_nonces.is_empty() {
                result.push(OrderbookView {
                    price_level: level.odds.clone(),
                    total_amount: level.total_stake.clone(),
                    bet_count: self.count_valid_bets_at_level(&level)
                });
            }
        }
        
        // Process lay levels (ascending order)
        let lay_levels = self.lay_levels().get();
        for level in lay_levels.iter() {
            if level.total_stake > BigUint::zero() && !level.bet_nonces.is_empty() {
                result.push(OrderbookView {
                    price_level: level.odds.clone(),
                    total_amount: level.total_stake.clone(),
                    bet_count: self.count_valid_bets_at_level(&level)
                });
            }
        }
        
        result
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
        let matched_count = self.matched_count().get() as u32;
        let total_matched = self.total_matched_amount(market_id, selection_id).get();
        (matched_count, total_matched)
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
}