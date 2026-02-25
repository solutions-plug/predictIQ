use soroban_sdk::{contracttype, Address, String, Vec, Map, Bytes};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketStatus {
    Active,
    PendingResolution,
    Disputed,
    Resolved,
    Cancelled,
}

// Optimized Market with bit-packing
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    pub id: u64,
    pub creator: Address,
    pub metadata: Bytes, // Compressed description + options
    pub header: u32, // Bit-packed: status(8) | winning_outcome(8) | flags(16)
    pub deadline: u64,
    pub resolution_deadline: u64,
    pub oracle_config: OracleConfig,
    pub total_staked: i128,
    pub outcome_stakes: Map<u32, i128>,
    pub dispute_snapshot_ledger: Option<u32>,
    pub pending_resolution_timestamp: Option<u64>,
    pub dispute_timestamp: Option<u64>,
    pub token_address: Address,
    pub resolved_at: Option<u64>, // For garbage collection
}

impl Market {
    pub fn status(&self) -> MarketStatus {
        bitpack::unpack_status(self.header)
    }
    
    pub fn winning_outcome(&self) -> Option<u32> {
        bitpack::unpack_winning_outcome(self.header)
    }
    
    pub fn set_status(&mut self, status: MarketStatus) {
        let winning_outcome = bitpack::unpack_winning_outcome(self.header);
        let is_disputed = bitpack::is_disputed(self.header);
        let is_cancelled = bitpack::is_cancelled(self.header);
        let has_oracle = bitpack::has_oracle(self.header);
        self.header = bitpack::pack_header(status, winning_outcome, is_disputed, is_cancelled, has_oracle);
    }
    
    pub fn set_winning_outcome(&mut self, outcome: Option<u32>) {
        let status = bitpack::unpack_status(self.header);
        let is_disputed = bitpack::is_disputed(self.header);
        let is_cancelled = bitpack::is_cancelled(self.header);
        let has_oracle = bitpack::has_oracle(self.header);
        self.header = bitpack::pack_header(status, outcome, is_disputed, is_cancelled, has_oracle);
    }
}

// Optimized Bet with minimal fields
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bet {
    pub market_id: u64,
    pub bettor: Address,
    pub outcome: u32,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vote {
    pub market_id: u64,
    pub voter: Address,
    pub outcome: u32,
    pub weight: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedTokens {
    pub voter: Address,
    pub market_id: u64,
    pub amount: i128,
    pub unlock_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub oracle_address: Address,
    pub feed_id: String,
    pub min_responses: Option<u32>, // Optimized: None defaults to 1
}

// Gas optimization constants
pub const MAX_PUSH_PAYOUT_WINNERS: u32 = 50; // Threshold for switching to pull mode
pub const MAX_OUTCOMES_PER_MARKET: u32 = 100; // Limit to prevent excessive iteration

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigKey {
    Admin,
    MarketAdmin,
    FeeAdmin,
    GuardianAccount,
    BaseFee,
    CircuitBreakerState,
    CreationDeposit,
    GuardianSet,
    PendingUpgrade,
    UpgradeVotes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
    Paused, // Emergency pause state - blocks high-risk operations
}

// Bit-packing helpers
pub mod bitpack {
    use super::MarketStatus;
    
    // Header layout: [status: 8 bits][winning_outcome: 8 bits][flags: 16 bits]
    // Flags: bit 0 = is_disputed, bit 1 = is_cancelled, bit 2 = has_oracle
    
    pub fn pack_header(status: MarketStatus, winning_outcome: Option<u32>, is_disputed: bool, is_cancelled: bool, has_oracle: bool) -> u32 {
        let status_bits = match status {
            MarketStatus::Active => 0u32,
            MarketStatus::PendingResolution => 1u32,
            MarketStatus::Disputed => 2u32,
            MarketStatus::Resolved => 3u32,
            MarketStatus::Cancelled => 4u32,
        };
        
        let outcome_bits = winning_outcome.unwrap_or(255) as u32;
        
        let mut flags = 0u32;
        if is_disputed { flags |= 0x01; }
        if is_cancelled { flags |= 0x02; }
        if has_oracle { flags |= 0x04; }
        
        (status_bits << 24) | (outcome_bits << 16) | flags
    }
    
    pub fn unpack_status(header: u32) -> MarketStatus {
        match (header >> 24) & 0xFF {
            0 => MarketStatus::Active,
            1 => MarketStatus::PendingResolution,
            2 => MarketStatus::Disputed,
            3 => MarketStatus::Resolved,
            4 => MarketStatus::Cancelled,
            _ => MarketStatus::Active,
        }
    }
    
    pub fn unpack_winning_outcome(header: u32) -> Option<u32> {
        let outcome = ((header >> 16) & 0xFF) as u32;
        if outcome == 255 { None } else { Some(outcome) }
    }
    
    pub fn is_disputed(header: u32) -> bool {
        (header & 0x01) != 0
    }
    
    pub fn is_cancelled(header: u32) -> bool {
        (header & 0x02) != 0
    }
    
    pub fn has_oracle(header: u32) -> bool {
        (header & 0x04) != 0
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: String,
    pub initiated_at: u64,
    pub votes_for: Vec<Address>,
    pub votes_against: Vec<Address>,
}

// Constants for upgrade governance
pub const TIMELOCK_DURATION: u64 = 48 * 60 * 60; // 48 hours in seconds
pub const MAJORITY_THRESHOLD_PERCENT: u32 = 51; // 51% for majority

// TTL Management Constants (in ledgers, ~5 seconds per ledger)
pub const TTL_LOW_THRESHOLD: u32 = 17_280; // ~1 day (86400 seconds / 5)
pub const TTL_HIGH_THRESHOLD: u32 = 518_400; // ~30 days (2592000 seconds / 5)
pub const PRUNE_GRACE_PERIOD: u64 = 2_592_000; // 30 days in seconds
