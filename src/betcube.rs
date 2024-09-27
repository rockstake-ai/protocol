#![no_std]

pub mod storage;
pub mod constants;
pub mod events;
pub mod place_bet;
pub mod errors;
pub mod betslip_nft;
pub mod payout;
pub mod market_manager;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();
#[multiversx_sc::contract]
pub trait BetCube:
storage::StorageModule
+ events::EventsModule
+ betslip_nft::BetslipNftModule
+ payout::PayoutModule
+ place_bet::PlaceBetModule{
    #[upgrade]
    fn upgrade(&self) {}

    #[init]
    fn init(&self) {
       
    }
}