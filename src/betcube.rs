#![no_std]

pub mod storage;
pub mod constants;
pub mod events;
pub mod bet_manager;
pub mod errors;
pub mod nft_manager;
pub mod payout;
pub mod market_manager;
pub mod fund_manager;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();
#[multiversx_sc::contract]
pub trait BetCube:
storage::StorageModule
+ events::EventsModule
+ nft_manager::NftManagerModule
+ payout::PayoutModule
+ bet_manager::BetManagerModule{
    #[upgrade]
    fn upgrade(&self) {}

    #[init]
    fn init(&self) {
        self.market_counter().set(&BigUint::from(1u32));
    }
}