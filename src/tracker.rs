// use crate::types::{Bet, BetStatus, BetType, QueueInspectView, StatusCounts, Tracker};
// multiversx_sc::imports!();
// multiversx_sc::derive_imports!();

// #[multiversx_sc::module]
// pub trait TrackerModule:
//     crate::storage::StorageModule +
//     crate::events::EventsModule
// {
//     fn init_bet_scheduler(&self) -> Tracker<Self::Api> {
//         Tracker {
//             back_bets: ManagedVec::new(),
//             lay_bets: ManagedVec::new(),
//             best_back_odds: BigUint::zero(),
//             best_lay_odds: BigUint::zero(),
//             back_liquidity: BigUint::zero(),
//             lay_liquidity: BigUint::zero(),
//             matched_count: 0,
//             unmatched_count: 0,
//             partially_matched_count: 0,
//             win_count: 0,
//             lost_count: 0,
//             canceled_count: 0,
//         }
//     }

//     #[view(inspectQueues)]
//     fn inspect_queues(
//         &self,
//         market_id: u64,
//         selection_id: u64
//     ) -> QueueInspectView<Self::Api> {
//         let scheduler = self.selection_scheduler(market_id, selection_id).get();
        
//         let mut back_odds = ManagedVec::new();
//         let mut lay_odds = ManagedVec::new();
        
//         for bet in scheduler.back_bets.iter() {
//             back_odds.push(bet.odd);
//         }
        
//         for bet in scheduler.lay_bets.iter() {
//             lay_odds.push(bet.odd);
//         }
        
//         QueueInspectView {
//             back_count: scheduler.back_bets.len(),
//             lay_count: scheduler.lay_bets.len(),
//             back_liquidity: scheduler.back_liquidity,
//             lay_liquidity: scheduler.lay_liquidity,
//             back_odds,
//             lay_odds,
//             status_counts: StatusCounts {
//                 matched: scheduler.matched_count,
//                 unmatched: scheduler.unmatched_count,
//                 partially_matched: scheduler.partially_matched_count,
//                 win: scheduler.win_count,
//                 lost: scheduler.lost_count,
//                 canceled: scheduler.canceled_count
//             }
//         }
//     }

//     fn process_bet(&self, bet: Bet<Self::Api>) -> (BigUint, BigUint, Bet<Self::Api>) {
//         let event_id = bet.event;
//         let selection_id = bet.selection.selection_id;
//         let mut scheduler = self.selection_scheduler(event_id, selection_id).get();

//         // Încercăm să găsim matches
//         let (matched_amount, unmatched_amount, matching_bets) = self.find_matches(&mut scheduler, &bet);
        
//         let mut updated_bet = bet;
//         updated_bet.matched_amount = matched_amount.clone();
//         updated_bet.unmatched_amount = unmatched_amount.clone();

//         let new_status = self.determine_status(&updated_bet);
//         if updated_bet.status != new_status {
//             self.update_status_counters(&mut scheduler, &updated_bet.status, &new_status);
//         }
//         updated_bet.status = new_status;
        
//         // Procesăm matches
//         self.process_matches(&mut scheduler, matching_bets);
        
//         // Adăugăm partea nematchuită în queue-ul corespunzător
//         if unmatched_amount > BigUint::zero() {
//             self.add_to_queue(&mut scheduler, &updated_bet);
//         }
//         self.selection_scheduler(event_id, selection_id).set(&scheduler);
//         (matched_amount, unmatched_amount, updated_bet)
//     }

//     fn can_match(&self, bet: &Bet<Self::Api>, existing_bet: &Bet<Self::Api>) -> bool {
//         match bet.bet_type {
//             BetType::Back => {
//                 // Un Back se poate matcha DOAR cu un Lay la aceeași cotă
//                 bet.odd == existing_bet.odd && 
//                 existing_bet.unmatched_amount > BigUint::zero()
//             },
//             BetType::Lay => {
//                 // Un Lay se poate matcha DOAR cu un Back la aceeași cotă
//                 bet.odd == existing_bet.odd && 
//                 existing_bet.unmatched_amount > BigUint::zero()
//             }
//         }
//     }
    
//     // Helper pentru calculul lichidității
//     fn calculate_queue_liquidity(&self, queue: &ManagedVec<Self::Api, Bet<Self::Api>>) -> BigUint {
//         let mut total = BigUint::zero();
//         for bet in queue.iter() {
//             total += &bet.unmatched_amount;
//         }
//         total
//     }
    
//     // Funcția care determină statusul unui pariu
//     fn determine_status(&self, bet: &Bet<Self::Api>) -> BetStatus {
//         let total = self.get_matchable_amount(bet);
//         if bet.matched_amount == total {
//             BetStatus::Matched
//         } else if bet.matched_amount > BigUint::zero() {
//             BetStatus::PartiallyMatched
//         } else {
//             BetStatus::Unmatched
//         }
//     }

//     fn find_matches(
//         &self,
//         scheduler: &mut Tracker<Self::Api>,
//         bet: &Bet<Self::Api>
//     ) -> (BigUint, BigUint, ManagedVec<Self::Api, Bet<Self::Api>>) {
//         let mut matched_amount = BigUint::zero();
//         let mut unmatched_amount = bet.stake_amount.clone();
//         let mut matching_bets = ManagedVec::new();
        
//         let queue = match bet.bet_type {
//             BetType::Back => &mut scheduler.lay_bets,
//             BetType::Lay => &mut scheduler.back_bets,
//         };
        
//         if queue.is_empty() {
//             return (BigUint::zero(), bet.stake_amount.clone(), matching_bets);
//         }
    
//         // Găsim primul match valid (cotă egală)
//         let mut matched_indices = ArrayVec::<usize, 5>::new(); // Limitam la max 5 matches pentru a economisi gas
//         let mut i = 0;
        
//         while i < queue.len() && matched_indices.len() < 5 {
//             let existing_bet = queue.get(i);
//             if self.can_match(bet, &existing_bet) {
//                 matched_indices.push(i);
//             }
//             i += 1;
//         }
    
//         if matched_indices.is_empty() {
//             return (BigUint::zero(), bet.stake_amount.clone(), matching_bets);
//         }
    
//         // Procesăm matches găsite
//         for &index in matched_indices.iter() {
//             let mut existing_bet = queue.get(index);
//             let match_amount = self.calculate_match_amount(bet, &existing_bet, &unmatched_amount);
            
//             if match_amount > BigUint::zero() {
//                 matched_amount += &match_amount;
//                 unmatched_amount -= &match_amount;
                
//                 self.update_matched_amounts(&mut existing_bet, &match_amount);
//                 matching_bets.push(existing_bet.clone());
                
//                 // Actualizăm pariul în queue dacă mai are sumă nematchuită
//                 if existing_bet.unmatched_amount > BigUint::zero() {
//                     queue.set(index, &existing_bet);
//                 }
                
//                 if unmatched_amount == BigUint::zero() {
//                     break;
//                 }
//             }
//         }
    
//         // Eliminăm pariurile matchuite complet într-o singură parcurgere
//         let mut write_index = 0;
//         for read_index in 0..queue.len() {
//             let current_bet = queue.get(read_index);
//             if current_bet.unmatched_amount > BigUint::zero() {
//                 if write_index != read_index {
//                     queue.set(write_index, &current_bet);
//                 }
//                 write_index += 1;
//             }
//         }
    
//         // Ajustăm dimensiunea queue-ului o singură dată
//         while queue.len() > write_index {
//             queue.remove(queue.len() - 1);
//         }
    
//         // Actualizăm lichiditatea o singură dată
//         match bet.bet_type {
//             BetType::Back => scheduler.lay_liquidity = self.calculate_queue_liquidity(queue),
//             BetType::Lay => scheduler.back_liquidity = self.calculate_queue_liquidity(queue),
//         }
    
//         (matched_amount, unmatched_amount, matching_bets)
//     }
    
//     // Funcție optimizată pentru procesarea matches
//     fn process_matches(
//         &self,
//         scheduler: &mut Tracker<Self::Api>,
//         matching_bets: ManagedVec<Self::Api, Bet<Self::Api>>
//     ) {
//         // Nu mai creăm vectori temporari
//         // Nu mai facem remove și add separate
//         // Actualizăm direct statusurile și contoarele
//         for matched_bet in matching_bets.iter() {
//             if matched_bet.unmatched_amount > BigUint::zero() {
//                 // Actualizăm doar contoarele pentru pariurile parțial matchuite
//                 if matched_bet.status != BetStatus::PartiallyMatched {
//                     self.update_status_counters(scheduler, &matched_bet.status, &BetStatus::PartiallyMatched);
//                 }
//             } else {
//                 // Pentru pariurile complet matchuite, actualizăm doar o dată statusul
//                 if matched_bet.status != BetStatus::Matched {
//                     self.update_status_counters(scheduler, &matched_bet.status, &BetStatus::Matched);
//                 }
//             }
//         }
//     }

//     fn add_to_queue(&self, scheduler: &mut Tracker<Self::Api>, bet: &Bet<Self::Api>) {
//         // Un pariu nou care intră în queue trebuie să fie contorizat ca unmatched
//         if bet.status == BetStatus::Unmatched {
//             scheduler.unmatched_count += 1;
//         }
    
//         match bet.bet_type {
//             BetType::Back => {
//                 self.insert_ordered(&mut scheduler.back_bets, bet.clone());
//                 scheduler.back_liquidity += &bet.stake_amount;
//                 self.update_best_back_odds(scheduler);
//             },
//             BetType::Lay => {
//                 let lay_bet = bet.clone();
//                 self.insert_ordered(&mut scheduler.lay_bets, lay_bet);
//                 scheduler.lay_liquidity += &bet.stake_amount;
//                 self.update_best_lay_odds(scheduler);
//             }
//         }
//     }
    
//     fn remove_from_queue(&self, scheduler: &mut Tracker<Self::Api>, bet: &Bet<Self::Api>) {
//         // Actualizăm contoarele
//         match bet.status {
//             BetStatus::Unmatched => {
//                 if scheduler.unmatched_count > 0 {
//                     scheduler.unmatched_count -= 1;
//                 }
//             },
//             BetStatus::PartiallyMatched => {
//                 if scheduler.partially_matched_count > 0 {
//                     scheduler.partially_matched_count -= 1;
//                 }
//             },
//             _ => {}
//         }
    
//         let queue = match bet.bet_type {
//             BetType::Back => &mut scheduler.back_bets,
//             BetType::Lay => &mut scheduler.lay_bets,
//         };
    
//         if let Some(index) = self.find_bet_index(queue, bet) {
//             // Actualizăm lichiditatea
//             match bet.bet_type {
//                 BetType::Back => {
//                     scheduler.back_liquidity -= &bet.stake_amount;
//                 },
//                 BetType::Lay => {
//                     scheduler.lay_liquidity -= &bet.stake_amount;
//                 }
//             }
            
//             // Mutăm elementele cu o poziție înapoi pentru a umple golul
//             for i in index..queue.len()-1 {
//                 let next = queue.get(i + 1);
//                 queue.set(i, &next);
//             }
            
//             // Eliminăm ultimul element (care acum e duplicat)
//             queue.remove(queue.len() - 1);
            
//             // Actualizăm best odds
//             match bet.bet_type {
//                 BetType::Back => self.update_best_back_odds(scheduler),
//                 BetType::Lay => self.update_best_lay_odds(scheduler)
//             }
//         }
//     }

    
//     fn insert_ordered(&self, queue: &mut ManagedVec<Self::Api, Bet<Self::Api>>, bet: Bet<Self::Api>) {
//         // Găsim poziția corectă pentru inserare
//         let mut insert_index = queue.len();
//         for i in 0..queue.len() {
//             if self.should_insert_before(&bet, &queue.get(i)) {
//                 insert_index = i;
//                 break;
//             }
//         }
        
//         // Dacă inserăm la final, pur și simplu adăugăm
//         if insert_index == queue.len() {
//             queue.push(bet);
//             return;
//         }
        
//         // Dacă inserăm în altă parte, mai întâi facem loc
//         queue.push(queue.get(queue.len() - 1)); // Duplicăm ultimul element temporar
        
//         // Mutăm elementele cu o poziție mai în spate, începând de la final
//         for i in (insert_index..queue.len()-1).rev() {
//             let temp = queue.get(i);
//             queue.set(i + 1, &temp);
//         }
        
//         // Punem noul pariu la poziția corectă
//         queue.set(insert_index, &bet);
//     }
    
//     fn should_insert_before(&self, new_bet: &Bet<Self::Api>, existing_bet: &Bet<Self::Api>) -> bool {
//         match new_bet.bet_type {
//             BetType::Back => {
//                 // Pentru Back, cotele mai mari au prioritate
//                 new_bet.odd > existing_bet.odd || 
//                 (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
//             },
//             BetType::Lay => {
//                 // Pentru Lay, cotele mai mici au prioritate
//                 new_bet.odd < existing_bet.odd || 
//                 (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
//             }
//         }
//     }

//     fn calculate_match_amount(
//         &self,
//         bet: &Bet<Self::Api>,
//         existing_bet: &Bet<Self::Api>,
//         unmatched_amount: &BigUint,
//     ) -> BigUint {
//         match bet.bet_type {
//             BetType::Back => unmatched_amount.clone().min(existing_bet.unmatched_amount.clone()),
//             BetType::Lay => unmatched_amount.clone().min(existing_bet.stake_amount.clone()),
//         }
//     }

//     fn update_matched_amounts(&self, bet: &mut Bet<Self::Api>, match_amount: &BigUint) {
//         bet.matched_amount += match_amount;
//         bet.unmatched_amount -= match_amount;
        
//         // Update liability for lay bets
//         if bet.bet_type == BetType::Lay {
//             bet.liability = match_amount * &(bet.odd.clone() - BigUint::from(1u32));
//         }
//     }

//     fn get_matchable_amount(&self, bet: &Bet<Self::Api>) -> BigUint {
//         match bet.bet_type {
//             BetType::Back => bet.stake_amount.clone(),
//             BetType::Lay => bet.liability.clone()
//         }
//     }

//     fn find_bet_index(
//         &self,
//         queue: &ManagedVec<Self::Api, Bet<Self::Api>>,
//         bet: &Bet<Self::Api>
//     ) -> Option<usize> {
//         for i in 0..queue.len() {
//             if queue.get(i).nft_nonce == bet.nft_nonce {
//                 return Some(i);
//             }
//         }
//         None
//     }

//     fn update_best_back_odds(&self, scheduler: &mut Tracker<Self::Api>) {
//         scheduler.best_back_odds = if scheduler.back_bets.is_empty() {
//             BigUint::zero()
//         } else {
//             scheduler.back_bets.get(0).odd.clone()
//         };
//     }

//     fn update_best_lay_odds(&self, scheduler: &mut Tracker<Self::Api>) {
//         scheduler.best_lay_odds = if scheduler.lay_bets.is_empty() {
//             BigUint::zero()
//         } else {
//             scheduler.lay_bets.get(0).odd.clone()
//         };
//     }

//     fn update_status_counters(
//         &self,
//         scheduler: &mut Tracker<Self::Api>,
//         old_status: &BetStatus,
//         new_status: &BetStatus
//     ) {
//         // Decrementăm contorul vechi doar dacă statusul s-a schimbat
//         if old_status != new_status {
//             match old_status {
//                 BetStatus::Matched => {
//                     if scheduler.matched_count > 0 {
//                         scheduler.matched_count -= 1;
//                     }
//                 },
//                 BetStatus::Unmatched => {
//                     if scheduler.unmatched_count > 0 {
//                         scheduler.unmatched_count -= 1;
//                     }
//                 },
//                 BetStatus::PartiallyMatched => {
//                     if scheduler.partially_matched_count > 0 {
//                         scheduler.partially_matched_count -= 1;
//                     }
//                 },
//                 BetStatus::Win => {
//                     if scheduler.win_count > 0 {
//                         scheduler.win_count -= 1;
//                     }
//                 },
//                 BetStatus::Lost => {
//                     if scheduler.lost_count > 0 {
//                         scheduler.lost_count -= 1;
//                     }
//                 },
//                 BetStatus::Canceled => {
//                     if scheduler.canceled_count > 0 {
//                         scheduler.canceled_count -= 1;
//                     }
//                 },
//             }
    
//             // Incrementăm noul contor
//             match new_status {
//                 BetStatus::Matched => scheduler.matched_count += 1,
//                 BetStatus::Unmatched => scheduler.unmatched_count += 1,
//                 BetStatus::PartiallyMatched => scheduler.partially_matched_count += 1,
//                 BetStatus::Win => scheduler.win_count += 1,
//                 BetStatus::Lost => scheduler.lost_count += 1,
//                 BetStatus::Canceled => scheduler.canceled_count += 1,
//             }
//         } else if *old_status == BetStatus::Unmatched && *new_status == BetStatus::Unmatched {
//             // Caz special pentru pariul inițial - incrementăm unmatchedCount
//             scheduler.unmatched_count += 1;
//         }
    
//         // Emitem evenimentul doar dacă e vreo schimbare
//         if old_status != new_status || (*old_status == BetStatus::Unmatched && *new_status == BetStatus::Unmatched) {
//             self.bet_counter_update_event(
//                 old_status,
//                 new_status,
//                 scheduler.matched_count as u64,
//                 scheduler.unmatched_count as u64,
//                 scheduler.partially_matched_count as u64,
//                 scheduler.win_count as u64,
//                 scheduler.lost_count as u64,
//                 scheduler.canceled_count as u64,
//             );
//         }
//     }
// }

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
            matched_count: 0u64,
            unmatched_count: 0u64,
            partially_matched_count: 0u64,
            win_count: 0u64,
            lost_count: 0u64,
            canceled_count: 0u64,
        }
    }

    // View function pentru inspecția stării
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

    // Funcția principală de procesare a pariurilor
    fn process_bet(&self, bet: Bet<Self::Api>) -> (BigUint, BigUint, Bet<Self::Api>) {
        let event_id = bet.event;
        let selection_id = bet.selection.selection_id;
        let mut scheduler = self.selection_scheduler(event_id, selection_id).get();

        let (matched_amount, unmatched_amount, matching_bets) = self.find_matches(&mut scheduler, &bet);
        
        let mut updated_bet = bet;
        updated_bet.matched_amount = matched_amount.clone();
        updated_bet.unmatched_amount = unmatched_amount.clone();

        // Update status și contoare într-o singură operație
        self.update_bet_status(&mut scheduler, &mut updated_bet);
        
        // Actualizăm queue-urile și procesăm matches
        if !matching_bets.is_empty() {
            self.update_queues(&mut scheduler, &matching_bets);
        }
        
        // Adăugăm partea nematchuită în queue dacă există
        if unmatched_amount > BigUint::zero() {
            self.add_to_queue(&mut scheduler, &updated_bet);
        }

        self.selection_scheduler(event_id, selection_id).set(&scheduler);
        (matched_amount, unmatched_amount, updated_bet)
    }

    // Helper pentru actualizarea statusului și contoarelor
    fn update_bet_status(&self, scheduler: &mut Tracker<Self::Api>, bet: &mut Bet<Self::Api>) {
        let new_status = if bet.matched_amount == bet.stake_amount {
            BetStatus::Matched
        } else if bet.matched_amount > BigUint::zero() {
            BetStatus::PartiallyMatched
        } else {
            BetStatus::Unmatched
        };

        if bet.status != new_status {
            match bet.status {
                BetStatus::Unmatched => scheduler.unmatched_count -= 1,
                BetStatus::PartiallyMatched => scheduler.partially_matched_count -= 1,
                BetStatus::Matched => scheduler.matched_count -= 1,
                _ => {}
            }

            match new_status {
                BetStatus::Unmatched => scheduler.unmatched_count += 1,
                BetStatus::PartiallyMatched => scheduler.partially_matched_count += 1,
                BetStatus::Matched => scheduler.matched_count += 1,
                _ => {}
            }
        }

        bet.status = new_status;
    }

    fn find_matches(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        bet: &Bet<Self::Api>
    ) -> (BigUint, BigUint, ManagedVec<Self::Api, Bet<Self::Api>>) {
        let mut matched_amount = BigUint::zero();
        let mut unmatched_amount = bet.stake_amount.clone();
        let mut matching_bets = ManagedVec::new();
        
        // Determinăm queue-ul corect și facem update-uri într-o singură parcurgere
        let (queue, queue_type) = match bet.bet_type {
            BetType::Back => (&mut scheduler.lay_bets, BetType::Lay),
            BetType::Lay => (&mut scheduler.back_bets, BetType::Back),
        };
        
        if queue.is_empty() {
            return (matched_amount, unmatched_amount, matching_bets);
        }

        let mut write_idx = 0;
        let queue_len = queue.len();
        
        // O singură parcurgere pentru procesarea tuturor matches
        for read_idx in 0..queue_len {
            let existing_bet = queue.get(read_idx);
            
            if existing_bet.odd != bet.odd || existing_bet.unmatched_amount == BigUint::zero() {
                if write_idx != read_idx {
                    queue.set(write_idx, &existing_bet);
                }
                write_idx += 1;
                continue;
            }

            let match_amount = unmatched_amount.clone().min(existing_bet.unmatched_amount.clone());
            if match_amount > BigUint::zero() {
                matched_amount += &match_amount;
                unmatched_amount -= &match_amount;
                
                let mut updated_bet = existing_bet;
                updated_bet.matched_amount += &match_amount;
                updated_bet.unmatched_amount -= &match_amount;
                
                matching_bets.push(updated_bet.clone());
                
                if updated_bet.unmatched_amount > BigUint::zero() {
                    queue.set(write_idx, &updated_bet);
                    write_idx += 1;
                }
                
                if unmatched_amount == BigUint::zero() {
                    // Copiem restul pariurilor nematchuite
                    for i in (read_idx + 1)..queue_len {
                        queue.set(write_idx, &queue.get(i));
                        write_idx += 1;
                    }
                    break;
                }
            }
        }

        // Ajustăm dimensiunea queue-ului
        while queue.len() > write_idx {
            queue.remove(queue.len() - 1);
        }

        // Actualizăm lichiditatea o singură dată la final
        let mut total = BigUint::zero();
        for bet in queue.iter() {
            total += &bet.unmatched_amount;
        }
        
        match queue_type {
            BetType::Back => scheduler.back_liquidity = total,
            BetType::Lay => scheduler.lay_liquidity = total,
        };
        
        (matched_amount, unmatched_amount, matching_bets)
    }

    fn add_to_queue(&self, scheduler: &mut Tracker<Self::Api>, bet: &Bet<Self::Api>) {
        if bet.status == BetStatus::Unmatched {
            scheduler.unmatched_count += 1;
        }

        let bet_type = bet.bet_type;  // Copiem bet_type pentru a evita borrowing issues
        let queue = match bet_type {
            BetType::Back => &mut scheduler.back_bets,
            BetType::Lay => &mut scheduler.lay_bets,
        };

        self.insert_ordered(queue, bet.clone());

        // Calculăm și actualizăm lichiditatea
        let mut total = BigUint::zero();
        for existing_bet in queue.iter() {
            total += &existing_bet.unmatched_amount;
        }

        match bet_type {
            BetType::Back => {
                scheduler.back_liquidity = total;
                if scheduler.best_back_odds == BigUint::zero() || bet.odd > scheduler.best_back_odds {
                    scheduler.best_back_odds = bet.odd.clone();
                }
            },
            BetType::Lay => {
                scheduler.lay_liquidity = total;
                if scheduler.best_lay_odds == BigUint::zero() || bet.odd < scheduler.best_lay_odds {
                    scheduler.best_lay_odds = bet.odd.clone();
                }
            }
        }
    }


    fn should_insert_before(&self, new_bet: &Bet<Self::Api>, existing_bet: &Bet<Self::Api>) -> bool {
        match new_bet.bet_type {
            BetType::Back => {
                // Pentru Back, cotele mai mari au prioritate (DESCRESCĂTOR)
                new_bet.odd > existing_bet.odd || 
                (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
            },
            BetType::Lay => {
                // Pentru Lay, cotele mai mici au prioritate (CRESCĂTOR)
                new_bet.odd < existing_bet.odd || 
                (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
            }
        }
    }

    fn should_add_at_end(&self, bet: &Bet<Self::Api>, queue: &ManagedVec<Self::Api, Bet<Self::Api>>) -> bool {
        if queue.is_empty() {
            return true;
        }
        let last_bet = queue.get(queue.len() - 1);
        match bet.bet_type {
            BetType::Back => bet.odd <= last_bet.odd,
            BetType::Lay => bet.odd >= last_bet.odd,
        }
    }

    // Helper pentru actualizarea lichidității
    fn update_liquidity(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        queue: &ManagedVec<Self::Api, Bet<Self::Api>>,
        bet_type: BetType
    ) {
        let mut total = BigUint::zero();
        for bet in queue.iter() {
            total += &bet.unmatched_amount;
        }
        
        match bet_type {
            BetType::Back => scheduler.back_liquidity = total,
            BetType::Lay => scheduler.lay_liquidity = total,
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

    fn update_queues(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        matching_bets: &ManagedVec<Self::Api, Bet<Self::Api>>
    ) {
        for matched_bet in matching_bets.iter() {
            let queue = match matched_bet.bet_type {
                BetType::Back => &mut scheduler.back_bets,
                BetType::Lay => &mut scheduler.lay_bets,
            };

            // Găsim și eliminăm pariul complet matchuit
            if matched_bet.unmatched_amount == BigUint::zero() {
                if let Some(index) = self.find_bet_index(queue, &matched_bet) {
                    // Mutăm toate elementele cu o poziție înapoi
                    for i in index..queue.len()-1 {
                        let next = queue.get(i + 1);
                        queue.set(i, &next);
                    }
                    queue.remove(queue.len() - 1);
                }
            } else {
                // Actualizăm pariul parțial matchuit
                if let Some(index) = self.find_bet_index(queue, &matched_bet) {
                    queue.set(index, &matched_bet);
                }
            }

            // Actualizăm best odds și lichiditate
            match matched_bet.bet_type {
                BetType::Back => {
                    self.update_best_back_odds(scheduler);
                    self.update_back_liquidity(scheduler);
                },
                BetType::Lay => {
                    self.update_best_lay_odds(scheduler);
                    self.update_lay_liquidity(scheduler);
                }
            }
        }
    }

    fn update_back_liquidity(&self, scheduler: &mut Tracker<Self::Api>) {
        let mut total = BigUint::zero();
        for bet in scheduler.back_bets.iter() {
            total += &bet.unmatched_amount;
        }
        scheduler.back_liquidity = total;
    }

    fn update_lay_liquidity(&self, scheduler: &mut Tracker<Self::Api>) {
        let mut total = BigUint::zero();
        for bet in scheduler.lay_bets.iter() {
            total += &bet.unmatched_amount;
        }
        scheduler.lay_liquidity = total;
    }

    fn update_best_back_odds(&self, scheduler: &mut Tracker<Self::Api>) {
        scheduler.best_back_odds = if scheduler.back_bets.is_empty() {
            BigUint::zero()
        } else {
            // Pentru Back, cea mai mare cotă este cea mai bună
            let mut best = scheduler.back_bets.get(0).odd.clone();
            for bet in scheduler.back_bets.iter() {
                if bet.odd > best {
                    best = bet.odd.clone();
                }
            }
            best
        };
    }

    fn update_best_lay_odds(&self, scheduler: &mut Tracker<Self::Api>) {
        scheduler.best_lay_odds = if scheduler.lay_bets.is_empty() {
            BigUint::zero()
        } else {
            // Pentru Lay, cea mai mică cotă este cea mai bună
            let mut best = scheduler.lay_bets.get(0).odd.clone();
            for bet in scheduler.lay_bets.iter() {
                if bet.odd < best {
                    best = bet.odd.clone();
                }
            }
            best
        };
    }

    fn insert_ordered(&self, queue: &mut ManagedVec<Self::Api, Bet<Self::Api>>, bet: Bet<Self::Api>) {
        // Dacă queue-ul e gol, doar adăugăm
        if queue.is_empty() {
            queue.push(bet);
            return;
        }

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
}