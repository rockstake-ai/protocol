use crate::{errors::ERR_ZERO_DEPOSIT, storage::{self, Bet, BetType, Betslip, Status}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait BetManagerModule: storage::StorageModule 
    + crate::events::EventsModule 
    + crate::nft_manager::BetslipNftModule{

    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(&self, market_id: BigUint, selection_id: BigUint, odds: BigUint, bet_type: BetType) -> u64 {
        let caller = self.blockchain().get_caller();
        let (token_identifier, token_nonce, token_amount) =
        self.call_value().egld_or_single_esdt().into_tuple();

        let bet_id = self.get_last_bet_id() + 1;

        require!(!self.markets(&market_id).is_empty(), "Market doesn't exist!");
        let market = self.markets(&market_id).get();

        let mut selection = market.selections.iter().find(|s| s.selection_id == selection_id)
            .expect("Selection is not valid for this specific market");

        let win_amount = match bet_type{
            BetType::Back => {
                selection.back_liquidity += &token_amount;
                if odds > selection.best_back_odds{
                    selection.best_back_odds = odds.clone();
                }
                (token_amount.clone() * (odds.clone() - BigUint::from(1000u32))) / BigUint::from(1000u32)
            },
            BetType::Lay => {
                selection.lay_liquidity +=token_amount;
                if odds < selection.best_lay_odds || selection.best_lay_odds == BigUint::zero(){
                    selection.best_lay_odds = odds.clone();
                }
                (token_amount.clone() * (odds.clone() - BigUint::from(1000u32))) / BigUint::from(1000u32)
            }
        };

        let bet = Bet {
            event: market_id.clone(),
            option: selection_id,
            stake_amount: token_amount.clone(),
            win_amount,
            odd: odds.clone(),
            bet_type: bet_type.clone(),
            status: Status::InProgress,
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            nft_nonce: bet_id,
        };

        let bet_nft_nonce = self.mint_bet_nft(&bet);
    
        self.bet_by_id(bet_id).set(&bet);
    
        self.send().direct_esdt(
            &caller,
            self.betslip_nft_token().get_token_id_ref(),
            bet_nft_nonce,
            &BigUint::from(1u64),
        ); 
        
        market.bets.push(bet);
        
        // Actualizăm piața cu noile date 
        self.markets(&market_id).set(&market);

        // Blocăm fondurile utilizatorului
        self.locked_funds(&caller).update(|current_locked| *current_locked += token_amount);

        self.bet_placed_event(&caller, 
            self.betslip_nft_token().get_token_id_ref(),
            &market_id, &selection_id, 
            &token_amount, 
            &odds, 
            bet_type, 
            &token_identifier,token_nonce
        );

        bet_id
    }

    fn match_bets(&self, market_id: BigUint) {
        // Obținem piața specificată de market_id
        let mut market = self.markets(&market_id).get();
    
        // Parcurgem toate selecțiile pieței
        for selection in market.selections.iter() {
            // Obținem pariurile BACK și LAY
            let mut unmatched_back_bets: ManagedVec<&mut Bet<Self::Api>> = ManagedVec::new();
            let mut unmatched_lay_bets: ManagedVec<&mut Bet<Self::Api>> = ManagedVec::new();
    
            // Separăm pariurile în BACK și LAY
            for bet in market.bets.iter() {
                if bet.option == selection.selection_id && bet.status == Status::InProgress {
                    match bet.bet_type {
                        BetType::Back => unmatched_back_bets.push(&mut bet),
                        BetType::Lay => unmatched_lay_bets.push(&mut bet),
                    }
                }
            }
    
            // Potrivim pariurile BACK și LAY
            for back_bet in unmatched_back_bets.iter_mut() {
                for lay_bet in unmatched_lay_bets.iter_mut() {
                    if back_bet.odd <= lay_bet.odd {
                        // Determinăm suma care poate fi potrivită
                        let match_amount = std::cmp::min(back_bet.value.clone(), lay_bet.value.clone());
    
                        // Actualizăm sumele rămase și statusul pariurilor
                        back_bet.value -= match_amount.clone();
                        lay_bet.value -= match_amount.clone();
    
                        if back_bet.value == BigUint::zero() {
                            back_bet.status = Status::Matched;
                        }
    
                        if lay_bet.value == BigUint::zero() {
                            lay_bet.status = Status::Matched;
                        }
    
                        // Actualizăm lichiditatea pentru selecția respectivă
                        selection.back_liquidity -= match_amount.clone();
                        selection.lay_liquidity -= match_amount.clone();
    
                        // Ieșim din bucla lay dacă pariul LAY a fost complet potrivit
                        if lay_bet.value == BigUint::zero() {
                            break;
                        }
                    }
                }
            }
        }
    
        // Salvăm piața actualizată
        self.markets(&market_id).set(&market);
    }
    

    
}