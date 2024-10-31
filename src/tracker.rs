use crate::types::{Bet, BetStatus, BetType, QueueInspectView, StatusCounts, Tracker};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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

    #[view(inspectQueues)]
    fn inspect_queues(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> QueueInspectView<Self::Api> {
        let scheduler = self.selection_scheduler(market_id, selection_id).get();
        
        let mut back_odds = ManagedVec::new();
        let mut lay_odds = ManagedVec::new();
        
        for bet in scheduler.back_bets.iter() {
            back_odds.push(bet.odd);
        }
        
        for bet in scheduler.lay_bets.iter() {
            lay_odds.push(bet.odd);
        }
        
        QueueInspectView {
            back_count: scheduler.back_bets.len(),
            lay_count: scheduler.lay_bets.len(),
            back_liquidity: scheduler.back_liquidity,
            lay_liquidity: scheduler.lay_liquidity,
            back_odds,
            lay_odds,
            status_counts: StatusCounts {
                matched: scheduler.matched_count,
                unmatched: scheduler.unmatched_count,
                partially_matched: scheduler.partially_matched_count,
                win: scheduler.win_count,
                lost: scheduler.lost_count,
                canceled: scheduler.canceled_count
            }
        }
    }

    fn process_bet(&self, bet: Bet<Self::Api>) -> (BigUint, BigUint, Bet<Self::Api>) {
        let event_id = bet.event;
        let selection_id = bet.selection.selection_id;
        let mut scheduler = self.selection_scheduler(event_id, selection_id).get();

        // Încercăm să găsim matches
        let (matched_amount, unmatched_amount, matching_bets) = self.find_matches(&mut scheduler, &bet);
        
        let mut updated_bet = bet;
        updated_bet.matched_amount = matched_amount.clone();
        updated_bet.unmatched_amount = unmatched_amount.clone();

        let new_status = self.determine_status(&updated_bet);
        if updated_bet.status != new_status {
            self.update_status_counters(&mut scheduler, &updated_bet.status, &new_status);
        }
        updated_bet.status = new_status;
        
        // Procesăm matches
        self.process_matches(&mut scheduler, matching_bets);
        
        // Adăugăm partea nematchuită în queue-ul corespunzător
        if unmatched_amount > BigUint::zero() {
            self.add_to_queue(&mut scheduler, &updated_bet);
        }
        self.selection_scheduler(event_id, selection_id).set(&scheduler);
        (matched_amount, unmatched_amount, updated_bet)
    }

    fn find_matches(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        bet: &Bet<Self::Api>
    ) -> (BigUint, BigUint, ManagedVec<Self::Api, Bet<Self::Api>>) {
        let mut matched_amount = BigUint::zero();
        let mut unmatched_amount = bet.stake_amount.clone();
        let mut matching_bets = ManagedVec::new();
        
        // În loc să clonam queue-ul, lucrăm direct cu referințe
        let queue_to_check = match bet.bet_type {
            BetType::Back => &mut scheduler.lay_bets,
            BetType::Lay => &mut scheduler.back_bets,
        };
        
        // Creăm un nou queue pentru pariurile rămase nematchuite
        let mut new_queue = ManagedVec::new();
        
        for existing_bet in queue_to_check.iter() {
            if self.can_match(bet, &existing_bet) {
                let match_amount = self.calculate_match_amount(bet, &existing_bet, &unmatched_amount);
                
                if match_amount > BigUint::zero() {
                    matched_amount += &match_amount;
                    unmatched_amount -= &match_amount;
                    
                    let mut updated_bet = existing_bet.clone();
                    self.update_matched_amounts(&mut updated_bet, &match_amount);
                    
                    // Dacă mai rămâne ceva nematchuit, îl păstrăm în queue
                    if updated_bet.unmatched_amount > BigUint::zero() {
                        new_queue.push(updated_bet.clone());
                    }
                    
                    matching_bets.push(updated_bet);
                    
                    if unmatched_amount == BigUint::zero() {
                        break;
                    }
                }
            } else {
                new_queue.push(existing_bet);
            }
        }
        
        // Actualizăm queue-ul în scheduler
        match bet.bet_type {
            BetType::Back => scheduler.lay_bets = new_queue,
            BetType::Lay => scheduler.back_bets = new_queue,
        }
        
        (matched_amount, unmatched_amount, matching_bets)
    }

    fn process_matches(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        matching_bets: ManagedVec<Self::Api, Bet<Self::Api>>
    ) {
        let mut to_readd: ManagedVec<Self::Api, Bet<Self::Api>> = ManagedVec::new();
        
        for matched_bet in matching_bets.iter() {
            self.remove_from_queue(scheduler, &matched_bet);
            
            if matched_bet.unmatched_amount > BigUint::zero() {
                to_readd.push(matched_bet);
            }
        }
        
        // Adăugăm înapoi pariurile parțial matchuite într-un singur batch
        for bet in to_readd.iter() {
            self.add_to_queue(scheduler, &bet);
        }
    }

    fn add_to_queue(&self, scheduler: &mut Tracker<Self::Api>, bet: &Bet<Self::Api>) {
        // Un pariu nou care intră în queue trebuie să fie contorizat ca unmatched
        if bet.status == BetStatus::Unmatched {
            scheduler.unmatched_count += 1;
        }
    
        match bet.bet_type {
            BetType::Back => {
                self.insert_ordered(&mut scheduler.back_bets, bet.clone());
                scheduler.back_liquidity += &bet.stake_amount;
                self.update_best_back_odds(scheduler);
            },
            BetType::Lay => {
                let lay_bet = bet.clone();
                self.insert_ordered(&mut scheduler.lay_bets, lay_bet);
                scheduler.lay_liquidity += &bet.stake_amount;
                self.update_best_lay_odds(scheduler);
            }
        }
    }
    
    fn remove_from_queue(&self, scheduler: &mut Tracker<Self::Api>, bet: &Bet<Self::Api>) {
        // Actualizăm contoarele
        match bet.status {
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
            _ => {}
        }
    
        let queue = match bet.bet_type {
            BetType::Back => &mut scheduler.back_bets,
            BetType::Lay => &mut scheduler.lay_bets,
        };
    
        if let Some(index) = self.find_bet_index(queue, bet) {
            // Actualizăm lichiditatea
            match bet.bet_type {
                BetType::Back => {
                    scheduler.back_liquidity -= &bet.stake_amount;
                },
                BetType::Lay => {
                    scheduler.lay_liquidity -= &bet.stake_amount;
                }
            }
            
            // Mutăm elementele cu o poziție înapoi pentru a umple golul
            for i in index..queue.len()-1 {
                let next = queue.get(i + 1);
                queue.set(i, &next);
            }
            
            // Eliminăm ultimul element (care acum e duplicat)
            queue.remove(queue.len() - 1);
            
            // Actualizăm best odds
            match bet.bet_type {
                BetType::Back => self.update_best_back_odds(scheduler),
                BetType::Lay => self.update_best_lay_odds(scheduler)
            }
        }
    }

    
    fn insert_ordered(&self, queue: &mut ManagedVec<Self::Api, Bet<Self::Api>>, bet: Bet<Self::Api>) {
        // Găsim poziția corectă pentru inserare
        let mut insert_index = queue.len();
        for i in 0..queue.len() {
            if self.should_insert_before(&bet, &queue.get(i)) {
                insert_index = i;
                break;
            }
        }
        
        // Dacă inserăm la final, pur și simplu adăugăm
        if insert_index == queue.len() {
            queue.push(bet);
            return;
        }
        
        // Dacă inserăm în altă parte, mai întâi facem loc
        queue.push(queue.get(queue.len() - 1)); // Duplicăm ultimul element temporar
        
        // Mutăm elementele cu o poziție mai în spate, începând de la final
        for i in (insert_index..queue.len()-1).rev() {
            let temp = queue.get(i);
            queue.set(i + 1, &temp);
        }
        
        // Punem noul pariu la poziția corectă
        queue.set(insert_index, &bet);
    }
    
    fn should_insert_before(&self, new_bet: &Bet<Self::Api>, existing_bet: &Bet<Self::Api>) -> bool {
        match new_bet.bet_type {
            BetType::Back => {
                // Pentru Back, cotele mai mari au prioritate
                new_bet.odd > existing_bet.odd || 
                (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
            },
            BetType::Lay => {
                // Pentru Lay, cotele mai mici au prioritate
                new_bet.odd < existing_bet.odd || 
                (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
            }
        }
    }

    // 6. HELPER METHODS
    fn can_match(&self, bet: &Bet<Self::Api>, existing_bet: &Bet<Self::Api>) -> bool {
        match bet.bet_type {
            BetType::Back => {
                bet.odd >= existing_bet.odd && 
                existing_bet.unmatched_amount > BigUint::zero()
            },
            BetType::Lay => {
                bet.odd <= existing_bet.odd && 
                existing_bet.stake_amount > BigUint::zero()
            }
        }
    }

    fn calculate_match_amount(
        &self,
        bet: &Bet<Self::Api>,
        existing_bet: &Bet<Self::Api>,
        unmatched_amount: &BigUint,
    ) -> BigUint {
        match bet.bet_type {
            BetType::Back => unmatched_amount.clone().min(existing_bet.unmatched_amount.clone()),
            BetType::Lay => unmatched_amount.clone().min(existing_bet.stake_amount.clone()),
        }
    }

    fn update_matched_amounts(&self, bet: &mut Bet<Self::Api>, match_amount: &BigUint) {
        bet.matched_amount += match_amount;
        bet.unmatched_amount -= match_amount;
        
        // Update liability for lay bets
        if bet.bet_type == BetType::Lay {
            bet.liability = match_amount * &(bet.odd.clone() - BigUint::from(1u32));
        }
    }

    fn get_matchable_amount(&self, bet: &Bet<Self::Api>) -> BigUint {
        match bet.bet_type {
            BetType::Back => bet.stake_amount.clone(),
            BetType::Lay => bet.liability.clone()
        }
    }

    fn determine_status(&self, bet: &Bet<Self::Api>) -> BetStatus {
        let total = self.get_matchable_amount(bet);
        if bet.matched_amount == total {
            BetStatus::Matched
        } else if bet.matched_amount > BigUint::zero() {
            BetStatus::PartiallyMatched
        } else {
            BetStatus::Unmatched
        }
    }

    fn find_bet_index(
        &self,
        queue: &ManagedVec<Self::Api, Bet<Self::Api>>,
        bet: &Bet<Self::Api>
    ) -> Option<usize> {
        for i in 0..queue.len() {
            if queue.get(i).nft_nonce == bet.nft_nonce {
                return Some(i);
            }
        }
        None
    }

    fn update_best_back_odds(&self, scheduler: &mut Tracker<Self::Api>) {
        scheduler.best_back_odds = if scheduler.back_bets.is_empty() {
            BigUint::zero()
        } else {
            scheduler.back_bets.get(0).odd.clone()
        };
    }

    fn update_best_lay_odds(&self, scheduler: &mut Tracker<Self::Api>) {
        scheduler.best_lay_odds = if scheduler.lay_bets.is_empty() {
            BigUint::zero()
        } else {
            scheduler.lay_bets.get(0).odd.clone()
        };
    }

    fn update_status_counters(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        old_status: &BetStatus,
        new_status: &BetStatus
    ) {
        // Decrementăm contorul vechi doar dacă statusul s-a schimbat
        if old_status != new_status {
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
    
            // Incrementăm noul contor
            match new_status {
                BetStatus::Matched => scheduler.matched_count += 1,
                BetStatus::Unmatched => scheduler.unmatched_count += 1,
                BetStatus::PartiallyMatched => scheduler.partially_matched_count += 1,
                BetStatus::Win => scheduler.win_count += 1,
                BetStatus::Lost => scheduler.lost_count += 1,
                BetStatus::Canceled => scheduler.canceled_count += 1,
            }
        } else if *old_status == BetStatus::Unmatched && *new_status == BetStatus::Unmatched {
            // Caz special pentru pariul inițial - incrementăm unmatchedCount
            scheduler.unmatched_count += 1;
        }
    
        // Emitem evenimentul doar dacă e vreo schimbare
        if old_status != new_status || (*old_status == BetStatus::Unmatched && *new_status == BetStatus::Unmatched) {
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
    }
}