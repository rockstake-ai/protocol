multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::types::{Bet, BetScheduler, BetStatus, BetType};

#[multiversx_sc::module]
pub trait BetSchedulerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule + {

    fn init_bet_scheduler(&self) -> BetScheduler<Self::Api> {
        BetScheduler {
            back_bets: ManagedVec::new(),
            lay_bets: ManagedVec::new(),
            best_back_odds: BigUint::zero(),
            best_lay_odds: BigUint::zero(),
            back_liquidity: BigUint::zero(),
            lay_liquidity: BigUint::zero(),
            matched_count: 0,
            unmatched_count: 0,
            partially_matched_count: 0,
            win_count: 0,
            lost_count: 0,
            canceled_count: 0,
        }
    }

    fn add(&self, bet: Bet<Self::Api>) {
        let mut scheduler = self.bet_scheduler().get();
        let old_status = bet.status.clone();
        let mut new_bet = bet;
        new_bet.status = BetStatus::Unmatched;
        self.update_status_counters(&mut scheduler, &old_status, &new_bet.status);

        match new_bet.bet_type {
            BetType::Back => {
                let mut queue = scheduler.back_bets.clone();
                self.insert_bet(&mut queue, new_bet.clone());
                scheduler.back_bets = queue;
                scheduler.back_liquidity += &new_bet.stake_amount;
                self.update_best_back_odds(&mut scheduler);
            },
            BetType::Lay => {
                let mut queue = scheduler.lay_bets.clone();
                self.insert_bet(&mut queue, new_bet.clone());
                scheduler.lay_bets = queue;
                scheduler.lay_liquidity += &new_bet.liability;
                self.update_best_lay_odds(&mut scheduler);
            },
        };
        self.bet_scheduler().set(scheduler);
    }

    fn remove(&self, bet: Bet<Self::Api>) -> Option<Bet<Self::Api>> {
        let mut scheduler = self.bet_scheduler().get();
        let queue = match bet.bet_type {
            BetType::Back => &mut scheduler.back_bets,
            BetType::Lay => &mut scheduler.lay_bets,
        };

        let mut index_to_remove = None;
        for i in 0..queue.len() {
            if queue.get(i).nft_nonce == bet.nft_nonce {
                index_to_remove = Some(i);
                break;
            }
        }

        if let Some(index) = index_to_remove {
            let removed_bet = queue.get(index);
            
            let mut new_queue = ManagedVec::new();
            for i in 0..queue.len() {
                if i != index {
                    new_queue.push(queue.get(i));
                }
            }
            *queue = new_queue;

            match bet.bet_type {
                BetType::Back => {
                    scheduler.back_liquidity -= &removed_bet.unmatched_amount;
                    self.update_best_back_odds(&mut scheduler);
                },
                BetType::Lay => {
                    scheduler.lay_liquidity -= &removed_bet.liability;
                    self.update_best_lay_odds(&mut scheduler);
                },
            }
            self.bet_scheduler().set(scheduler);
            Some(removed_bet)
        } else {
            None
        }
    }

    fn get_matching_bets(
        &self,
        bet: Bet<Self::Api>
    ) -> (ManagedVec<Self::Api, Bet<Self::Api>>, BigUint, BigUint) {
        let scheduler = self.bet_scheduler().get();
        let mut matched_amount = BigUint::zero();
        let mut unmatched_amount = match bet.bet_type {
            BetType::Back => bet.stake_amount.clone(),
            BetType::Lay => bet.liability.clone(),
        };
        let mut matching_bets = ManagedVec::new();
        let source = match bet.bet_type {
            BetType::Back => &scheduler.lay_bets,
            BetType::Lay => &scheduler.back_bets,
        };
    
        for i in 0..source.len() {
            let existing_bet = source.get(i);
            let is_match = match bet.bet_type {
                BetType::Back => bet.odd >= existing_bet.odd,
                BetType::Lay => bet.odd <= existing_bet.odd,
            };
    
            if is_match {
                let match_amount = if bet.bet_type == BetType::Back {
                    unmatched_amount.clone().min(existing_bet.unmatched_amount.clone())
                } else {
                    unmatched_amount.clone().min(existing_bet.stake_amount.clone())
                };
    
                matched_amount += &match_amount;
                unmatched_amount -= &match_amount;
    
                let mut updated_bet = existing_bet.clone();
                updated_bet.matched_amount += &match_amount;
                updated_bet.unmatched_amount -= &match_amount;
                
                if bet.bet_type == BetType::Lay {
                    updated_bet.liability = &match_amount * &(&bet.odd - &BigUint::from(1u32));
                }
                
                matching_bets.push(updated_bet);
    
                if unmatched_amount == BigUint::zero() {
                    break;
                }
            } else {
                break;
            }
        }
    
        (matching_bets, matched_amount, unmatched_amount)
    }

    
    fn match_bet(&self, bet: Bet<Self::Api>) -> (BigUint, BigUint, Bet<Self::Api>) {
        let mut scheduler = self.bet_scheduler().get();
        let old_status = bet.status.clone();
        let (matching_bets, matched_amount, unmatched_amount) = self.get_matching_bets(bet.clone());
        
        let mut updated_bet = bet;
        updated_bet.matched_amount = matched_amount.clone();
        updated_bet.unmatched_amount = unmatched_amount.clone();
        
        let new_status = match updated_bet.bet_type {
            BetType::Back => {
                if matched_amount == updated_bet.stake_amount {
                    BetStatus::Matched
                } else if matched_amount > BigUint::zero() {
                    BetStatus::PartiallyMatched
                } else {
                    BetStatus::Unmatched
                }
            },
            BetType::Lay => {
                if matched_amount == updated_bet.liability {
                    BetStatus::Matched
                } else if matched_amount > BigUint::zero() {
                    BetStatus::PartiallyMatched
                } else {
                    BetStatus::Unmatched
                }
            }
        };
    
        if old_status != new_status {
            self.update_status_counters(&mut scheduler, &old_status, &new_status);
        }
        updated_bet.status = new_status;
    
        for matched_bet in matching_bets.iter() {
            let old_matched_status = matched_bet.status.clone();
            self.remove(matched_bet.clone());
            
            let mut updated_matched_bet = matched_bet;
            let new_matched_status = match updated_matched_bet.bet_type {
                BetType::Back => {
                    if updated_matched_bet.matched_amount == updated_matched_bet.stake_amount {
                        BetStatus::Matched
                    } else {
                        BetStatus::PartiallyMatched
                    }
                },
                BetType::Lay => {
                    if updated_matched_bet.matched_amount == updated_matched_bet.liability {
                        BetStatus::Matched
                    } else {
                        BetStatus::PartiallyMatched
                    }
                }
            };
    
            if old_matched_status != new_matched_status {
                self.update_status_counters(&mut scheduler, &old_matched_status, &new_matched_status);
            }
            updated_matched_bet.status = new_matched_status;
    
            if updated_matched_bet.unmatched_amount > BigUint::zero() {
                self.add(updated_matched_bet);
            }
        }
    
        if updated_bet.unmatched_amount > BigUint::zero() {
            self.add(updated_bet.clone());
        }
    
        self.bet_scheduler().set(scheduler);
        (matched_amount, unmatched_amount, updated_bet)
    }

    fn update_bet_status(&self, bet: Bet<Self::Api>, new_status: BetStatus) -> Bet<Self::Api> {
        let mut scheduler = self.bet_scheduler().get();
        let old_status = bet.status.clone(); 
        self.update_status_counters(&mut scheduler, &old_status, &new_status);
        let mut updated_bet = bet;
        updated_bet.status = new_status;
        self.bet_scheduler().set(scheduler);
        updated_bet
    }

    fn insert_bet(&self, queue: &mut ManagedVec<Self::Api, Bet<Self::Api>>, bet: Bet<Self::Api>) {
        let mut insert_index = queue.len();
        for i in 0..queue.len() {
            if self.should_insert_before(&bet, &queue.get(i), bet.bet_type == BetType::Back) {
                insert_index = i;
                break;
            }
        }
        
        let mut new_queue = ManagedVec::new();
        for i in 0..insert_index {
            new_queue.push(queue.get(i));
        }
        new_queue.push(bet);
        for i in insert_index..queue.len() {
            new_queue.push(queue.get(i));
        }
        *queue = new_queue;
    }

    fn should_insert_before(
        &self,
        new_bet: &Bet<Self::Api>,
        existing_bet: &Bet<Self::Api>,
        is_back: bool
    ) -> bool {
        if is_back {
            new_bet.odd > existing_bet.odd || 
            (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
        } else {
            new_bet.odd < existing_bet.odd || 
            (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
        }
    }

    fn update_status_counters(
        &self,
        scheduler: &mut BetScheduler<Self::Api>,
        old_status: &BetStatus,
        new_status: &BetStatus
    ) {
        // Emit event before update
        self.bet_counter_update_event(
            old_status,
            new_status,
            scheduler.matched_count,
            scheduler.unmatched_count,
            scheduler.partially_matched_count,
            scheduler.win_count,
            scheduler.lost_count,
            scheduler.canceled_count,
        );

        match old_status {
            BetStatus::Matched => scheduler.matched_count = scheduler.matched_count.saturating_sub(1),
            BetStatus::Unmatched => scheduler.unmatched_count = scheduler.unmatched_count.saturating_sub(1),
            BetStatus::PartiallyMatched => scheduler.partially_matched_count = scheduler.partially_matched_count.saturating_sub(1),
            BetStatus::Win => scheduler.win_count = scheduler.win_count.saturating_sub(1),
            BetStatus::Lost => scheduler.lost_count = scheduler.lost_count.saturating_sub(1),
            BetStatus::Canceled => scheduler.canceled_count = scheduler.canceled_count.saturating_sub(1),
        }

        match new_status {
            BetStatus::Matched => scheduler.matched_count += 1,
            BetStatus::Unmatched => scheduler.unmatched_count += 1,
            BetStatus::PartiallyMatched => scheduler.partially_matched_count += 1,
            BetStatus::Win => scheduler.win_count += 1,
            BetStatus::Lost => scheduler.lost_count += 1,
            BetStatus::Canceled => scheduler.canceled_count += 1,
        }

        // Emit event after update
        self.bet_counter_updated_event(
            old_status,
            new_status,
            scheduler.matched_count,
            scheduler.unmatched_count,
            scheduler.partially_matched_count,
            scheduler.win_count,
            scheduler.lost_count,
            scheduler.canceled_count,
        );
    }

    fn update_best_back_odds(&self, scheduler: &mut BetScheduler<Self::Api>) {
        if scheduler.back_bets.is_empty() {
            scheduler.best_back_odds = BigUint::zero();
        } else {
            scheduler.best_back_odds = scheduler.back_bets.get(0).odd.clone();
        }
    }

    fn update_best_lay_odds(&self, scheduler: &mut BetScheduler<Self::Api>) {
        if scheduler.lay_bets.is_empty() {
            scheduler.best_lay_odds = BigUint::zero();
        } else {
            scheduler.best_lay_odds = scheduler.lay_bets.get(0).odd.clone();
        }
    }
}