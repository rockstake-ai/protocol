#![no_std]

pub mod storage;
pub mod constants;
pub mod p2p;
pub mod events;
pub mod p2e;
pub mod errors;
pub mod betslip_nft;
pub mod payout;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();
#[multiversx_sc::contract]
pub trait BetCube:
storage::StorageModule
+ events::EventsModule
+ betslip_nft::BetslipNftModule
+ p2e::P2EModule
+ p2p::P2PModule{
    #[upgrade]
    fn upgrade(&self) {}

    #[init]
    fn init(&self) {
       
    }
}