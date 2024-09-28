#![no_std]

pub mod storage;
pub mod constants;
pub mod events;
pub mod betting_manager;
pub mod errors;
pub mod betslip_nft;
pub mod payout;
pub mod market_manager;
// pub mod fund_manager;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();
#[multiversx_sc::contract]
pub trait BetCube:
storage::StorageModule
+ events::EventsModule
+ betslip_nft::BetslipNftModule
+ payout::PayoutModule
+ betting_manager::BettingManagerModule{
    #[upgrade]
    fn upgrade(&self) {}

    #[init]
    fn init(&self) {
       
    }
}