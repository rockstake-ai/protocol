use crate::types::{Bet, BetQueueView, BetStatus, BetType, DetailedSchedulerView, Tracker};
use multiversx_sc::codec::multi_types::MultiValue2;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();


#[multiversx_sc::module]
pub trait TrackerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
{
    // 2. INITIALIZATION
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
    ) -> MultiValue6<
        usize,              // back_count
        usize,              // lay_count
        BigUint,            // total_back_liquidity
        BigUint,            // total_lay_liquidity
        ManagedVec<Self::Api, BigUint>,  // back_odds
        ManagedVec<Self::Api, BigUint>   // lay_odds
    > {
        let scheduler = self.selection_scheduler(market_id, selection_id).get();
        
        let mut back_odds = ManagedVec::new();
        let mut lay_odds = ManagedVec::new();
        
        for bet in scheduler.back_bets.iter() {
            back_odds.push(bet.odd);
        }
        
        for bet in scheduler.lay_bets.iter() {
            lay_odds.push(bet.odd);
        }
        
        (
            scheduler.back_bets.len(),
            scheduler.lay_bets.len(),
            scheduler.back_liquidity,
            scheduler.lay_liquidity,
            back_odds,
            lay_odds
        ).into()
    }


fn process_bet(&self, bet: Bet<Self::Api>) -> (BigUint, BigUint, Bet<Self::Api>) {
    let event_id = bet.event;
    let selection_id = bet.selection.selection_id;
    let mut scheduler = self.selection_scheduler(event_id, selection_id).get();
    
    // Debug print pentru a vedea pariul care intră
    self.debug_bet_event(
        &bet.bet_type,
        &bet.odd,
        &bet.unmatched_amount,
        &bet.stake_amount
    );
    
    // Încercăm să găsim matches
    let (matched_amount, unmatched_amount, matching_bets) = self.find_matches(&mut scheduler, &bet);
    
    let mut updated_bet = bet;
    updated_bet.matched_amount = matched_amount.clone();
    updated_bet.unmatched_amount = unmatched_amount.clone();
    
    // Debug print pentru matches găsite
    self.debug_matching_event(
        &matched_amount,
        &unmatched_amount,
        &matching_bets.len()
    );
    
    let new_status = self.determine_status(&updated_bet);
    if updated_bet.status != new_status {
        self.update_status_counters(&mut scheduler, &updated_bet.status, &new_status);
    }
    updated_bet.status = new_status;
    
    // Procesăm matches
    self.process_matches(&mut scheduler, matching_bets);
    
    // Adăugăm partea nematchuită în queue-ul corespunzător
    if unmatched_amount > BigUint::zero() {
        // Debug print înainte de adăugare în queue
        self.debug_queue_event(
            &updated_bet.bet_type,
            &scheduler.back_bets.len(),
            &scheduler.lay_bets.len()
        );
        
        self.add_to_queue(&mut scheduler, &updated_bet);
        
        // Debug print după adăugare
        self.debug_queue_event(
            &updated_bet.bet_type,
            &scheduler.back_bets.len(),
            &scheduler.lay_bets.len()
        );
    }
    
    self.selection_scheduler(event_id, selection_id).set(&scheduler);
    (matched_amount, unmatched_amount, updated_bet)
}

#[event("debug_bet")]
fn debug_bet_event(
    &self,
    #[indexed] bet_type: &BetType,
    #[indexed] odd: &BigUint,
    #[indexed] unmatched: &BigUint,
    #[indexed] stake: &BigUint
);

#[event("debug_matching")]
fn debug_matching_event(
    &self,
    #[indexed] matched: &BigUint,
    #[indexed] unmatched: &BigUint,
    #[indexed] matching_count: &usize
);

#[event("debug_queue")]
fn debug_queue_event(
    &self,
    #[indexed] bet_type: &BetType,
    #[indexed] back_count: &usize,
    #[indexed] lay_count: &usize
);

    // 4. MATCHING LOGIC
    fn find_matches(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        bet: &Bet<Self::Api>
    ) -> (BigUint, BigUint, ManagedVec<Self::Api, Bet<Self::Api>>) {
        let mut matched_amount = BigUint::zero();
        let mut unmatched_amount = self.get_matchable_amount(bet);
        let mut matching_bets = ManagedVec::new();
        
        let opposing_queue = match bet.bet_type {
            BetType::Back => scheduler.lay_bets.clone(),
            BetType::Lay => scheduler.back_bets.clone()
        };
        
        for existing_bet in opposing_queue.iter() {
            if self.can_match(bet, &existing_bet) {
                let match_amount = self.calculate_match_amount(bet, &existing_bet, &unmatched_amount);
                
                if match_amount > BigUint::zero() {
                    matched_amount += &match_amount;
                    unmatched_amount -= &match_amount;
                    
                    let mut updated_matched_bet = existing_bet.clone();
                    self.update_matched_amounts(&mut updated_matched_bet, &match_amount);
                    matching_bets.push(updated_matched_bet);
                    
                    if unmatched_amount == BigUint::zero() {
                        break;
                    }
                }
            }
        }
        
        (matched_amount, unmatched_amount, matching_bets)
    }

    fn process_matches(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        matching_bets: ManagedVec<Self::Api, Bet<Self::Api>>
    ) {
        for matched_bet in matching_bets.iter() {
            // Remove matched bet from its queue
            self.remove_from_queue(scheduler, &matched_bet);
            
            // If partially matched, add back the unmatched portion
            if matched_bet.unmatched_amount > BigUint::zero() {
                self.add_to_queue(scheduler, &matched_bet);
            }
        }
    }

    // 5. QUEUE OPERATIONS
    fn add_to_queue(&self, scheduler: &mut Tracker<Self::Api>, bet: &Bet<Self::Api>) {
        match bet.bet_type {
            BetType::Back => {
                self.insert_ordered(&mut scheduler.back_bets, bet.clone());
                scheduler.back_liquidity += &bet.unmatched_amount;
                self.update_best_back_odds(scheduler);
            },
            BetType::Lay => {
                self.insert_ordered(&mut scheduler.lay_bets, bet.clone());
                scheduler.lay_liquidity += &bet.liability;
                self.update_best_lay_odds(scheduler);
            }
        }
    }

    fn remove_from_queue(&self, scheduler: &mut Tracker<Self::Api>, bet: &Bet<Self::Api>) {
        let queue = match bet.bet_type {
            BetType::Back => &mut scheduler.back_bets,
            BetType::Lay => &mut scheduler.lay_bets,
        };

        if let Some(index) = self.find_bet_index(queue, bet) {
            // Update liquidity before removal
            match bet.bet_type {
                BetType::Back => scheduler.back_liquidity -= &bet.unmatched_amount,
                BetType::Lay => scheduler.lay_liquidity -= &bet.liability,
            }
            
            // Remove bet from queue
            let mut new_queue = ManagedVec::new();
            for i in 0..queue.len() {
                if i != index {
                    new_queue.push(queue.get(i));
                }
            }
            *queue = new_queue;
            
            // Update best odds
            match bet.bet_type {
                BetType::Back => self.update_best_back_odds(scheduler),
                BetType::Lay => self.update_best_lay_odds(scheduler),
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

    fn insert_ordered(&self, queue: &mut ManagedVec<Self::Api, Bet<Self::Api>>, bet: Bet<Self::Api>) {
        let mut insert_index = queue.len();
        
        for i in 0..queue.len() {
            if self.should_insert_before(&bet, &queue.get(i)) {
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

    fn should_insert_before(&self, new_bet: &Bet<Self::Api>, existing_bet: &Bet<Self::Api>) -> bool {
        match new_bet.bet_type {
            BetType::Back => {
                new_bet.odd > existing_bet.odd || 
                (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
            },
            BetType::Lay => {
                new_bet.odd < existing_bet.odd || 
                (new_bet.odd == existing_bet.odd && new_bet.created_at < existing_bet.created_at)
            }
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

    // 7. VIEW FUNCTIONS
    #[view(getSchedulerView)]
    fn get_scheduler_view(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> DetailedSchedulerView<Self::Api> {
        let scheduler = self.get_scheduler_state(market_id, selection_id);
        
        DetailedSchedulerView {
            back_bets_count: scheduler.back_bets.len(),
            lay_bets_count: scheduler.lay_bets.len(),
            total_back_liquidity: scheduler.back_liquidity,
            total_lay_liquidity: scheduler.lay_liquidity,
            best_back_odds: scheduler.best_back_odds,
            best_lay_odds: scheduler.best_lay_odds,
            matched_count: scheduler.matched_count as u32,
            unmatched_count: scheduler.unmatched_count as u32,
            partially_matched_count: scheduler.partially_matched_count as u32,
            back_queue: self.build_queue_view(&scheduler.back_bets),
            lay_queue: self.build_queue_view(&scheduler.lay_bets)
        }
    }

    #[view(getMarketLiquidity)]
    fn get_market_liquidity(
        &self,
        market_id: u64,
        selection_id: u64
    ) -> MultiValue2<BigUint, BigUint> {
        let scheduler = self.get_scheduler_state(market_id, selection_id);
        (scheduler.back_liquidity, scheduler.lay_liquidity).into()
    }

    fn get_scheduler_state(&self, market_id: u64, selection_id: u64) -> Tracker<Self::Api> {
        if self.selection_scheduler(market_id, selection_id).is_empty() {
            return self.init_bet_scheduler();
        }
        self.selection_scheduler(market_id, selection_id).get()
    }

    fn build_queue_view(
        &self,
        queue: &ManagedVec<Self::Api, Bet<Self::Api>>
    ) -> ManagedVec<Self::Api, BetQueueView<Self::Api>> {
        let mut view = ManagedVec::new();
        for bet in queue.iter() {
            view.push(BetQueueView {
                odd: bet.odd,
                amount: bet.unmatched_amount,
                status: bet.status
            });
        }
        view
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