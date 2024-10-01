use multiversx_sc::types::BigUint;
multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const IPFS_GATEWAY: &[u8] = "https://ipfs.io/ipfs/".as_bytes();
pub const NFT_ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.05 EGLD
pub const ROYALTIES_MAX: u32 = 10;
pub const NFT_AMOUNT: u32 = 1;
pub const TOKEN_NAME: &[u8] = b"BetCube";
pub const TOKEN_TICKER: &[u8] = b"BET";
pub const PRECISION: u32 = 18;

pub fn precision_factor<M: multiversx_sc::api::ManagedTypeApi>() -> BigUint<M> {
    BigUint::from(10u64).pow(PRECISION)
}