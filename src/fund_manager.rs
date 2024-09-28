use crate::storage;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait FundManagerModule: storage::StorageModule{
    #[payable("*")]
    #[endpoint]
    fn lock_funds(&self, user: ManagedAddress, amount: BigUint) {
    }

    // Funcție pentru distribuirea câștigurilor după finalizarea evenimentului
    #[only_owner]
    #[endpoint]
    fn distribute_winnings(&self, market_id: BigUint, winner_address: ManagedAddress) {
    }
}