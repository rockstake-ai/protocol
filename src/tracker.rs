use crate::types::{Bet, BetStatus, BetType, PriceLevel, Tracker};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait TrackerModule:
    crate::storage::StorageModule +
    crate::events::EventsModule
{
    fn init_bet_scheduler(&self) -> Tracker<Self::Api> {
        Tracker {
            back_levels: ManagedVec::new(),
            lay_levels: ManagedVec::new(),
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

    fn process_bet(&self, bet: Bet<Self::Api>) -> (BigUint, BigUint, Bet<Self::Api>) {
        let event_id = bet.event;
        let selection_id = bet.selection.selection_id;
        let mut scheduler = self.selection_scheduler(event_id, selection_id).get();

        let (matched_amount, unmatched_amount, updated_bet) = self.try_match_bet(&mut scheduler, bet);
        
        self.selection_scheduler(event_id, selection_id).set(&scheduler);
        (matched_amount, unmatched_amount, updated_bet)
    }

    fn try_match_bet(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        bet: Bet<Self::Api>
    ) -> (BigUint, BigUint, Bet<Self::Api>) {
        let mut matched_amount = BigUint::zero();
        let mut remaining_amount = bet.stake_amount.clone();
        let mut updated_bet = bet;

        // Găsim price level-ul potrivit pentru matching
        let levels = match updated_bet.bet_type {
            BetType::Back => &mut scheduler.lay_levels,
            BetType::Lay => &mut scheduler.back_levels,
        };

        let mut i = 0;
        while i < levels.len() && remaining_amount > BigUint::zero() {
            let mut level = levels.get(i);
            
            // Verificăm dacă putem face match la acest nivel de preț
            if self.can_match_at_level(&updated_bet, &level) {
                let match_amount = remaining_amount.clone().min(level.total_stake.clone());
                
                if match_amount > BigUint::zero() {
                    matched_amount += &match_amount;
                    remaining_amount -= &match_amount;
                    
                    // Actualizăm pariurile din acest nivel
                    let mut updated_level_bets = ManagedVec::new();
                    let mut level_matched = BigUint::zero();
                    
                    for mut existing_bet in level.bets.iter() {
                        if level_matched < match_amount && existing_bet.unmatched_amount > BigUint::zero() {
                            let bet_match = (match_amount.clone() - level_matched.clone())
                                .min(existing_bet.unmatched_amount.clone());
                            
                            existing_bet.matched_amount += &bet_match;
                            existing_bet.unmatched_amount -= &bet_match;
                            level_matched += &bet_match;
                            
                            if existing_bet.unmatched_amount > BigUint::zero() {
                                updated_level_bets.push(existing_bet);
                            } else {
                                scheduler.matched_count += 1;
                            }
                        } else {
                            updated_level_bets.push(existing_bet);
                        }
                    }
                    
                    // Actualizăm level-ul
                    level.total_stake -= &match_amount;
                    level.bets = updated_level_bets;
                    
                    if level.total_stake > BigUint::zero() {
                        levels.set(i, &level);
                        i += 1;
                    } else {
                        // Eliminăm level-ul gol
                        if i < levels.len() - 1 {
                            for j in i..levels.len()-1 {
                                let next = levels.get(j + 1);
                                levels.set(j, &next);
                            }
                        }
                        levels.remove(levels.len() - 1);
                    }
                }
            } else {
                i += 1;
            }
        }

        // Actualizăm pariul curent
        updated_bet.matched_amount = matched_amount.clone();
        updated_bet.unmatched_amount = remaining_amount.clone();

        // Adăugăm partea nematchuită în orderbook dacă există
        if remaining_amount > BigUint::zero() {
            self.add_to_orderbook(scheduler, &updated_bet);
            scheduler.unmatched_count += 1;
        }

        // Update status
        if matched_amount > BigUint::zero() {
            if remaining_amount == BigUint::zero() {
                updated_bet.status = BetStatus::Matched;
                scheduler.matched_count += 1;
            } else {
                updated_bet.status = BetStatus::PartiallyMatched;
                scheduler.partially_matched_count += 1;
            }
        }

        (matched_amount, remaining_amount, updated_bet)
    }

    fn can_match_at_level(&self, bet: &Bet<Self::Api>, level: &PriceLevel<Self::Api>) -> bool {
        match bet.bet_type {
            BetType::Back => bet.odd >= level.odds,
            BetType::Lay => bet.odd <= level.odds,
        }
    }

    fn add_to_orderbook(
        &self,
        scheduler: &mut Tracker<Self::Api>,
        bet: &Bet<Self::Api>
    ) {
        let levels = match bet.bet_type {
            BetType::Back => &mut scheduler.back_levels,
            BetType::Lay => &mut scheduler.lay_levels,
        };

        // Căutăm nivel de preț existent sau poziția pentru inserare
        let mut found_level = false;
        for i in 0..levels.len() {
            let mut level = levels.get(i);
            if level.odds == bet.odd {
                // Adăugăm la nivel existent
                level.total_stake += &bet.unmatched_amount;
                level.bets.push(bet.clone());
                levels.set(i, &level);
                found_level = true;
                break;
            }
        }

        if !found_level {
            // Creăm nivel nou
            let mut new_level = PriceLevel {
                odds: bet.odd.clone(),
                total_stake: bet.unmatched_amount.clone(),
                bets: ManagedVec::new(),
            };
            new_level.bets.push(bet.clone());

            // Găsim poziția corectă pentru inserare
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

            // Inserăm level-ul nou
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

        // Update liquidity
        match bet.bet_type {
            BetType::Back => scheduler.back_liquidity += &bet.unmatched_amount,
            BetType::Lay => scheduler.lay_liquidity += &bet.unmatched_amount,
        }
    }
}