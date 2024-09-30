use crate::{errors::ERR_ZERO_DEPOSIT, storage::{self, Bet, BetType, Betslip, Market, Status}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: storage::StorageModule 
    + crate::events::EventsModule 
    + crate::nft_manager::NftManagerModule{

    // #[payable("*")]
    // #[endpoint(placeBet)]
    // fn place_bet(&self, market_id: BigUint, selection_id: BigUint, odds: BigUint, bet_type: BetType) -> u64 {
    //     let caller = self.blockchain().get_caller();
    //     let (token_identifier, token_nonce, token_amount) =
    //     self.call_value().egld_or_single_esdt().into_tuple();

    //     let bet_id = self.get_last_bet_id() + 1;

    //     require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
    //     let market = self.markets(&market_id).get();

    //     let mut selection = market.selections.iter().find(|s| s.selection_id == selection_id)
    //         .expect("Selection is not valid for this specific market");

    //     let win_amount = match bet_type{
    //         BetType::Back => {
    //             selection.back_liquidity += &token_amount;
    //             if odds > selection.best_back_odds{
    //                 selection.best_back_odds = odds.clone();
    //             }
    //             (token_amount.clone() * (odds.clone() - BigUint::from(1000u32))) / BigUint::from(1000u32)
    //         },
    //         BetType::Lay => {
    //             selection.lay_liquidity +=token_amount;
    //             if odds < selection.best_lay_odds || selection.best_lay_odds == BigUint::zero(){
    //                 selection.best_lay_odds = odds.clone();
    //             }
    //             (token_amount.clone() * (odds.clone() - BigUint::from(1000u32))) / BigUint::from(1000u32)
    //         }
    //     };

    //     let bet = Bet {
    //         event: market_id.clone(),
    //         option: selection_id,
    //         stake_amount: token_amount.clone(),
    //         win_amount,
    //         odd: odds.clone(),
    //         bet_type: bet_type.clone(),
    //         status: Status::InProgress,
    //         payment_token: token_identifier.clone(),
    //         payment_nonce: token_nonce,
    //         nft_nonce: bet_id,
    //     };

    //     let bet_nft_nonce = self.mint_bet_nft(&bet);
    
    //     self.bet_by_id(bet_id).set(&bet);
    
    //     self.send().direct_esdt(
    //         &caller,
    //         self.betslip_nft_token().get_token_id_ref(),
    //         bet_nft_nonce,
    //         &BigUint::from(1u64),
    //     ); 
        
    //     market.bets.push(bet);
        
    //     // Actualizăm piața cu noile date 
    //     self.markets(&market_id).set(&market);

    //     // Blocăm fondurile utilizatorului
    //     self.locked_funds(&caller).update(|current_locked| *current_locked += token_amount);

    //     self.bet_placed_event(&caller, 
    //         self.betslip_nft_token().get_token_id_ref(),
    //         &market_id, &selection_id, 
    //         &token_amount, 
    //         &odds, 
    //         bet_type, 
    //         &token_identifier,token_nonce
    //     );

    //     bet_id
    // }

    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(&self, market_id: BigUint, selection_id: BigUint, odds: BigUint, bet_type: BetType) -> u64 {
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, token_amount) =
        self.call_value().egld_or_single_esdt().into_tuple();

        let bet_id = self.get_last_bet_id() + 1;

        require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
        let mut market = self.markets(&market_id).get();
        
        let mut bet = self.create_bet(&market_id, &selection_id, &odds, bet_type, &token_amount, &token_identifier, token_nonce, bet_id);

        let bet_nft_nonce = self.mint_bet_nft(&bet);

        let (matched_amount,remaining_amount) = self.match_single_bet(&mut market, bet);
        if matched_amount > BigUint::zero() {
            self.update_bet_after_match(&mut bet, &matched_amount, &remaining_amount);
        }
        self.bet_by_id(bet_id).set(&bet);
        market.bets.push(bet.clone());
        self.markets(&market_id).set(&market);
    
        // Blocăm doar fondurile rămase nepotrivite
        if remaining_amount > BigUint::zero() {
            self.locked_funds(&caller).update(|current_locked| *current_locked += &remaining_amount);
        }
    
        self.send().direct_esdt(&caller, self.betslip_nft_token().get_token_id_ref(), bet_nft_nonce, &BigUint::from(1u64));
    
        self.bet_placed_event(
            &caller,
            self.betslip_nft_token().get_token_id_ref(),
            &market_id,
            &selection_id,
            &token_amount,
            &odds,
            bet_type,
            &token_identifier,
            token_nonce,
            &matched_amount,
            &remaining_amount
        );
    
        bet_id
    }

    fn match_single_bet(&self, market: &mut Market<Self::Api>, new_bet: &mut Bet<Self::Api>) -> (BigUint, BigUint) {
        let mut matched_amount = BigUint::zero();
        let mut remaining_amount = new_bet.stake_amount.clone();
    
        for existing_bet in market.bets.iter_mut() {
            if existing_bet.option == new_bet.option && existing_bet.status == Status::InProgress && 
               existing_bet.bet_type != new_bet.bet_type {
                if (new_bet.bet_type == BetType::Back && new_bet.odd >= existing_bet.odd) ||
                   (new_bet.bet_type == BetType::Lay && new_bet.odd <= existing_bet.odd) {
                    let match_amount = if remaining_amount < existing_bet.stake_amount {
                        remaining_amount.clone()
                    } else {
                        existing_bet.stake_amount.clone()
                    };
                    
                    matched_amount += &match_amount;
                    remaining_amount -= &match_amount;
                    existing_bet.stake_amount -= &match_amount;
    
                    if existing_bet.stake_amount == BigUint::zero() {
                        existing_bet.status = Status::Matched;
                    }
    
                    if remaining_amount == BigUint::zero() {
                        break;
                    }
                }
            }
        }
    
        (matched_amount, remaining_amount)
    }
    
    fn update_bet_after_match(&self, bet: &mut Bet<Self::Api>, matched_amount: &BigUint, remaining_amount: &BigUint) {
        bet.stake_amount = remaining_amount.clone();
        if remaining_amount == &BigUint::zero() {
            bet.status = Status::Matched;
        } else {
            bet.status = Status::PartiallyMatched;
        }
        bet.win_amount = self.calculate_win_amount(bet.bet_type, &bet.stake_amount, &bet.odd);
    }
    
    fn calculate_win_amount(&self, bet_type: BetType, stake_amount: &BigUint, odds: &BigUint) -> BigUint {
        match bet_type {
            BetType::Back => (stake_amount * (odds - BigUint::from(1000u32))) / BigUint::from(1000u32),
            BetType::Lay => (stake_amount * (odds - BigUint::from(1000u32))) / BigUint::from(1000u32),
        }
    }    
    
}