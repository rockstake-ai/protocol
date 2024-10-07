use crate::{types::{Bet, BetType, Market}};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();
#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct PriorityQueue<M: ManagedTypeApi> {
    back_bets: ManagedVec<M, Bet<M>>,
    lay_bets: ManagedVec<M, Bet<M>>,
    best_back_odds: BigUint<M>,
    best_lay_odds: BigUint<M>,
    back_liquidity: BigUint<M>,
    lay_liquidity: BigUint<M>,
}

impl<M: ManagedTypeApi> PriorityQueue<M> {
    pub fn new() -> Self {
        PriorityQueue {
            back_bets: ManagedVec::new(),
            lay_bets: ManagedVec::new(),
            best_back_odds: BigUint::zero(),
            best_lay_odds: BigUint::from(u64::MAX),
            back_liquidity: BigUint::zero(),
            lay_liquidity: BigUint::zero(),
        }
    }

    pub fn add(&mut self, bet: Bet<M>) {
        match bet.bet_type {
            BetType::Back => self.add_to_queue(&mut self.back_bets, bet, true),
            BetType::Lay => self.add_to_queue(&mut self.lay_bets, bet, false),
        }
        self.update_liquidity_and_odds();
    }

    fn add_to_queue(&mut self, queue: &mut ManagedVec<M, Bet<M>>, bet: Bet<M>, is_back: bool) {
        let mut new_queue = ManagedVec::new();
        let mut inserted = false;
        for existing_bet in queue.iter() {
            if !inserted && self.should_insert_before(&bet, &existing_bet, is_back) {
                new_queue.push(bet.clone());
                inserted = true;
            }
            new_queue.push(existing_bet);
        }
        if !inserted {
            new_queue.push(bet);
        }
        *queue = new_queue;
    }

    pub fn contains(&self, bet_id: u64) -> bool {
        self.back_bets.iter().any(|bet| bet.nft_nonce == bet_id) ||
        self.lay_bets.iter().any(|bet| bet.nft_nonce == bet_id)
    }

    pub fn peek(&self, bet_type: BetType) -> Option<Bet<M>> {
        match bet_type {
            BetType::Back => Some(self.back_bets.get(0)),
            BetType::Lay => Some(self.lay_bets.get(0)),
        }
    }

    pub fn remove(&mut self, bet_id: u64) -> Option<Bet<M>> {
        let mut removed_bet = self.remove_from_queue(&mut self.back_bets, bet_id);
        if removed_bet.is_none() {
            removed_bet = self.remove_from_queue(&mut self.lay_bets, bet_id);
        }
        if removed_bet.is_some() {
            self.update_liquidity_and_odds();
        }
        removed_bet
    }

    fn remove_from_queue(&mut self, queue: &mut ManagedVec<M, Bet<M>>, bet_id: u64) -> Option<Bet<M>> {
        let mut removed_bet = None;
        let mut new_queue = ManagedVec::new();
        for bet in queue.iter() {
            if bet.nft_nonce == bet_id {
                removed_bet = Some(bet.clone());
            } else {
                new_queue.push(bet);
            }
        }
        *queue = new_queue;
        removed_bet
    }

    pub fn size(&self) -> usize {
        self.back_bets.len() + self.lay_bets.len()
    }

    pub fn clear(&mut self) {
        self.back_bets.clear();
        self.lay_bets.clear();
        self.best_back_odds = BigUint::zero();
        self.best_lay_odds = BigUint::from(u64::MAX);
        self.back_liquidity = BigUint::zero();
        self.lay_liquidity = BigUint::zero();
    }

    pub fn get_matching_bets(&mut self, bet_type: &BetType, odds: &BigUint<M>) -> ManagedVec<M, Bet<M>> {
        let source_queue = match bet_type {
            BetType::Back => &mut self.lay_bets,
            BetType::Lay => &mut self.back_bets,
        };

        let mut matching_bets = ManagedVec::new();
        let mut remaining_bets = ManagedVec::new();

        for bet in source_queue.iter() {
            if self.odds_match(bet_type, odds, &bet.odd) {
                matching_bets.push(bet);
            } else {
                remaining_bets.push(bet);
            }
        }

        *source_queue = remaining_bets;
        self.update_liquidity_and_odds();
        matching_bets
    }

    fn should_insert_before(&self, new_bet: &Bet<M>, existing_bet: &Bet<M>, is_back: bool) -> bool {
        if is_back {
            new_bet.odd > existing_bet.odd ||
            (new_bet.odd == existing_bet.odd && new_bet.timestamp < existing_bet.timestamp)
        } else {
            new_bet.odd < existing_bet.odd ||
            (new_bet.odd == existing_bet.odd && new_bet.timestamp < existing_bet.timestamp)
        }
    }

    fn odds_match(&self, bet_type: &BetType, new_odds: &BigUint<M>, existing_odds: &BigUint<M>) -> bool {
        match bet_type {
            BetType::Back => new_odds <= existing_odds,
            BetType::Lay => new_odds >= existing_odds,
        }
    }

    fn update_liquidity_and_odds(&mut self) {
        self.back_liquidity = self.back_bets.iter().fold(BigUint::zero(), |acc, bet| acc + &bet.stake_amount);
        self.lay_liquidity = self.lay_bets.iter().fold(BigUint::zero(), |acc, bet| acc + &bet.liability);

        // Actualizăm cele mai bune cote pentru back
        if !self.back_bets.is_empty() {
            if let best_back_bet = self.back_bets.get(0) {
                self.best_back_odds = best_back_bet.odd.clone();
            }
        } else {
            self.best_back_odds = BigUint::zero();
        }

        if !self.lay_bets.is_empty() {
            if let best_lay_bet = self.lay_bets.get(0) {
                self.best_lay_odds = best_lay_bet.odd.clone();
            }
        } else {
            self.best_lay_odds = BigUint::from(u64::MAX);
        }
    }


    pub fn get_best_back_odds(&self) -> BigUint<M> {
        self.best_back_odds.clone()
    }

    pub fn get_best_lay_odds(&self) -> BigUint<M> {
        self.best_lay_odds.clone()
    }

    pub fn get_back_liquidity(&self) -> BigUint<M> {
        self.back_liquidity.clone()
    }

    pub fn get_lay_liquidity(&self) -> BigUint<M> {
        self.lay_liquidity.clone()
    }
}