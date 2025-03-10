use crate::{errors::{ERR_BET_ALREADY_CLAIMED, ERR_BET_NOT_WON, ERR_INVALID_MARKET, ERR_MARKET_NOT_CLOSED, ERR_NOT_BET_OWNER, ERR_NO_MARKETS_FOUND}, types::{BetStatus, BetType, MarketStatus, MarketType, Sport, Tracker}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FundModule:
    crate::storage::StorageModule
    + crate::events::EventsModule
    + crate::nft::NftModule
    + crate::orderbook::OrderbookModule
{
    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Market Handling -------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Handles an expired market by closing it and processing unmatched bets.
    /// Parameters:
    /// - sport: The type of sport (e.g., Football, Basketball).
    /// - event_id: The unique ID of the event.
    /// - market_id: The ID of the market to handle.
    fn handle_expired_market(&self, sport: Sport, event_id: u64, market_id: u64) {
        let market_ids = self.markets_by_event_and_sport(sport, event_id).get();
        require!(
            market_ids.contains(&market_id),
           ERR_INVALID_MARKET
        );

        let mut market = self.markets(market_id).get();
        market.market_status = MarketStatus::Closed;
        self.markets(market_id).set(&market);
        
        for selection in market.selections.iter() {
            let back_levels = self.selection_back_levels(market_id, selection.id).get();
            for level in back_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.return_unmatched_amount(bet_nonce);
                }
            }
    
            let lay_levels = self.selection_lay_levels(market_id, selection.id).get();
            for level in lay_levels.iter() {
                for bet_nonce in level.bet_nonces.iter() {
                    self.return_unmatched_amount(bet_nonce);
                }
            }
    
            self.selection_back_levels(market_id, selection.id).set(&ManagedVec::new());
            self.selection_lay_levels(market_id, selection.id).set(&ManagedVec::new());
            self.selection_back_liquidity(market_id, selection.id).set(&BigUint::zero());
            self.selection_lay_liquidity(market_id, selection.id).set(&BigUint::zero());
    
            let tracker = Tracker {
                back_levels: ManagedVec::new(),
                lay_levels: ManagedVec::new(),
                back_liquidity: BigUint::zero(),
                lay_liquidity: BigUint::zero(),
                matched_count: self.selection_matched_count(market_id, selection.id).get(),
                unmatched_count: self.selection_unmatched_count(market_id, selection.id).get(),
                partially_matched_count: self.selection_partially_matched_count(market_id, selection.id).get(),
                win_count: self.selection_win_count(market_id, selection.id).get(),
                lost_count: self.selection_lost_count(market_id, selection.id).get(),
                canceled_count: self.selection_canceled_count(market_id, selection.id).get(),
            };
            
            self.selection_tracker(market_id, selection.id).set(&tracker);
        }
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Bet Processing --------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Returns unmatched amounts to the bettor for a specific bet.
    /// Parameters:
    /// - bet_nonce: The unique identifier (nonce) of the bet.
    fn return_unmatched_amount(&self, bet_nonce: u64) {
        let mut bet = self.bet_by_id(bet_nonce).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
    
        if unmatched > BigUint::zero() {
            let original_stake_amount = bet.stake_amount.clone();
            let refund_amount = match bet.bet_type {
                BetType::Back => unmatched.clone(),
                BetType::Lay => {
                    let unmatched_ratio = (&unmatched * &BigUint::from(100u64)) / &original_stake_amount;
                    (&bet.total_amount * &unmatched_ratio) / &BigUint::from(100u64)
                }
            };
    
            self.locked_funds(&bet.bettor).update(|funds| {
                if *funds >= refund_amount {
                    *funds -= &refund_amount;
                } else {
                    *funds = BigUint::zero();
                }
            });
            self.send().direct(&bet.bettor, &bet.payment_token, bet.payment_nonce, &refund_amount);
    
            if bet.total_matched > BigUint::zero() {
                bet.stake_amount = bet.total_matched.clone();
                bet.total_amount = if bet.bet_type == BetType::Lay {
                    let matched_liability = (&bet.liability * &bet.total_matched) / &original_stake_amount;
                    &bet.total_matched + &matched_liability
                } else {
                    bet.total_matched.clone()
                };
                bet.potential_profit = self.calculate_total_potential_profit(&bet);
                bet.status = BetStatus::Matched;
                self.bet_by_id(bet_nonce).set(&bet);
            } else {
                self.bet_by_id(bet_nonce).clear();
            }
        }
    }

    /// Processes all unmatched bets for an event across all markets.
    /// Parameters:
    /// - sport: The type of sport.
    /// - event_id: The unique ID of the event.
    fn process_unmatched_bets(&self, sport: Sport, event_id: u64) {
        let market_ids = self.markets_by_event_and_sport(sport, event_id).get();
        for market_id in market_ids.iter() {
            let market = self.markets(market_id).get();
            
            for selection in market.selections.iter() {
                let back_levels = self.selection_back_levels(market_id, selection.id).get();
                for level in back_levels.iter() {
                    for bet_nonce in level.bet_nonces.iter() {
                        self.process_unmatched_bet(bet_nonce);
                    }
                }

                let lay_levels = self.selection_lay_levels(market_id, selection.id).get();
                for level in lay_levels.iter() {
                    for bet_nonce in level.bet_nonces.iter() {
                        self.process_unmatched_bet(bet_nonce);
                    }
                }

                self.selection_back_liquidity(market_id, selection.id).set(&BigUint::zero());
                self.selection_lay_liquidity(market_id, selection.id).set(&BigUint::zero());
            }
        }
    }

    /// Processes a single unmatched bet, refunding the unmatched amount.
    /// Parameters:
    /// - bet_nonce: The unique identifier (nonce) of the bet.
    fn process_unmatched_bet(&self, bet_nonce: u64) {
        let mut bet = self.bet_by_id(bet_nonce).get();
        let unmatched = &bet.stake_amount - &bet.total_matched;
    
        if unmatched > BigUint::zero() {
            let refund_amount = match bet.bet_type {
                BetType::Back => unmatched.clone(),
                BetType::Lay => {
                    let unmatched_ratio = (&unmatched * &BigUint::from(100u64)) / &bet.stake_amount;
                    (&bet.total_amount * &unmatched_ratio) / &BigUint::from(100u64)
                }
            };
    
            self.send().direct(
                &bet.bettor,
                &bet.payment_token,
                bet.payment_nonce,
                &refund_amount,
            );
    
            if bet.total_matched > BigUint::zero() {
                bet.stake_amount = bet.total_matched.clone();
                bet.total_amount = if bet.bet_type == BetType::Lay {
                    let matched_ratio = &bet.total_matched / &bet.stake_amount;
                    &bet.total_amount * &matched_ratio
                } else {
                    bet.total_matched.clone()
                };
                bet.status = BetStatus::Matched;
                bet.potential_profit = self.calculate_total_potential_profit(&bet); 
                self.bet_by_id(bet_nonce).set(&bet);
            } else {
                self.bet_by_id(bet_nonce).clear();
            }
        }
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Event Settlement ------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Sets the result of an event and settles the associated markets (only owner).
    /// Parameters:
    /// - sport: The type of sport.
    /// - event_id: The unique ID of the event.
    /// - score_home: The score of the home team.
    /// - score_away: The score of the away team.
    #[only_owner]
    #[endpoint(setEventScore)]
    fn set_event_score(
        &self,
        sport: Sport,
        event_id: u64,
        score_home: u32,
        score_away: u32
    ) {
        self.event_score(event_id).set(&(score_home, score_away));
        
        let market_ids = self.markets_by_event_and_sport(sport, event_id).get();
        require!(
            !market_ids.is_empty(),
            ERR_NO_MARKETS_FOUND
        );
        
        for market_id in market_ids.iter() {
            let mut market = self.markets(market_id).get();
            
            require!(
                market.market_status == MarketStatus::Closed,
                ERR_MARKET_NOT_CLOSED
            );
            
            let winning_selection = self.determine_winner(sport, market.market_type, score_home, score_away, event_id);
            
            self.winning_selection(market_id).set(winning_selection);
            
            market.market_status = MarketStatus::Settled;
            self.markets(market_id).set(&market);
            
            self.mark_bets_win_loss(sport, market_id, winning_selection);
        }
    }

    /// Marks bets as won or lost based on the winning selection.
    /// Parameters:
    /// - sport: The type of sport.
    /// - market_id: The ID of the market.
    /// - winning_selection: The ID of the winning selection.
    fn mark_bets_win_loss(
        &self,
        _sport: Sport,        
        market_id: u64,
        winning_selection: u64,
    ) {
        let bet_ids = self.market_bet_ids(market_id);
        
        for bet_id in bet_ids.iter() {
            if !self.bet_by_id(bet_id).is_empty() {
                let mut bet = self.bet_by_id(bet_id).get();
                
                if bet.status == BetStatus::Matched {
                    let is_winner = match bet.bet_type {
                        BetType::Back => bet.selection.id == winning_selection,
                        BetType::Lay => bet.selection.id != winning_selection
                    };
                    
                    bet.status = if is_winner {
                        BetStatus::Win
                    } else {
                        BetStatus::Lost
                    };
                    
                    if is_winner {
                        self.selection_win_count(market_id, bet.selection.id)
                            .update(|count| *count += 1);
                    } else {
                        self.selection_lost_count(market_id, bet.selection.id)
                            .update(|count| *count += 1);
                    }
                    
                    self.bet_by_id(bet_id).set(&bet);
                }
            }
        }
    }

    /// Allows a bettor to claim win from a winning bet.
    /// Parameters:
    /// - bet_id: The ID of the bet to claim winnings for.
    #[payable("*")]
    #[endpoint(claimWin)]
    fn claim_win(&self, bet_id: u64) {
        let caller = self.blockchain().get_caller();
        let (token_identifier, payment_nonce, amount) = self
            .call_value()
            .egld_or_single_esdt()
            .into_tuple();
        let token_identifier_wrap = token_identifier.unwrap_esdt();

        let mut bet = self.bet_by_id(bet_id).get();
        require!(bet.bettor == caller, "Only the bet owner can claim the win");
        require!(bet.status != BetStatus::Claimed, "Bet already claimed");
        require!(bet.status == BetStatus::Win, "Bet must be in Won state to claim");

        require!(
            token_identifier_wrap == self.bet_nft_token().get_token_id(),
            "Must send the bet NFT to claim"
        );
        require!(
            payment_nonce == bet.nft_nonce,
            "Invalid NFT nonce"
        );
        require!(
            amount == BigUint::from(1u64),
            "Must send exactly 1 NFT"
        );

        let payout = match bet.bet_type {
            BetType::Back => &bet.stake_amount + &bet.potential_profit,
            BetType::Lay => &bet.total_amount + &bet.potential_profit
        };

        bet.status = BetStatus::Claimed;
        self.bet_by_id(bet_id).set(&bet);

        self.send().direct(
            &caller,
            &bet.payment_token,
            0,
            &payout
        );

        self.send().direct_esdt(
            &caller,
            &token_identifier_wrap,
            bet_id,
            &BigUint::from(1u64)
        );

        self.claim_win_event(
            &caller,
            bet_id,
            BetStatus::Claimed as u8,
            &payout,
        );
    }

    //--------------------------------------------------------------------------------------------//
    //-------------------------------- Helper Functions ------------------------------------------//
    //--------------------------------------------------------------------------------------------//

    /// Determines the winning selection based on the event result.
    /// Parameters:
    /// - sport: The type of sport.
    /// - market_type: The type of market (e.g., FullTimeResult, TotalGoals).
    /// - score_home: The score of the home team.
    /// - score_away: The score of the away team.
    /// - event_id: The unique ID of the event.
    /// Returns: The ID of the winning selection.
    fn determine_winner(
        &self,
        sport: Sport,
        market_type: MarketType,
        score_home: u32,
        score_away: u32,
        event_id: u64,
    ) -> u64 {
        let market_ids = self.markets_by_event_and_sport(sport, event_id).get();
        let market_id = market_ids.iter()
            .find(|&id| {
                let market = self.markets(id).get();
                market.market_type == market_type
            })
            .unwrap_or_else(|| sc_panic!(ERR_INVALID_MARKET));
        
        let market = self.markets(market_id).get();
        
        let winning_index = match market_type {
            MarketType::FullTimeResult => {
                if score_home > score_away { 0 }
                else if score_home < score_away { 2 }
                else { 1 }
            },
            MarketType::TotalGoals => {
                if score_home + score_away > 2 { 0 }
                else { 1 }
            },
            MarketType::BothTeamsToScore => {
                if score_home > 0 && score_away > 0 { 0 }
                else { 1 }
            }
            MarketType::Winner => {
                if score_home > score_away { 0 }
                else { 2 }
            },
        };
        
        market.selections.get(winning_index).id
    }
}