
use crate::{constants::NFT_AMOUNT, errors::ERR_ZERO_DEPOSIT, storage::{self, Bet, BetType, Betslip, BetslipAttributes, Status}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();


#[multiversx_sc::module]
pub trait BettingManagerModule: storage::StorageModule 
    + crate::events::EventsModule 
    + crate::betslip_nft::BetslipNftModule{

    #[payable("*")]
    #[endpoint(placeBet)]
    fn place_bet(&self, bets: ManagedVec<Bet<Self::Api>>) -> u64 {
        let caller = self.blockchain().get_caller();
       
        let (token_identifier, token_nonce, token_amount) =
            self.call_value().egld_or_single_esdt().into_tuple();

        require!(token_amount > 0, ERR_ZERO_DEPOSIT);
    
        let betslip_id = self.get_last_betslip_id() + 1;

        let stake = token_amount.clone();
        let total_odd = self.calculate_total_odd(&bets);
        let payout = self.calculate_payout(&total_odd, &stake);
    
        let betslip = Betslip {
            creator: caller.clone(),
            bets: bets.clone(),
            total_odd: total_odd.clone(),
            stake: stake.clone(),
            payout: payout.clone(),
            payment_token: token_identifier.clone(),
            payment_nonce: token_nonce,
            // status: storage::Status::InProgress,
            nft_nonce: betslip_id,
        };
        let betslip_nft_nonce = self.mint_betslip_nft(&betslip);
    
        self.betslip_by_id(betslip_id).set(&betslip);
    
        self.send().direct_esdt(
            &caller,
            self.betslip_nft_token().get_token_id_ref(),
            betslip_nft_nonce,
            &BigUint::from(1u64),
        ); 
    
        self.place_bet_event(
            &caller,
            self.betslip_nft_token().get_token_id_ref(),
            &bets,
            &total_odd,
            &stake,
            &payout,
            &token_identifier,
            token_nonce,
            // &betslip.status,
        );
    
        betslip_id
    }
    

    fn calculate_total_odd(self,bets: &ManagedVec<Bet<Self::Api>>) -> BigUint {
       let mut total_odd = BigUint::from(1u64);
       for bet in bets.iter(){
        total_odd *= bet.odd;

    }
       total_odd
    }

    fn calculate_payout(self, total_odd: &BigUint, stake: &BigUint) -> BigUint {
        let mut payout = BigUint::from(1u64);
        payout = total_odd * stake;
        payout
    }

    #[payable("EGLD")]
#[endpoint(placeBackBet)]
fn place_back_bet(
    &self,
    market_id: BigUint,
    selection_id: BigUint,
    odds: BigUint
) {
    let caller = self.blockchain().get_caller();  // Adresa utilizatorului care plasează pariul
    let amount = self.call_value().egld_value();  // Suma trimisă împreună cu tranzacția
    
    // Verificăm dacă piața există
    let mut market = self.markets(&market_id).get();
    require!(market.is_some(), "Piața nu există");
    let mut market = market.unwrap();

    // Găsim selecția validă
    let mut selection = market.selections.iter_mut().find(|s| s.selection_id == selection_id)
        .expect("Selecția nu este validă pentru această piață");

    // Adăugăm lichiditate pentru BACK și actualizăm cotele
    selection.back_liquidity += amount;
    if odds > selection.best_back_odds {
        selection.best_back_odds = odds;
    }

    // Creăm și stocăm pariul
    let new_bet = Bet {
        event: market_id.clone(),
        option: selection_id.clone(),
        value: amount.clone(),
        odd: odds.clone(),
        bet_type: BetType::Back,
        status: Status::InProgress,
    };
    
    market.bets.push(new_bet);
    
    // Actualizăm piața cu noile date
    self.markets(&market_id).set(&market);

    // Blocăm fondurile utilizatorului
    self.locked_funds(&caller).update(|current_locked| *current_locked += amount);
}


    // Funcție pentru a plasa pariuri LAY
    #[payable("EGLD")]
    #[endpoint(placeLayBet)]
    fn place_lay_bet(
        &self,
        market_id: BigUint,
        selection_id: BigUint,
        odds: BigUint
    ) {
        let caller = self.blockchain().get_caller();  // Adresa utilizatorului care plasează pariul
        let amount = self.call_value().egld_value();  // Suma trimisă împreună cu tranzacția
        
        // Verificăm dacă piața există
        let mut market = self.markets(&market_id).get();
        // require!(market.is_some(), "Piața nu există");
        let mut market = market.unwrap();
    
        // Găsim selecția validă
        let mut selection = market.selections.iter_mut().find(|s| s.selection_id == selection_id)
            .expect("Selecția nu este validă pentru această piață");
    
        // Adăugăm lichiditate pentru LAY și actualizăm cotele
        selection.lay_liquidity += amount;
        if odds > selection.best_lay_odds {
            selection.best_lay_odds = odds;
        }
    
        // Creăm și stocăm pariul
        let new_bet = Bet {
            event: market_id.clone(),
            option: selection_id.clone(),
            value: amount.clone(),
            odd: odds.clone(),
            bet_type: BetType::Lay,
            status: Status::InProgress,
        };
        
        market.bets.push(new_bet);
    
        // Actualizăm piața cu noile date
        self.markets(&market_id).set(&market);
    
        // Blocăm fondurile utilizatorului
        self.locked_funds(&caller).update(|current_locked| *current_locked += amount);
    }
    

    fn match_bets(&self, market_id: BigUint) {
        // Obținem piața specificată de market_id
        let mut market = self.markets(&market_id).get();
    
        // Parcurgem toate selecțiile pieței
        for selection in market.selections.iter() {
            // Obținem pariurile BACK și LAY
            let mut unmatched_back_bets: Vec<&mut Bet<Self::Api>> = Vec::new();
            let mut unmatched_lay_bets: Vec<&mut Bet<Self::Api>> = Vec::new();
    
            // Separăm pariurile în BACK și LAY
            for bet in market.bets.iter() {
                if bet.option == selection.selection_id && bet.status == Status::InProgress {
                    match bet.bet_type {
                        BetType::Back => unmatched_back_bets.push(bet),
                        BetType::Lay => unmatched_lay_bets.push(bet),
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