pub const ERR_INVALID_MARKET: &str = "Market doesn't exist!";
pub const ERR_MARKET_NOT_OPEN: &str = "Market is not open for betting";
pub const ERR_MARKET_NOT_CLOSED: &str = "Market is not closed";
pub const ERR_MARKET_NOT_SETTLED: &str = "Market is not settled";
pub const ERR_MARKET_CLOSED: &str = "Cannot place bets after event start time";
pub const ERR_MARKET_ALREADY_EXISTS: &str = "Market already exists";
pub const ERR_MARKET_TIMESTAMP: &str = "Invalid closing timestamp";

pub const ERR_TOO_MANY_SELECTIONS: &str = "Too many selections";
pub const ERR_INVALID_SELECTION: &str = "Invalid selection ID";

pub const ERR_STAKE_OUT_OF_RANGE: &str = "Stake amount outside allowed range";
pub const ERR_ODDS_OUT_OF_RANGE: &str = "Odds outside allowed range";

pub const ERR_LIABILITY_BACK_BET: &str = "Liability must be zero for Back bets";
pub const ERR_LIABILITY_ZERO: &str = "Liability must be greater than zero for Lay bets";
pub const ERR_LIABILITY_TOTAL_AMOUNT: &str = "Liability parameter doesn't match the required liability for the given total amount";
pub const ERR_INVALID_STAKE_LIABILITY_LAY_BET: &str = "Invalid stake/liability ratio for Lay bet";

pub const ERR_TOKEN_ALREADY_ISSUED: &str = "Token already issued";
pub const ERR_TOKEN_NOT_ISSUED: &str = "Token not issued";
pub const ERR_INVALID_NFT_TOKEN: &str = "Invalid token";
pub const ERR_INVALID_NFT_TOKEN_NONCE: &str = "Invalid token nonce";
pub const ERR_INVALID_ROLE: &str = "Unauthorized! Invalid Role";
pub const ERR_INVALID_TIMESTAMP: &str = "Close timestamp must be in the future";

pub const ERR_MAXIMUM_STAKE: &str = "Exceeds maximum user exposure limit";

// Erori noi pentru calculate_stake_and_liability
pub const ERR_ODDS_TOO_LOW: &str = "Odds must be greater than 1.00";
pub const ERR_INVALID_STAKE: &str = "Invalid stake calculation for Lay bet";
pub const ERR_INVALID_LIABILITY: &str = "Invalid liability calculation for Lay bet";

// Erori noi pentru cancel_bet
pub const ERR_NOT_BET_OWNER: &str = "Not bet owner";
pub const ERR_BET_CANNOT_BE_CANCELLED: &str = "Bet cannot be cancelled";