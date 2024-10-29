use multiversx_sc::codec::multi_types::MultiValue6;
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::types::{Bet, BetStatus, BetType, SchedulerDebugView, Tracker};

#[multiversx_sc::module]
pub trait TrackerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
    {

    fn init_bet_scheduler(&self) -> Tracker<Self::Api> {
        Tracker {
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

    fn match_bet(&self, bet: Bet<Self::Api>) -> (BigUint, BigUint, Bet<Self::Api>) {
        let mut scheduler = self.selection_scheduler(
            bet.event,
            bet.selection.selection_id
        ).get();
        
        let old_status = bet.status.clone();
        let (matching_bets, matched_amount, unmatched_amount) = self.get_matching_bets(bet.clone());
        
        let mut updated_bet = bet;
        updated_bet.matched_amount = matched_amount.clone();
        updated_bet.unmatched_amount = unmatched_amount.clone();
        
        let new_status = self.calculate_bet_status(&updated_bet, &matched_amount);
    
        if old_status != new_status {
            self.update_status_counters(&mut scheduler, &old_status, &new_status);
        }
        updated_bet.status = new_status;
    
        self.process_matching_bets(&mut scheduler, matching_bets);
    
        if updated_bet.unmatched_amount > BigUint::zero() {
            self.add_to_scheduler(&mut scheduler, updated_bet.clone());
        }
    
        // Salvăm starea actualizată
        self.selection_scheduler(
            updated_bet.event,
            updated_bet.selection.selection_id
        ).set(&scheduler);
        
        (matched_amount, unmatched_amount, updated_bet)
    }

    fn add_to_scheduler(&self, scheduler: &mut Tracker<Self::Api>, bet: Bet<Self::Api>) {
        let old_status = bet.status.clone();
        let mut new_bet = bet;
        new_bet.status = BetStatus::Unmatched;
        
        self.update_status_counters(scheduler, &old_status, &new_bet.status);

        match new_bet.bet_type {
            BetType::Back => {
                let mut queue = scheduler.back_bets.clone();
                self.insert_bet(&mut queue, new_bet.clone());
                scheduler.back_bets = queue;
                scheduler.back_liquidity += &new_bet.stake_amount;
                self.update_best_back_odds(scheduler);
            },
            BetType::Lay => {
                let mut queue = scheduler.lay_bets.clone();
                self.insert_bet(&mut queue, new_bet.clone());
                scheduler.lay_bets = queue;
                scheduler.lay_liquidity += &new_bet.liability;
                self.update_best_lay_odds(scheduler);
            },
        };
    }



    fn add(&self, bet: Bet<Self::Api>) {
        let selection_id = bet.selection.selection_id;
        let event = bet.event;
        let mut scheduler = self.selection_scheduler(
            event,
            selection_id,
        ).get();
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
        self.selection_scheduler(event.clone(), selection_id.clone()).set(&scheduler);
    }

    fn remove(&self, bet: Bet<Self::Api>) -> Option<Bet<Self::Api>> {
        let mut scheduler = self.selection_scheduler(
            bet.event,
            bet.selection.selection_id
        ).get();
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
            self.selection_scheduler(bet.event, bet.selection.selection_id).set(&scheduler);
            Some(removed_bet)
        } else {
            None
        }
    }

    fn get_matching_bets(
        &self,
        bet: Bet<Self::Api>
    ) -> (ManagedVec<Self::Api, Bet<Self::Api>>, BigUint, BigUint) {
        let scheduler = self.selection_scheduler(
            bet.event,
            bet.selection.selection_id
        ).get();

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
            let is_match = self.is_matching_bet(&bet, &existing_bet);
    
            if is_match {
                let match_amount = self.calculate_match_amount(&bet, &existing_bet, &unmatched_amount);
    
                matched_amount += &match_amount;
                unmatched_amount -= &match_amount;
    
                let mut updated_bet = existing_bet.clone();
                self.update_matched_bet(&mut updated_bet, &match_amount, &bet);
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
            // Pentru back, cotele mai mari au prioritate
            if new_bet.odd > existing_bet.odd {
                return true;
            } else if new_bet.odd < existing_bet.odd {
                return false;
            }
        } else {
            // Pentru lay, cotele mai mici au prioritate
            if new_bet.odd < existing_bet.odd {
                return true;
            } else if new_bet.odd > existing_bet.odd {
                return false;
            }
        }
        
        // La cote egale, folosim FIFO
        new_bet.created_at < existing_bet.created_at
    }

    fn is_matching_bet(&self, bet: &Bet<Self::Api>, existing_bet: &Bet<Self::Api>) -> bool {
        match bet.bet_type {
            BetType::Back => bet.odd >= existing_bet.odd,
            BetType::Lay => bet.odd <= existing_bet.odd,
        }
    }

    fn calculate_match_amount(
        &self,
        bet: &Bet<Self::Api>,
        existing_bet: &Bet<Self::Api>,
        unmatched_amount: &BigUint,
    ) -> BigUint {
        if bet.bet_type == BetType::Back {
            unmatched_amount.clone().min(existing_bet.unmatched_amount.clone())
        } else {
            unmatched_amount.clone().min(existing_bet.stake_amount.clone())
        }
    }

    fn calculate_bet_status(
        &self,
        bet: &Bet<Self::Api>,
        matched_amount: &BigUint
    ) -> BetStatus {
        match bet.bet_type {
            BetType::Back => {
                if matched_amount == &bet.stake_amount {
                    BetStatus::Matched
                } else if matched_amount > &BigUint::zero() {
                    BetStatus::PartiallyMatched
                } else {
                    BetStatus::Unmatched
                }
            },
            BetType::Lay => {
                if matched_amount == &bet.liability {
                    BetStatus::Matched
                } else if matched_amount > &BigUint::zero() {
                    BetStatus::PartiallyMatched
                } else {
                    BetStatus::Unmatched
                }
            }
        }
    }

    fn update_matched_bet(
        &self,
        bet: &mut Bet<Self::Api>,
        match_amount: &BigUint,
        matching_bet: &Bet<Self::Api>
    ) {
        bet.matched_amount += match_amount;
        bet.unmatched_amount -= match_amount;
        
        if matching_bet.bet_type == BetType::Lay {
            bet.liability = match_amount * &(matching_bet.odd.clone() - BigUint::from(1u32));
        }
    }

    fn process_matching_bets(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        matching_bets: ManagedVec<Self::Api, Bet<Self::Api>>
    ) {
        for matched_bet in matching_bets.iter() {
            let old_matched_status = matched_bet.status.clone();
            self.remove(matched_bet.clone());
            
            let mut updated_matched_bet = matched_bet;
            let new_matched_status = self.calculate_bet_status(
                &updated_matched_bet,
                &updated_matched_bet.matched_amount
            );
    
            if old_matched_status != new_matched_status {
                self.update_status_counters(scheduler, &old_matched_status, &new_matched_status);
            }
            updated_matched_bet.status = new_matched_status;
    
            if updated_matched_bet.unmatched_amount > BigUint::zero() {
                self.add(updated_matched_bet);
            }
        }
    }

    fn update_best_back_odds(&self, scheduler: &mut Tracker<Self::Api>) {
        if scheduler.back_bets.is_empty() {
            scheduler.best_back_odds = BigUint::zero();
        } else {
            scheduler.best_back_odds = scheduler.back_bets.get(0).odd.clone();
        }
    }

    fn update_best_lay_odds(&self, scheduler: &mut Tracker<Self::Api>) {
        if scheduler.lay_bets.is_empty() {
            scheduler.best_lay_odds = BigUint::zero();
        } else {
            scheduler.best_lay_odds = scheduler.lay_bets.get(0).odd.clone();
        }
    }

    fn update_status_counters(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        old_status: &BetStatus,
        new_status: &BetStatus
    ) {
        // Primul pariu sau schimbare de status
        if *old_status == BetStatus::Unmatched && *new_status == BetStatus::Unmatched {
            // Este un pariu nou, doar incrementăm unmatched
            scheduler.unmatched_count += 1;
        } else {
            // Pentru orice altă tranziție
            match old_status {
                BetStatus::Matched => {
                    if scheduler.matched_count > 0 {
                        scheduler.matched_count -= 1;
                    }
                },
                BetStatus::Unmatched => {
                    if scheduler.unmatched_count > 0 {
                        scheduler.unmatched_count -= 1;
                    }
                },
                BetStatus::PartiallyMatched => {
                    if scheduler.partially_matched_count > 0 {
                        scheduler.partially_matched_count -= 1;
                    }
                },
                BetStatus::Win => {
                    if scheduler.win_count > 0 {
                        scheduler.win_count -= 1;
                    }
                },
                BetStatus::Lost => {
                    if scheduler.lost_count > 0 {
                        scheduler.lost_count -= 1;
                    }
                },
                BetStatus::Canceled => {
                    if scheduler.canceled_count > 0 {
                        scheduler.canceled_count -= 1;
                    }
                },
            }
    
            // Incrementăm noul status
            match new_status {
                BetStatus::Matched => scheduler.matched_count += 1,
                BetStatus::Unmatched => scheduler.unmatched_count += 1,
                BetStatus::PartiallyMatched => scheduler.partially_matched_count += 1,
                BetStatus::Win => scheduler.win_count += 1,
                BetStatus::Lost => scheduler.lost_count += 1,
                BetStatus::Canceled => scheduler.canceled_count += 1,
            }
        }
    
        // Emitem evenimentul cu noile valori
        self.bet_counter_update_event(
            old_status,
            new_status,
            scheduler.matched_count as u64,
            scheduler.unmatched_count as u64,
            scheduler.partially_matched_count as u64,
            scheduler.win_count as u64,
            scheduler.lost_count as u64,
            scheduler.canceled_count as u64,
        );
    }
    
    // Adaugă această metodă pentru debugging
    #[view(getCurrentSchedulerState)]
    fn get_current_scheduler_state(&self) -> Tracker<Self::Api> {
        self.bet_scheduler().get()
    }
    
    // Modifică get_bet_counts pentru a include debugging info
    #[view(getBetCounts)]
    fn get_bet_counts(
        &self, 
        market_id: u64, 
        selection_id: u64
    ) -> MultiValue6<BigUint, BigUint, BigUint, BigUint, BigUint, BigUint> {
        let scheduler = self.get_scheduler_state(market_id, selection_id);
        
        // Debug event pentru a vedea valorile exacte
        self.bet_counter_debug_event(
            &scheduler.matched_count,
            &scheduler.unmatched_count,
            &scheduler.partially_matched_count,
            &scheduler.win_count,
            &scheduler.lost_count,
            &scheduler.canceled_count
        );
        
        (
            BigUint::from(scheduler.matched_count),
            BigUint::from(scheduler.unmatched_count),
            BigUint::from(scheduler.partially_matched_count),
            BigUint::from(scheduler.win_count),
            BigUint::from(scheduler.lost_count),
            BigUint::from(scheduler.canceled_count)
        ).into()
    }

    #[view(getSchedulerState)]
    fn get_scheduler_state(&self, market_id: u64, selection_id: u64) -> Tracker<Self::Api> {
        if self.selection_scheduler(market_id, selection_id).is_empty() {
            return self.init_bet_scheduler();
        }
        self.selection_scheduler(market_id, selection_id).get()
    }

    #[view(getMarketLiquidity)]
    fn get_market_liquidity(&self, market_id: u64, selection_id: u64) -> MultiValue2<BigUint, BigUint> {
        let scheduler = self.get_scheduler_state(market_id, selection_id);
        (scheduler.back_liquidity, scheduler.lay_liquidity).into()
    }

    #[view(getSchedulerDebugState)]
    fn get_scheduler_debug_state(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> SchedulerDebugView<Self::Api> {
        let scheduler = self.get_scheduler_state(market_id, selection_id);
        
        SchedulerDebugView {
            back_bets_count: scheduler.back_bets.len(),
            lay_bets_count: scheduler.lay_bets.len(),
            best_back_odds: scheduler.best_back_odds,
            best_lay_odds: scheduler.best_lay_odds,
            back_liquidity: scheduler.back_liquidity,
            lay_liquidity: scheduler.lay_liquidity,
            matched_count: scheduler.matched_count as u32,
            unmatched_count: scheduler.unmatched_count as u32,
            partially_matched_count: scheduler.partially_matched_count as u32,
            win_count: scheduler.win_count as u32,
            lost_count: scheduler.lost_count as u32,
            canceled_count: scheduler.canceled_count as u32
        }
    }
}


// #[endpoint(updateBetStatus)]
// fn update_bet_status(
//     &self,
//     market_id: u64,
//     selection_id: u64,
//     bet_id: u64,
//     new_status: BetStatus
// ) -> SCResult<()> {
//     let mut market = self.markets(&market_id).get();
//     let selection_index = market
//         .selections
//         .iter()
//         .position(|s| s.selection_id == selection_id)
//         .ok_or("Selection not found")?;

//     let mut selection = market.selections.get(selection_index);
//     let mut scheduler = selection.priority_queue.clone();
    
//     let bet = self.bet_by_id(bet_id).get();
//     let old_status = bet.status.clone();
    
//     self.update_status_counters(&mut scheduler, &old_status, &new_status);
    
//     selection.priority_queue = scheduler;
//     let _ = market.selections.set(selection_index, &selection);
//     self.markets(&market_id).set(&market);

//     self.bet_status_updated_event(
//         market_id,
//         selection_id,
//         bet_id,
//         &old_status,
//         &new_status
//     );

//     Ok(())
// }