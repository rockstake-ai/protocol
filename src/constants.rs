pub mod constants {
    pub const IPFS_GATEWAY: &[u8] = "https://ipfs.io/ipfs/".as_bytes();
    pub const NFT_ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.05 EGLD
    pub const ROYALTIES_MAX: u32 = 10;
    pub const NFT_AMOUNT: u32 = 1;
    pub const TOKEN_NAME: &[u8] = b"BetCube";
    pub const TOKEN_TICKER: &[u8] = b"BET";
    
    //Bet
    pub const MIN_ODDS: u64 = 101; // 1.01
    pub const MAX_ODDS: u64 = 1000; // 10.00
    pub const MIN_STAKE: u64 = 1_000_000_000_000_000; // 0.001 EGLD în atomic units
    pub const MAX_STAKE: u64 = 1_000_000_000_000_000_000; // 1 EGLD în atomic units
}


