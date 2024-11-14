pub mod constants {

    pub const IPFS_GATEWAY: &[u8] = "https://ipfs.io/ipfs/".as_bytes();
    pub const NFT_ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.05 EGLD
    pub const NFT_ROYALTIES: u64 = 0_00;
    pub const NFT_AMOUNT: u32 = 1;
    pub const TOKEN_NAME: &[u8] = b"BetcubeTickets";
    pub const TOKEN_TICKER: &[u8] = b"BET";
    
    //Bet
    pub const MIN_ODDS: u32 = 101;      // 1.01
    pub const MAX_ODDS: u32 = 100000;   // 1000.00

    // Market constants
    pub const MAX_MARKETS: u64 = 1_000_000;
    pub const MAX_SELECTIONS: usize = 100;
    // pub const MAX_DESCRIPTION_LENGTH: usize = 100;
    // pub const MIN_DESCRIPTION_LENGTH: usize = 3;
    
    // User constants
    pub const MAX_USER_EXPOSURE: u64 = 10_000_000_000_000_000_000; // 10 EGLD
}

