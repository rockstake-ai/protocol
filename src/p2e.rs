
use crate::{constants::NFT_AMOUNT, errors::ERR_ZERO_DEPOSIT, storage::{self, Bet, Betslip, BetslipAttributes}};
multiversx_sc::imports!();
multiversx_sc::derive_imports!();


#[multiversx_sc::module]
pub trait P2EModule: storage::StorageModule 
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

    #[payable("*")]
    #[endpoint(payout)]
    fn payout(&self, token_identifier: TokenIdentifier, nft_nonce: ManagedBuffer){
        let num = BigUint::from_bytes_be_buffer(&nft_nonce).to_u64().unwrap();
        let betslip_attributes = self
        .token_manager()
        .get_token_attributes::<BetslipAttributes<Self::Api>>(num);
        require!(betslip_attributes.is_paid != true, "Ticket was already paid!");
        let new_attributes = BetslipAttributes {
            is_paid: true,
            ..betslip_attributes
        };
        self.send()
            .nft_update_attributes(&token_identifier, num, &new_attributes);
        let caller = self.blockchain().get_caller();
        self.send()
            .direct_egld(&caller, &new_attributes.payout);
    }


    #[payable("*")]
    #[endpoint(payoutMeta)]
    fn payout_meta(&self, nft: TokenIdentifier, nft_nonce: ManagedBuffer, token: TokenIdentifier){
        let num = BigUint::from_bytes_be_buffer(&nft_nonce).to_u64().unwrap();
        let nft_attributes = self
        .token_manager()
        .get_token_attributes::<BetslipAttributes<Self::Api>>(num);
        require!(nft_attributes.is_paid != true, "Ticket was already paid!");
        let new_attributes = BetslipAttributes {
            is_paid: true,
            ..nft_attributes
        };
        self.send().nft_update_attributes(&nft, num, &new_attributes);
        let caller = self.blockchain().get_caller();
        let original_payout_in_atoms = &new_attributes.payout;
        let atoms_in_micro_usdc = BigUint::from(10u64.pow(12));
        let payout_in_micro_usdc = original_payout_in_atoms / &atoms_in_micro_usdc;
    
        self.send().direct(&caller, &EgldOrEsdtTokenIdentifier::esdt(token), 0, &payout_in_micro_usdc);
        self.send().direct_esdt(&caller, &nft, num, &BigUint::from(NFT_AMOUNT));
    }
    
}