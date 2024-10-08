use crate::{types::{Bet, BetType, Market}};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode,Clone, ManagedVecItem)]
pub struct BetIndex<> {
    bet_id: u64,
    index: usize,
    bet_type: BetType,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem)]
pub struct PriorityQueue<M: ManagedTypeApi> {
    back_bets: ManagedVec<M, Bet<M>>,
    lay_bets: ManagedVec<M, Bet<M>>,
    best_back_odds: BigUint<M>,
    best_lay_odds: BigUint<M>,
    back_liquidity: BigUint<M>,
    lay_liquidity: BigUint<M>,
    bet_id_to_index: ManagedVec<M, BetIndex<>>,
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
            bet_id_to_index: ManagedVec::new(),
        }
    }

    pub fn add(&mut self, bet: Bet<M>) {
        let index = match bet.bet_type {
            BetType::Back => {
                self.insert_bet(&mut self.back_bets.clone(), bet.clone(), true);
                self.back_liquidity += &bet.stake_amount;
                self.update_best_back_odds();
                self.back_bets.len() - 1
            },
            BetType::Lay => {
                self.insert_bet(&mut self.lay_bets.clone(), bet.clone(), false);
                self.lay_liquidity += &bet.liability;
                self.update_best_lay_odds();
                self.lay_bets.len() - 1
            },
        };
        self.bet_id_to_index.push(BetIndex {
            bet_id: bet.nft_nonce,
            index,
            bet_type: bet.bet_type,
        });
    }

    fn insert_bet(&mut self, queue: &mut ManagedVec<M, Bet<M>>, bet: Bet<M>, is_back: bool) {
        let mut insert_position = queue.len();
        for i in 0..queue.len() {
            if self.should_insert_before(&bet, &queue.get(i), is_back) {
                insert_position = i;
                break;
            }
        }
        queue.push(bet);
        if insert_position < queue.len() - 1 {
            for i in (insert_position + 1..queue.len()).rev() {
                let temp = queue.get(i - 1);
                queue.set(i, &temp);
            }
            queue.set(insert_position, &queue.get(queue.len() - 1));
        }
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

    fn remove_bet_id_index(&mut self, bet_id: u64) {
        for i in 0..self.bet_id_to_index.len() {
            if self.bet_id_to_index.get(i).bet_id == bet_id {
                self.bet_id_to_index.remove(i);
                break;
            }
        }
    }

    pub fn remove(&mut self, bet_id: u64) -> Option<Bet<M>> {
        if let Some((index, bet_type)) = self.find_bet_index(bet_id) {
            let removed_bet = match bet_type {
                BetType::Back => {
                    let bet = self.back_bets.get(index);
                    self.back_liquidity -= &bet.stake_amount;
                    self.back_bets.remove(index);
                    self.update_best_back_odds();
                    bet
                },
                BetType::Lay => {
                    let bet = self.lay_bets.get(index);
                    self.lay_liquidity -= &bet.liability;
                    self.lay_bets.remove(index);
                    self.update_best_lay_odds();
                    bet
                },
            };
            self.remove_bet_id_index(bet_id);
            Some(removed_bet)
        } else {
            None
        }
    }


    fn find_bet_index(&self, bet_id: u64) -> Option<(usize, BetType)> {
        for i in 0..self.bet_id_to_index.len() {
            let bet_index = self.bet_id_to_index.get(i);
            if bet_index.bet_id == bet_id {
                return Some((bet_index.index, bet_index.bet_type));
            }
        }
        None
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
        self.update_back_liquidity();
        self.update_best_back_odds();
        
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
        self.update_lay_liquidity();
        self.update_best_lay_odds();
        
        matching_bets
    }

    fn update_back_liquidity(&mut self) {
        self.back_liquidity = BigUint::zero();
        for i in 0..self.back_bets.len() {
            self.back_liquidity += &self.back_bets.get(i).stake_amount;
        }
    }

    fn update_lay_liquidity(&mut self) {
        self.lay_liquidity = BigUint::zero();
        for i in 0..self.lay_bets.len() {
            self.lay_liquidity += &self.lay_bets.get(i).liability;
        }
    }

    fn update_best_back_odds(&mut self) {
        self.best_back_odds = if self.back_bets.is_empty() {
            BigUint::zero()
        } else {
            self.back_bets.get(0).odd.clone()
        };
    }

    fn update_best_lay_odds(&mut self) {
        self.best_lay_odds = if self.lay_bets.is_empty() {
            BigUint::zero()
        } else {
            self.lay_bets.get(0).odd.clone()
        };
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

    pub fn get_bet_index(&self, bet: &Bet<M>) -> usize {
        match bet.bet_type {
            BetType::Back => {
                for i in 0..self.back_bets.len() {
                    if self.back_bets.get(i).nft_nonce == bet.nft_nonce {
                        return i;
                    }
                }
            },
            BetType::Lay => {
                for i in 0..self.lay_bets.len() {
                    if self.lay_bets.get(i).nft_nonce == bet.nft_nonce {
                        return i;
                    }
                }
            },
        }
        panic!("Bet not found");
    }

    pub fn get_top_n_bets(&self, bet_type: BetType, n: usize) -> ManagedVec<M, Bet<M>> {
        let source = match bet_type {
            BetType::Back => &self.back_bets,
            BetType::Lay => &self.lay_bets,
        };
        let mut result = ManagedVec::new();
        for i in 0..n.min(source.len()) {
            result.push(source.get(i));
        }
        result
    }

    pub fn get_total_bets(&self) -> usize {
        self.back_bets.len() + self.lay_bets.len()
    }

    pub fn bet_exists(&self, bet_id: u64) -> bool {
        for i in 0..self.bet_id_to_index.len() {
            if self.bet_id_to_index.get(i).bet_id == bet_id {
                return true;
            }
        }
        false
    }
}