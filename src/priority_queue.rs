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
            best_lay_odds: BigUint::zero(),
            back_liquidity: BigUint::zero(),
            lay_liquidity: BigUint::zero(),
        }
    }

    pub fn add(&mut self, bet: Bet<M>) {
        match bet.bet_type {
            BetType::Back => {
                self.insert_bet_to_queue(&mut self.back_bets.clone(), bet, true);
                self.update_back_liquidity_and_odds();
            },
            BetType::Lay => {
                self.insert_bet_to_queue(&mut self.lay_bets.clone(), bet, false);
                self.update_lay_liquidity_and_odds();
            },
        }
    }

    fn insert_bet_to_queue(&self, queue: &mut ManagedVec<M, Bet<M>>, bet: Bet<M>, is_back: bool) {
        let mut temp_vec = ManagedVec::new();
        let mut inserted = false;

        for i in 0..queue.len() {
            let existing_bet = queue.get(i);
            if !inserted && self.should_insert_before(&bet, &existing_bet, is_back) {
                temp_vec.push(bet.clone());
                inserted = true;
            }
            temp_vec.push(existing_bet);
        }

        if !inserted {
            temp_vec.push(bet);
        }

        *queue = temp_vec;
    }

    pub fn contains(&self, bet_id: u64) -> bool {
        for i in 0..self.back_bets.len() {
            if self.back_bets.get(i).nft_nonce == bet_id {
                return true;
            }
        }
        for i in 0..self.lay_bets.len() {
            if self.lay_bets.get(i).nft_nonce == bet_id {
                return true;
            }
        }
        false
    }

    pub fn peek(&self, bet_type: BetType) -> Option<Bet<M>> {
        match bet_type {
            BetType::Back => if !self.back_bets.is_empty() { Some(self.back_bets.get(0)) } else { None },
            BetType::Lay => if !self.lay_bets.is_empty() { Some(self.lay_bets.get(0)) } else { None },
        }
    }

    pub fn remove(&mut self, bet_id: u64) -> Option<Bet<M>> {
        if let Some(bet) = self.remove_from_queue(&mut self.back_bets.clone(), bet_id) {
            self.update_back_liquidity_and_odds();
            Some(bet)
        } else if let Some(bet) = self.remove_from_queue(&mut self.lay_bets.clone(), bet_id) {
            self.update_lay_liquidity_and_odds();
            Some(bet)
        } else {
            None
        }
    }

    fn remove_from_queue(&self, queue: &mut ManagedVec<M, Bet<M>>, bet_id: u64) -> Option<Bet<M>> {
        let mut temp_vec = ManagedVec::new();
        let mut removed_bet = None;

        for i in 0..queue.len() {
            let bet = queue.get(i);
            if bet.nft_nonce == bet_id {
                removed_bet = Some(bet.clone());
            } else {
                temp_vec.push(bet);
            }
        }

        *queue = temp_vec;
        removed_bet
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

    fn update_back_liquidity_and_odds(&mut self) {
        self.back_liquidity = BigUint::zero();
        for i in 0..self.back_bets.len() {
            let bet = self.back_bets.get(i);
            self.back_liquidity += &bet.stake_amount;
        }
        self.best_back_odds = if !self.back_bets.is_empty() {
            self.back_bets.get(0).odd.clone()
        } else {
            BigUint::zero()
        };
    }

    fn update_lay_liquidity_and_odds(&mut self) {
        self.lay_liquidity = BigUint::zero();
        for i in 0..self.lay_bets.len() {
            let bet = self.lay_bets.get(i);
            self.lay_liquidity += &bet.liability;
        }
        self.best_lay_odds = if !self.lay_bets.is_empty() {
            self.lay_bets.get(0).odd.clone()
        } else {
            BigUint::zero()
        };
    }

    pub fn get_matching_bets(&mut self, bet_type: &BetType, odds: &BigUint<M>) -> ManagedVec<M, Bet<M>> {
        match bet_type {
            BetType::Back => self.get_matching_lay_bets(odds),
            BetType::Lay => self.get_matching_back_bets(odds),
        }
    }

    fn get_matching_back_bets(&mut self, odds: &BigUint<M>) -> ManagedVec<M, Bet<M>> {
        let mut matching_bets = ManagedVec::new();
        let mut remaining_bets = ManagedVec::new();

        for i in 0..self.back_bets.len() {
            let bet = self.back_bets.get(i);
            if odds >= &bet.odd {
                matching_bets.push(bet);
            } else {
                remaining_bets.push(bet);
            }
        }

        self.back_bets = remaining_bets;
        self.update_back_liquidity_and_odds();
        
        matching_bets
    }

    fn get_matching_lay_bets(&mut self, odds: &BigUint<M>) -> ManagedVec<M, Bet<M>> {
        let mut matching_bets = ManagedVec::new();
        let mut remaining_bets = ManagedVec::new();

        for i in 0..self.lay_bets.len() {
            let bet = self.lay_bets.get(i);
            if odds <= &bet.odd {
                matching_bets.push(bet);
            } else {
                remaining_bets.push(bet);
            }
        }

        self.lay_bets = remaining_bets;
        self.update_lay_liquidity_and_odds();
        
        matching_bets
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