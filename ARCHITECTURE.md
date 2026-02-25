# PredictIQ Architecture

Comprehensive architecture documentation for the PredictIQ prediction market platform.

## Table of Contents

- [System Overview](#system-overview)
- [Architecture Principles](#architecture-principles)
- [Smart Contract Architecture](#smart-contract-architecture)
- [Module Design](#module-design)
- [Data Flow](#data-flow)
- [Oracle Integration](#oracle-integration)
- [Security Architecture](#security-architecture)
- [Scalability Considerations](#scalability-considerations)

## System Overview

PredictIQ is a decentralized prediction market platform built on Stellar's Soroban smart contract platform. The system enables users to create and participate in prediction markets with hybrid resolution mechanisms combining oracle data and community voting.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend Layer                        │
│  (Web App, Mobile App, Third-party Integrations)            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ JSON-RPC / REST API
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    Stellar Blockchain                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           PredictIQ Smart Contract                   │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐       │  │
│  │  │Markets │ │ Bets   │ │Oracles │ │ Voting │       │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘       │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐       │  │
│  │  │Disputes│ │  Fees  │ │ Admin  │ │Circuit │       │  │
│  │  └────────┘ └────────┘ └────────┘ │Breaker │       │  │
│  │                                    └────────┘       │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ Oracle Feeds
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    Oracle Providers                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │Pyth Network  │  │  Reflector   │  │Custom Oracles│     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

1. **Smart Contract Layer**: Soroban smart contracts handling all business logic
2. **Oracle Layer**: External data providers (Pyth, Reflector)
3. **Storage Layer**: On-chain persistent storage
4. **Event Layer**: Contract events for off-chain indexing
5. **Frontend Layer**: User interfaces and integrations

## Architecture Principles

### 1. Modularity

The contract is organized into independent modules, each responsible for a specific domain:

- **Separation of Concerns**: Each module handles one aspect of functionality
- **Loose Coupling**: Modules interact through well-defined interfaces
- **High Cohesion**: Related functionality is grouped together

### 2. Security First

- **Access Control**: Role-based permissions for sensitive operations
- **Input Validation**: All inputs validated before processing
- **Circuit Breaker**: Emergency pause mechanism for critical issues
- **Fail-Safe Defaults**: Secure defaults for all configurations

### 3. Gas Optimization

- **Efficient Storage**: Minimized storage operations
- **Batch Operations**: Support for batching where possible
- **Lazy Loading**: Data loaded only when needed
- **Event-Driven**: Use events instead of storage for historical data

### 4. Extensibility

- **Plugin Architecture**: Easy to add new oracle providers
- **Upgradeable Design**: Support for future enhancements
- **Configurable Parameters**: Admin-adjustable settings

## Smart Contract Architecture

### Contract Structure

```rust
PredictIQContract
├── Storage
│   ├── Markets (Map<u64, Market>)
│   ├── Bets (Map<(u64, Address), Bet>)
│   ├── Votes (Map<(u64, Address), Vote>)
│   ├── Config (Admin, Fees, etc.)
│   └── Monitoring (Error counts, metrics)
├── Modules
│   ├── admin.rs        # Admin functions
│   ├── markets.rs      # Market management
│   ├── bets.rs         # Betting logic
│   ├── oracles.rs      # Oracle integration
│   ├── voting.rs       # Voting system
│   ├── disputes.rs     # Dispute resolution
│   ├── fees.rs         # Fee management
│   ├── governance.rs   # Governance functions
│   ├── circuit_breaker.rs  # Emergency controls
│   ├── monitoring.rs   # System monitoring
│   └── events.rs       # Event definitions
└── Types
    ├── Market          # Market data structure
    ├── Bet             # Bet data structure
    ├── Vote            # Vote data structure
    └── ErrorCode       # Error definitions
```

### Data Structures

#### Market

```rust
pub struct Market {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    pub outcomes: Vec<String>,
    pub deadline: u64,
    pub status: MarketStatus,
    pub resolution_source: ResolutionSource,
    pub winning_outcome: Option<u32>,
    pub total_volume: i128,
    pub outcome_volumes: Vec<i128>,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
}

pub enum MarketStatus {
    Active,
    Closed,
    PendingResolution,
    Disputed,
    Resolved,
    Cancelled,
}

pub enum ResolutionSource {
    Oracle,
    Voting,
    Hybrid,
}
```

#### Bet

```rust
pub struct Bet {
    pub bettor: Address,
    pub market_id: u64,
    pub outcome: u32,
    pub amount: i128,
    pub odds: u32,
    pub placed_at: u64,
    pub claimed: bool,
}
```

#### Vote

```rust
pub struct Vote {
    pub voter: Address,
    pub market_id: u64,
    pub outcome: u32,
    pub weight: i128,
    pub voted_at: u64,
}
```

## Module Design

### Markets Module

**Responsibility**: Market creation, lifecycle management, and queries

**Key Functions**:
- `create_market()`: Create new prediction market
- `get_market()`: Retrieve market data
- `close_market()`: Close market for betting
- `cancel_market()`: Cancel market (admin only)

**State Transitions**:
```
Active → Closed → PendingResolution → Resolved
                                    ↓
                                 Disputed → Resolved
```

### Bets Module

**Responsibility**: Bet placement, tracking, and payout calculation

**Key Functions**:
- `place_bet()`: Place bet on market outcome
- `get_bet()`: Retrieve bet information
- `calculate_payout()`: Calculate potential winnings
- `claim_winnings()`: Claim winnings after resolution

**Bet Lifecycle**:
```
Placed → Active → (Market Resolved) → Claimable → Claimed
```

### Oracles Module

**Responsibility**: Oracle integration and data fetching

**Supported Oracles**:
1. **Pyth Network**: High-frequency price feeds
2. **Reflector**: Stellar-native oracle
3. **Custom**: Extensible for additional providers

**Key Functions**:
- `fetch_oracle_result()`: Get result from oracle
- `register_oracle()`: Add new oracle provider
- `set_oracle_priority()`: Configure fallback order

**Oracle Resolution Flow**:
```
1. Query Primary Oracle (Pyth)
   ↓ (if fails)
2. Query Secondary Oracle (Reflector)
   ↓ (if fails)
3. Fallback to Community Voting
```

### Voting Module

**Responsibility**: Community voting for disputed markets

**Key Functions**:
- `cast_vote()`: Cast vote on disputed market
- `get_vote_results()`: Get current vote tally
- `finalize_vote()`: Finalize voting and resolve market

**Voting Mechanism**:
- **Weight**: Based on reputation or stake
- **Duration**: Configurable voting period
- **Threshold**: Majority required for resolution
- **Incentives**: Rewards for correct votes

### Disputes Module

**Responsibility**: Dispute filing and resolution

**Key Functions**:
- `file_dispute()`: Challenge market resolution
- `get_dispute_status()`: Check dispute state
- `resolve_dispute()`: Finalize dispute resolution

**Dispute Process**:
```
1. Market Resolved (Oracle)
   ↓
2. User Files Dispute (within window)
   ↓
3. Voting Period Opens
   ↓
4. Community Votes
   ↓
5. Dispute Resolved (majority wins)
```

### Circuit Breaker Module

**Responsibility**: Emergency pause mechanism

**States**:
- `Open`: Normal operation
- `PartialFreeze`: Limited operations (withdrawals only)
- `FullFreeze`: All operations paused

**Triggers**:
- Manual (admin)
- Automatic (error threshold exceeded)
- Governance vote

## Data Flow

### Market Creation Flow

```
User → create_market()
  ↓
Validate inputs
  ↓
Check authorization
  ↓
Generate market ID
  ↓
Store market data
  ↓
Emit MarketCreated event
  ↓
Return market ID
```

### Bet Placement Flow

```
User → place_bet()
  ↓
Validate market exists & active
  ↓
Check deadline not passed
  ↓
Validate outcome index
  ↓
Transfer tokens from user
  ↓
Calculate odds
  ↓
Store bet data
  ↓
Update market volumes
  ↓
Emit BetPlaced event
  ↓
Return success
```

### Market Resolution Flow

```
Deadline Passed
  ↓
close_market() → Status: Closed
  ↓
resolve_market()
  ↓
Query Oracle
  ↓
┌─────────────┬─────────────┐
│ Success     │ Failure     │
↓             ↓             
Set outcome   Open voting   
  ↓             ↓
Status:       Status:
Resolved      PendingResolution
  ↓             ↓
Emit event    Wait for votes
              ↓
              Finalize vote
              ↓
              Status: Resolved
```

### Dispute Resolution Flow

```
Market Resolved
  ↓
User files dispute (within window)
  ↓
Status: Disputed
  ↓
Voting period opens
  ↓
Users cast votes
  ↓
Voting period ends
  ↓
Tally votes
  ↓
Majority outcome wins
  ↓
Update market outcome
  ↓
Status: Resolved
  ↓
Emit DisputeResolved event
```

## Oracle Integration

### Oracle Architecture

```
┌─────────────────────────────────────────┐
│        PredictIQ Contract               │
│  ┌───────────────────────────────────┐  │
│  │      Oracle Manager               │  │
│  │  ┌─────────┐  ┌─────────┐        │  │
│  │  │Priority │  │Fallback │        │  │
│  │  │ Queue   │  │ Logic   │        │  │
│  │  └─────────┘  └─────────┘        │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
         │         │         │
         ▼         ▼         ▼
    ┌────────┐ ┌────────┐ ┌────────┐
    │ Pyth   │ │Reflect.│ │Custom  │
    │Network │ │ Oracle │ │Oracles │
    └────────┘ └────────┘ └────────┘
```

### Oracle Priority System

1. **Primary**: Pyth Network (high-frequency, institutional-grade)
2. **Secondary**: Reflector (Stellar-native, reliable)
3. **Tertiary**: Custom oracles (market-specific)
4. **Fallback**: Community voting

### Oracle Data Format

```rust
pub struct OracleResult {
    pub source: String,        // Oracle identifier
    pub outcome: u32,          // Winning outcome index
    pub confidence: u32,       // Confidence score (0-100)
    pub timestamp: u64,        // Result timestamp
    pub signature: BytesN<64>, // Cryptographic signature
}
```

## Security Architecture

### Access Control

```
┌─────────────────────────────────────┐
│         Access Control              │
├─────────────────────────────────────┤
│ Admin                               │
│  - Set fees                         │
│  - Pause contract                   │
│  - Update config                    │
│  - Emergency actions                │
├─────────────────────────────────────┤
│ Market Creator                      │
│  - Create markets                   │
│  - Cancel own markets (conditions)  │
├─────────────────────────────────────┤
│ Users                               │
│  - Place bets                       │
│  - Cast votes                       │
│  - File disputes                    │
│  - Claim winnings                   │
└─────────────────────────────────────┘
```

### Security Layers

1. **Input Validation**: All inputs sanitized and validated
2. **Authorization Checks**: Role-based access control
3. **Reentrancy Protection**: State updates before external calls
4. **Integer Overflow Protection**: Safe math operations
5. **Circuit Breaker**: Emergency pause mechanism
6. **Rate Limiting**: Prevent spam and abuse
7. **Monitoring**: Automatic error detection

### Threat Mitigation

| Threat | Mitigation |
|--------|------------|
| Oracle Manipulation | Multiple oracle sources, voting fallback |
| Front-running | Commit-reveal scheme (future) |
| Spam Markets | Market creation fee, reputation system |
| Vote Manipulation | Weighted voting, stake requirements |
| Reentrancy | Checks-effects-interactions pattern |
| Integer Overflow | Checked arithmetic operations |
| Unauthorized Access | Role-based access control |
| DoS Attacks | Rate limiting, gas limits |

## Scalability Considerations

### Storage Optimization

- **Minimal Storage**: Only essential data on-chain
- **Event-Driven**: Historical data via events
- **Pagination**: Large datasets paginated
- **Archival**: Old markets archived off-chain

### Gas Optimization

- **Batch Operations**: Multiple operations in one transaction
- **Efficient Data Structures**: Optimized storage layout
- **Lazy Evaluation**: Compute only when needed
- **Caching**: Frequently accessed data cached

### Performance Targets

| Metric | Target |
|--------|--------|
| Market Creation | < 0.5s |
| Bet Placement | < 0.3s |
| Market Query | < 0.1s |
| Vote Casting | < 0.3s |
| Resolution | < 1.0s |

### Horizontal Scaling

- **Multiple Markets**: Unlimited concurrent markets
- **Sharding**: Future consideration for extreme scale
- **Off-chain Indexing**: Event indexing for queries
- **CDN**: Static content delivery

## Future Enhancements

### Planned Features

1. **Advanced Market Types**
   - Scalar markets (price ranges)
   - Combinatorial markets
   - Conditional markets

2. **Enhanced Oracle Integration**
   - More oracle providers
   - Oracle reputation system
   - Decentralized oracle network

3. **Governance**
   - DAO structure
   - Token-based voting
   - Protocol upgrades

4. **Layer 2 Integration**
   - State channels for high-frequency trading
   - Rollups for scalability

5. **Cross-chain Support**
   - Bridge to other blockchains
   - Multi-chain markets

### Upgrade Path

```
Current: v1.0 (Monolithic Contract)
  ↓
v2.0: Modular Contracts
  ↓
v3.0: DAO Governance
  ↓
v4.0: Cross-chain Support
```

## Diagrams

### System Context Diagram

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Users   │────▶│PredictIQ │────▶│ Stellar  │
└──────────┘     │ Frontend │     │Blockchain│
                 └──────────┘     └──────────┘
                      │                 │
                      ▼                 ▼
                 ┌──────────┐     ┌──────────┐
                 │Analytics │     │ Oracles  │
                 └──────────┘     └──────────┘
```

### Component Diagram

```
┌─────────────────────────────────────────┐
│         PredictIQ Contract              │
│                                         │
│  ┌────────┐  ┌────────┐  ┌────────┐   │
│  │Markets │  │  Bets  │  │Oracles │   │
│  └───┬────┘  └───┬────┘  └───┬────┘   │
│      │           │            │        │
│      └───────────┼────────────┘        │
│                  │                     │
│            ┌─────▼─────┐               │
│            │  Storage  │               │
│            └───────────┘               │
└─────────────────────────────────────────┘
```

## References

- [Stellar Documentation](https://developers.stellar.org/)
- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Pyth Network](https://pyth.network/)
- [Reflector Oracle](https://reflector.network/)

---

For implementation details, see [DEVELOPMENT.md](./DEVELOPMENT.md).  
For contribution guidelines, see [CONTRIBUTING.md](./CONTRIBUTING.md).
