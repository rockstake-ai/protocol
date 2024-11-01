use crate::types::{Bet, BetStatus, BetType, PriceLevel, Tracker};
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

        // Get mutable reference to appropriate levels
        let mut levels = match bet.bet_type {
            BetType::Back => self.lay_levels().get(),
            BetType::Lay => self.back_levels().get(),
        };

        // Încercăm să găsim matches în primele nivele disponibile
        let mut i = 0;
        while i < levels.len() && remaining > BigUint::zero() {
            let mut level = levels.get(i);
            
            if self.can_match(&bet, &level.odds) {
                let match_amount = remaining.clone().min(level.total_stake.clone());
                
                if match_amount > BigUint::zero() {
                    matched_amount += &match_amount;
                    remaining -= &match_amount;
                    level.total_stake -= &match_amount;

                    // Update matched bets
                    for nonce in level.bet_nonces.iter() {
                        let mut matched_bet = self.bet_by_id(nonce).get();
                        let bet_match = matched_bet.unmatched_amount.clone().min(match_amount.clone());
                        
                        if bet_match > BigUint::zero() {
                            matched_bet.matched_amount += &bet_match;
                            matched_bet.unmatched_amount -= &bet_match;
                            
                            // Update bet status
                            matched_bet.status = if matched_bet.unmatched_amount == BigUint::zero() {
                                self.matched_count().update(|val| *val += 1);
                                BetStatus::Matched
                            } else {
                                self.partially_matched_count().update(|val| *val += 1);
                                BetStatus::PartiallyMatched
                            };
                            
                            self.bet_by_id(nonce).set(&matched_bet);
                        }
                    }

                    // Clean up empty bets from level
                    let mut updated_nonces = ManagedVec::new();
                    for nonce in level.bet_nonces.iter() {
                        let bet = self.bet_by_id(nonce).get();
                        if bet.unmatched_amount > BigUint::zero() {
                            updated_nonces.push(nonce);
                        }
                    }
                    level.bet_nonces = updated_nonces;

                    if level.total_stake > BigUint::zero() && !level.bet_nonces.is_empty() {
                        levels.set(i, &level);
                        i += 1;
                    } else {
                        // Remove empty level
                        if i < levels.len() - 1 {
                            let last = levels.get(levels.len() - 1);
                            levels.set(i, &last);
                        }
                        levels.remove(levels.len() - 1);
                    }
                }
            } else {
                i += 1;
            }
        }

        // Save updated levels
        match bet.bet_type {
            BetType::Back => self.lay_levels().set(&levels),
            BetType::Lay => self.back_levels().set(&levels),
        }

        // Update current bet
        bet.matched_amount = matched_amount.clone();
        bet.unmatched_amount = remaining.clone();
        
        // Add remaining amount to orderbook if any
        if remaining > BigUint::zero() {
            self.add_to_orderbook(&mut bet);
            self.unmatched_count().update(|val| *val += 1);
        }

        // Store final bet state
        self.bet_by_id(bet.nft_nonce).set(&bet);

        (matched_amount, remaining)
    }

    fn add_to_orderbook(&self, bet: &Bet<Self::Api>) {
        let mut levels = match bet.bet_type {
            BetType::Back => self.back_levels().get(),
            BetType::Lay => self.lay_levels().get(),
        };

        // Try to find existing level
        let mut level_found = false;
        for i in 0..levels.len() {
            let mut level = levels.get(i);
            if level.odds == bet.odd {
                level.total_stake += &bet.unmatched_amount;
                level.bet_nonces.push(bet.nft_nonce);
                levels.set(i, &level);
                level_found = true;
                break;
            }
        }

        // Create new level if needed
        if !level_found {
            let mut new_level = PriceLevel {
                odds: bet.odd.clone(),
                total_stake: bet.unmatched_amount.clone(),
                bet_nonces: ManagedVec::new(),
            };
            new_level.bet_nonces.push(bet.nft_nonce);

            // Find correct position for new level
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

            // Insert new level
            if insert_pos == levels.len() {
                levels.push(new_level);
            } else {
                levels.push(levels.get(levels.len() - 1));
                for i in (insert_pos..levels.len()-1).rev() {
                    let temp = levels.get(i);
                    levels.set(i + 1, &temp);
                }
                levels.set(insert_pos, &new_level);
            }
        }

        // Save updated levels
        match bet.bet_type {
            BetType::Back => {
                self.back_levels().set(&levels);
                self.back_liquidity().update(|val| *val += &bet.unmatched_amount);
            },
            BetType::Lay => {
                self.lay_levels().set(&levels);
                self.lay_liquidity().update(|val| *val += &bet.unmatched_amount);
            },
        }
    }

    fn can_match(&self, bet: &Bet<Self::Api>, level_odds: &BigUint) -> bool {
        match bet.bet_type {
            BetType::Back => bet.odd >= *level_odds,
            BetType::Lay => bet.odd <= *level_odds,
        }
    }

}