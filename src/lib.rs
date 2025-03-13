#![no_std]

pub mod storage;
pub mod constants;
pub mod events;
pub mod bet;
pub mod errors;
pub mod nft;
pub mod fund;
pub mod market;
pub mod orderbook;
pub mod validation;
pub mod types;
pub mod utils;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();
#[multiversx_sc::contract]
pub trait Rockstake:
storage::StorageModule
+ events::EventsModule
+ nft::NftModule
+ fund::FundModule
+ bet::BetModule
+ market::MarketModule
+ orderbook::OrderbookModule
+ validation::ValidationModule
+ utils::UtilsModule{
    #[upgrade]
    fn upgrade(&self) {}

    #[init]
    fn init(&self) {
    }
}