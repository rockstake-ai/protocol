pub mod constants {
    use multiversx_sc::{api::ManagedTypeApi, types::BigUint};

    pub const IPFS_GATEWAY: &[u8] = "https://ipfs.io/ipfs/".as_bytes();
    pub const NFT_ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.05 EGLD
    pub const NFT_ROYALTIES: u64 = 0_00;
    pub const NFT_AMOUNT: u32 = 1;
    pub const TOKEN_NAME: &[u8] = b"BetcubeTickets";
    pub const TOKEN_TICKER: &[u8] = b"BET";
    
    //Bet
    pub const MIN_ODDS: u64 = 101; // 1.01
    pub const MAX_ODDS: u64 = 1000; // 100.00
    pub fn min_stake<M: ManagedTypeApi>() -> BigUint<M> {
        BigUint::from(1_000_000u64) // 1 USDC
    }

    pub fn max_stake<M: ManagedTypeApi>() -> BigUint<M> {
        BigUint::from(10_000_000_000u64) // 10000 USDC
    }

    // Market constants
    pub const MAX_MARKETS: u64 = 1_000_000;
    pub const MAX_SELECTIONS: usize = 100;
    pub const MAX_DESCRIPTION_LENGTH: usize = 100;
    pub const MIN_DESCRIPTION_LENGTH: usize = 3;
    
    // User constants
    pub const MAX_USER_EXPOSURE: u64 = 10_000_000_000_000_000_000; // 10 EGLD
}


