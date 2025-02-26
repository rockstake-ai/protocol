use crate::{errors::{ERR_INVALID_BACK_LIQUIDITY, ERR_INVALID_LAY_LIQUIDITY, ERR_INVALID_MATCHED_AMOUNT}, types::{Bet, BetStatus, BetType, MatchedPart, PriceLevel}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait OrderbookModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
{
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Bet Processing --------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Processes a bet by matching it against existing opposite orders and updating its status.
    /// Parameters:
    /// - bet: The bet to process.
    /// Returns: A tuple containing the updated bet, total matched amount, and remaining unmatched amount.
    fn process_bet(&self, mut bet: Bet<Self::Api>) -> (Bet<Self::Api>, BigUint, BigUint) {
        let mut total_matched = bet.total_matched.clone();
        let mut remaining = &bet.stake_amount - &bet.total_matched;

        let remaining_liability = self.calculate_remaining_liability(&bet, &remaining);
        let opposite_levels = self.get_opposite_levels(&bet);

        self.match_bet_against_levels(&mut bet, &mut total_matched, &mut remaining, opposite_levels);
        self.update_bet_status_and_totals(&mut bet, &total_matched, &remaining);

        bet.liability = remaining_liability;
        self.bet_by_id(bet.bet_id).set(&bet);

        (bet, total_matched, remaining)
    }

    /// Matches a bet against opposite price levels and updates matched parts.
    /// Parameters:
    /// - bet: The bet being processed.
    /// - total_matched: The cumulative matched amount (updated in place).
    /// - remaining: The remaining unmatched amount (updated in place).
    /// - opposite_levels: The list of opposite price levels to match against.
    fn match_bet_against_levels(
        &self,
        bet: &mut Bet<Self::Api>,
        total_matched: &mut BigUint<Self::Api>,  
        remaining: &mut BigUint<Self::Api>,      
        mut opposite_levels: ManagedVec<Self::Api, PriceLevel<Self::Api>>,
    ) {
        let current_timestamp = self.blockchain().get_block_timestamp();
        let mut i = 0;
    
        while i < opposite_levels.len() && *remaining > BigUint::zero() {
            let mut level = opposite_levels.get(i);
            if level.odds == bet.odd {
                let mut to_match = BigUint::zero();
                if *remaining < level.total_stake {
                    to_match = remaining.clone();
                } else {
                    to_match = level.total_stake.clone();
                }
                
                if &to_match > &BigUint::zero() {
                    *total_matched += &to_match;
                    *remaining -= &to_match;
                    self.process_level_matches(bet, &to_match, &mut level, current_timestamp);
                    self.update_opposite_levels(&mut opposite_levels, i, level);
                }
                i += 1;
            } else {
                i += 1;
            }
        }
    
        self.save_opposite_levels(bet, opposite_levels);  
    }

    /// Processes matches for a specific price level and updates counterparty bets.
    /// Parameters:
    /// - bet: The bet being matched.
    /// - match_amount: The amount to match at this level.
    /// - level: The price level being processed (updated in place).
    /// - current_timestamp: The timestamp of the match.
    fn process_level_matches(
        &self,
        bet: &mut Bet<Self::Api>,
        match_amount: &BigUint,
        level: &mut PriceLevel<Self::Api>,
        current_timestamp: u64,
    ) {
        let mut updated_nonces = ManagedVec::new();
        let mut total_level_stake = BigUint::zero();

        for nonce in level.bet_nonces.iter() {
            let mut matched_bet = self.bet_by_id(nonce).get();
            let current_unmatched = &matched_bet.stake_amount - &matched_bet.total_matched;

            if current_unmatched > BigUint::zero() {
                let match_this_bet = current_unmatched.min(match_amount.clone());
                if match_this_bet > BigUint::zero() {
                    self.add_matched_part(bet, &mut matched_bet, &match_this_bet, current_timestamp);
                    self.update_matched_bet(&mut matched_bet, &match_this_bet);

                    let remaining_unmatched = &matched_bet.stake_amount - &matched_bet.total_matched;
                    if remaining_unmatched > BigUint::zero() {
                        updated_nonces.push(nonce);
                        total_level_stake += remaining_unmatched;
                    }

                    self.bet_by_id(nonce).set(&matched_bet);
                }
            }
        }

        level.bet_nonces = updated_nonces;
        level.total_stake = total_level_stake;
    }

    /// Updates the status of a bet and related totals after matching.
    /// Parameters:
    /// - bet: The bet being updated.
    /// - total_matched: The total matched amount.
    /// - remaining: The remaining unmatched amount.
    fn update_bet_status_and_totals(
        &self,
        bet: &mut Bet<Self::Api>,
        total_matched: &BigUint,
        remaining: &BigUint,
    ) {
        bet.total_matched = total_matched.clone();
        bet.status = if *remaining == BigUint::zero() {
            bet.stake_amount = total_matched.clone();
            bet.total_amount = match bet.bet_type {
                BetType::Back => total_matched.clone(),
                BetType::Lay => bet.total_amount.clone(),
            };
            self.selection_matched_count(bet.event, bet.selection.id).update(|val| *val += 1);
            BetStatus::Matched
        } else if *total_matched > BigUint::zero() {
            self.selection_partially_matched_count(bet.event, bet.selection.id).update(|val| *val += 1);
            BetStatus::PartiallyMatched
        } else {
            self.selection_unmatched_count(bet.event, bet.selection.id).update(|val| *val += 1);
            BetStatus::Unmatched
        };

        bet.potential_profit = self.calculate_total_potential_profit(bet);
        if *total_matched > BigUint::zero() {
            let new_matches = total_matched - &bet.total_matched;
            self.update_total_matched(bet.event, bet.selection.id, &new_matches);
        }

        if *remaining > BigUint::zero() {
            self.add_to_orderbook(bet);
        }
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Orderbook Management --------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Adds a bet to the order book if it has unmatched amounts.
    /// Parameters:
    /// - bet: The bet to add to the order book.
    fn add_to_orderbook(&self, bet: &Bet<Self::Api>) {
        let unmatched_amount = &bet.stake_amount - &bet.total_matched;
        if unmatched_amount == BigUint::zero() { 
            return;
        }
    
        let mut levels = match bet.bet_type {
            BetType::Back => self.selection_back_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_lay_levels(bet.event, bet.selection.id).get(),
        };
    
        let level_index = self.find_level_index(&levels, &bet.odd);
        match level_index {
            Some(i) => {
                let mut level = levels.get(i);
                level.total_stake += &unmatched_amount;  
                level.bet_nonces.push(bet.bet_id);
                let _ = levels.set(i, level);
            },
            None => {
                let new_level = PriceLevel {
                    odds: bet.odd.clone(),
                    total_stake: unmatched_amount.clone(),  
                    bet_nonces: ManagedVec::from_single_item(bet.bet_id),
                };
                levels.push(new_level);
            }
        };
    
        self.update_levels_and_liquidity(bet, levels, unmatched_amount.clone());  
    }

    /// Removes a bet from the order book if it has unmatched amounts.
    /// Parameters:
    /// - bet: The bet to remove from the order book.
    fn remove_from_orderbook(&self, bet: &Bet<Self::Api>) {
        let unmatched_amount = &bet.stake_amount - &bet.total_matched;
        let mut levels = match bet.bet_type {
            BetType::Back => self.selection_back_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_lay_levels(bet.event, bet.selection.id).get(),
        };
    
        let level_index = self.find_level_index(&levels, &bet.odd);
        if let Some(i) = level_index {
            let mut level = levels.get(i);
            level.total_stake -= &unmatched_amount;  
    
            let mut updated_nonces = ManagedVec::new();
            for nonce in level.bet_nonces.iter() {
                if nonce != bet.bet_id {
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
    
            let negative_amount = BigUint::zero() - &unmatched_amount;
            self.update_levels_and_liquidity(bet, levels, negative_amount);
        }
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Helper Functions ------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Calculates the potential profit for a bet based on its type and matched amount.
    /// Parameters:
    /// - bet: The bet to calculate profit for.
    /// Returns: The total potential profit as BigUint.
    fn calculate_total_potential_profit(&self, bet: &Bet<Self::Api>) -> BigUint {
        match bet.bet_type {
            BetType::Back => {
                &bet.total_matched * &(&bet.odd - &BigUint::from(100u64)) / &BigUint::from(100u64)
            },
            BetType::Lay => bet.total_matched.clone(),
        }
    }

    /// Calculates the remaining liability for a bet.
    /// Parameters:
    /// - bet: The bet to calculate liability for.
    /// - remaining: The remaining unmatched amount.
    /// Returns: The remaining liability as BigUint.
    fn calculate_remaining_liability(&self, bet: &Bet<Self::Api>, remaining: &BigUint) -> BigUint {
        match bet.bet_type {
            BetType::Back => BigUint::zero(),
            BetType::Lay => {
                let odds_minus_one = &bet.odd - &BigUint::from(100u64);
                (remaining * &odds_minus_one) / &BigUint::from(100u64)
            }
        }
    }

    /// Retrieves the opposite price levels for a bet (lay for back bets, back for lay bets).
    /// Parameters:
    /// - bet: The bet to get opposite levels for.
    /// Returns: A vector of price levels.
    fn get_opposite_levels(&self, bet: &Bet<Self::Api>) -> ManagedVec<Self::Api, PriceLevel<Self::Api>> {
        match bet.bet_type {
            BetType::Back => self.selection_lay_levels(bet.event, bet.selection.id).get(),
            BetType::Lay => self.selection_back_levels(bet.event, bet.selection.id).get(),
        }
    }

    /// Adds a matched part to both the current bet and the counterparty bet.
    /// Parameters:
    /// - bet: The current bet being matched.
    /// - matched_bet: The counterparty bet being updated.
    /// - match_amount: The amount matched.
    /// - timestamp: The timestamp of the match.
    fn add_matched_part(
        &self,
        bet: &mut Bet<Self::Api>,
        matched_bet: &mut Bet<Self::Api>,
        match_amount: &BigUint,
        timestamp: u64,
    ) {
        bet.matched_parts.push(MatchedPart {
            matched_with: matched_bet.bettor.clone(),
            amount: match_amount.clone(),
            odds: bet.odd.clone(),
            matched_at: timestamp,
            counterparty_bet_id: matched_bet.nft_nonce,
            counterparty_payment_token: matched_bet.payment_token.clone(),
            counterparty_payment_nonce: matched_bet.payment_nonce,
        });

        matched_bet.matched_parts.push(MatchedPart {
            matched_with: bet.bettor.clone(),
            amount: match_amount.clone(),
            odds: matched_bet.odd.clone(),
            matched_at: timestamp,
            counterparty_bet_id: bet.nft_nonce,
            counterparty_payment_token: bet.payment_token.clone(),
            counterparty_payment_nonce: bet.payment_nonce,
        });
    }

    /// Updates the status and totals of a matched counterparty bet.
    /// Parameters:
    /// - matched_bet: The counterparty bet being updated.
    /// - match_amount: The amount matched.
    fn update_matched_bet(&self, matched_bet: &mut Bet<Self::Api>, match_amount: &BigUint) {
        matched_bet.total_matched += match_amount;
        if matched_bet.bet_type == BetType::Lay {
            let matched_ratio = match_amount / &matched_bet.stake_amount;
            let matched_total = &matched_bet.total_amount * &matched_ratio;
            matched_bet.total_amount -= &matched_total;
        }

        matched_bet.status = if &matched_bet.total_matched == &matched_bet.stake_amount {
            BetStatus::Matched
        } else {
            BetStatus::PartiallyMatched
        };

        matched_bet.potential_profit = self.calculate_total_potential_profit(matched_bet);
    }

    /// Updates the opposite levels after matching.
    /// Parameters:
    /// - opposite_levels: The list of opposite levels (updated in place).
    /// - index: The index of the level being updated.
    /// - level: The updated price level.
    fn update_opposite_levels(
        &self,
        opposite_levels: &mut ManagedVec<Self::Api, PriceLevel<Self::Api>>,
        index: usize,
        level: PriceLevel<Self::Api>,
    ) {
        if !level.bet_nonces.is_empty() {
            let _ = opposite_levels.set(index, level);
        } else if index < opposite_levels.len() - 1 {
            let last = opposite_levels.get(opposite_levels.len() - 1);
            let _ = opposite_levels.set(index, last);
            opposite_levels.remove(opposite_levels.len() - 1);
        } else {
            opposite_levels.remove(opposite_levels.len() - 1);
        }
    }

    /// Saves the updated opposite levels back to storage.
    /// Parameters:
    /// - bet: The bet being processed.
    /// - opposite_levels: The updated list of opposite levels.
    fn save_opposite_levels(
        &self,
        bet: &Bet<Self::Api>,
        opposite_levels: ManagedVec<Self::Api, PriceLevel<Self::Api>>,
    ) {
        match bet.bet_type {
            BetType::Back => self.selection_lay_levels(bet.event, bet.selection.id).set(&opposite_levels),
            BetType::Lay => self.selection_back_levels(bet.event, bet.selection.id).set(&opposite_levels),
        };
    }

    /// Finds the index of a level with matching odds in the order book.
    /// Parameters:
    /// - levels: The list of price levels.
    /// - odds: The odds to match.
    /// Returns: The index of the matching level, if found.
    fn find_level_index(
        &self,
        levels: &ManagedVec<Self::Api, PriceLevel<Self::Api>>,
        odds: &BigUint,
    ) -> Option<usize> {
        for i in 0..levels.len() {
            let level = levels.get(i);
            if level.odds == *odds {
                return Some(i);
            }
        }
        None
    }

    /// Updates levels and liquidity in storage after adding or removing a bet.
    /// Parameters:
    /// - bet: The bet affecting the levels.
    /// - levels: The updated list of price levels.
    /// - amount_change: The change in liquidity (positive for add, negative for remove).
    fn update_levels_and_liquidity(
        &self,
        bet: &Bet<Self::Api>,
        levels: ManagedVec<Self::Api, PriceLevel<Self::Api>>,
        amount_change: BigUint  // Plus une référence ici
    ) {
        match bet.bet_type {
            BetType::Back => {
                self.selection_back_levels(bet.event, bet.selection.id).set(&levels);
                self.selection_back_liquidity(bet.event, bet.selection.id)
                    .update(|val| *val += &amount_change);
                require!(
                    self.selection_back_liquidity(bet.event, bet.selection.id).get() >= BigUint::zero(),
                    ERR_INVALID_BACK_LIQUIDITY
                );
            },
            BetType::Lay => {
                self.selection_lay_levels(bet.event, bet.selection.id).set(&levels);
                self.selection_lay_liquidity(bet.event, bet.selection.id)
                    .update(|val| *val += &amount_change);
                require!(
                    self.selection_lay_liquidity(bet.event, bet.selection.id).get() >= BigUint::zero(),
                    ERR_INVALID_LAY_LIQUIDITY
                );
            },
        }
    }

    /// Updates the total matched amount for a selection.
    /// Parameters:
    /// - market_id: The ID of the market.
    /// - selection_id: The ID of the selection.
    /// - matched_amount: The amount to add to the total matched.
    fn update_total_matched(&self, market_id: u64, selection_id: u64, matched_amount: &BigUint) {
        self.total_matched_amount(market_id, selection_id)
            .update(|total| {
                *total += matched_amount;
                require!(
                    *total <= self.selection_back_liquidity(market_id, selection_id).get(),
                    ERR_INVALID_MATCHED_AMOUNT
                );
            });
    }
}