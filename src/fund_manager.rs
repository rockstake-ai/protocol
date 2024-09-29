use crate::storage::{self, Status};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FundManagerModule: storage::StorageModule{
    #[payable("*")]
    #[endpoint(lockFunds)]
    fn lock_funds(&self, user: ManagedAddress, amount: BigUint) {
        let caller = self.blockchain().get_caller();
        let payment_amount = self.call_value().egld_or_single_fungible_esdt();  // Obținem suma plătită în EGLD
        
        // require!(payment_amount == amount, "Suma trimisă nu corespunde cu cea specificată");
    
        self.locked_funds(&user).update(|current_locked| *current_locked += payment_amount);
    }
    
    #[only_owner]
    #[endpoint(distributeWinnings)]
    fn distribute_winnings(&self, market_id: BigUint, winner_address: ManagedAddress) {
        let market = self.markets(&market_id).get();  // Obținem piața respectivă
        let mut total_payout = BigUint::zero();

        for bet in market.bets.iter() {
            if bet.status == Status::Win && bet.user == winner_address {
            let payout = bet.value.clone() * bet.odd.clone();  // câștig = suma pariată * cota
            total_payout += payout;

            // Actualizăm fondurile blocate ale câștigătorului
            self.locked_funds(&winner_address).update(|current_locked| *current_locked -= bet.value.clone());
            }
        };
        self.send().direct_egld(&winner_address, &total_payout);
}

}