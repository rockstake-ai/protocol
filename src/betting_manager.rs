
use crate::{constants::NFT_AMOUNT, errors::ERR_ZERO_DEPOSIT, storage::{self, Bet, Betslip, BetslipAttributes}};
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

    #[endpoint]
    fn place_back_bet(
        &self,
        market_id: BigUint,
        selection_id: BigUint,
        amount: BigUint,
        odds: BigUint
    ) {
        // Logica pentru plasarea unui pariu BACK
    }

    // Funcție pentru a plasa pariuri LAY
    #[endpoint]
    fn place_lay_bet(
        &self,
        market_id: BigUint,
        selection_id: BigUint,
        amount: BigUint,
        odds: BigUint
    ) {
        // Logica pentru plasarea unui pariu LAY
    }

    // Funcție pentru potrivirea pariurilor BACK și LAY
    fn match_bets(&self, market_id: BigUint) {
        
    }

    
}