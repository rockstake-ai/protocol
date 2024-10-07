use crate::{errors::ERR_INVALID_STREAM, storage::{Bet, BetType}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct PriorityQueue<M: ManagedTypeApi> {
    bets: ManagedVec<M, Bet<M>>,
}

impl<M: ManagedTypeApi> PriorityQueue<M> {
    pub fn new() -> Self {
        PriorityQueue {
            bets: ManagedVec::new(),
        }
    }

    pub fn add(&mut self, bet: Bet<M>) {
        let mut new_bets = ManagedVec::new();
        let mut inserted = false;

        for existing_bet in self.bets.iter() {
            if !inserted && should_insert_before(&bet, &existing_bet) {
                new_bets.push(bet.clone());
                inserted = true;
            }
            new_bets.push(existing_bet);
        }

        if !inserted {
            new_bets.push(bet);
        }

        self.bets = new_bets;
    }

    pub fn contains(&self, bet_id: u64) -> bool {
        self.bets.iter().any(|bet| bet.nft_nonce == bet_id)
    }

    pub fn peek(&self) -> Option<Bet<M>> {
        Some(self.bets.get(0))
    }

    pub fn remove(&mut self, bet_id: u64) -> Option<Bet<M>> {
        let mut removed_bet = None;
        let mut new_bets = ManagedVec::new();

        for bet in self.bets.iter() {
            if bet.nft_nonce == bet_id {
                removed_bet = Some(bet.clone());
            } else {
                new_bets.push(bet);
            }
        }

        self.bets = new_bets;
        removed_bet
    }

    pub fn size(&self) -> usize {
        self.bets.len()
    }

    pub fn clear(&mut self) {
        self.bets.clear();
    }

    pub fn get_matching_bets(&mut self, bet_type: &BetType, odds: &BigUint<M>) -> ManagedVec<M, Bet<M>> {
        let mut matching_bets = ManagedVec::new();
        let mut remaining_bets = ManagedVec::new();

        for bet in self.bets.iter() {
            if bet.bet_type != *bet_type && odds_match(bet_type, odds, &bet.odd) {
                matching_bets.push(bet);
            } else {
                remaining_bets.push(bet);
            }
        }

        self.bets = remaining_bets;
        matching_bets
    }
}

fn should_insert_before<M: ManagedTypeApi>(new_bet: &Bet<M>, existing_bet: &Bet<M>) -> bool {
    match new_bet.bet_type {
        BetType::Back => {
            new_bet.odd > existing_bet.odd || 
            (new_bet.odd == existing_bet.odd && new_bet.timestamp < existing_bet.timestamp)
        },
        BetType::Lay => {
            new_bet.odd < existing_bet.odd || 
            (new_bet.odd == existing_bet.odd && new_bet.timestamp < existing_bet.timestamp)
        }
    }
}

fn odds_match<M: ManagedTypeApi>(bet_type: &BetType, new_odds: &BigUint<M>, existing_odds: &BigUint<M>) -> bool {
    match bet_type {
        BetType::Back => new_odds <= existing_odds,
        BetType::Lay => new_odds >= existing_odds,
    }
}