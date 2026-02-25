# PredictIQ API Documentation

## Overview

The PredictIQ API provides a comprehensive interface for interacting with prediction markets on the Stellar blockchain. This documentation covers all available endpoints, authentication methods, error handling, and integration examples.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Authentication](#authentication)
3. [API Endpoints](#api-endpoints)
4. [Error Handling](#error-handling)
5. [Rate Limits](#rate-limits)
6. [Code Examples](#code-examples)
7. [Client Libraries](#client-libraries)
8. [Versioning](#versioning)

## Getting Started

### Base URLs

- **Testnet**: `https://soroban-testnet.stellar.org:443`
- **Futurenet**: `https://rpc-futurenet.stellar.org:443`
- **Mainnet**: `https://rpc.mainnet.stellar.org:443`

### Quick Start

```bash
# Install Stellar SDK
npm install @stellar/stellar-sdk

# Or using yarn
yarn add @stellar/stellar-sdk
```

### Basic Example

```typescript
import * as StellarSdk from '@stellar/stellar-sdk';

// Connect to testnet
const server = new StellarSdk.SorobanRpc.Server(
  'https://soroban-testnet.stellar.org:443'
);

// Load your account
const sourceKeypair = StellarSdk.Keypair.fromSecret('YOUR_SECRET_KEY');
const sourceAccount = await server.getAccount(sourceKeypair.publicKey());

// Contract address
const contractAddress = 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX';
```

## Authentication

All contract interactions require Stellar account signatures. The authentication flow:

1. **Account Setup**: Create or load a Stellar account
2. **Transaction Building**: Build a transaction with contract invocation
3. **Signing**: Sign the transaction with your secret key
4. **Submission**: Submit the signed transaction to the network

### Authentication Example

```typescript
import * as StellarSdk from '@stellar/stellar-sdk';

const keypair = StellarSdk.Keypair.fromSecret('YOUR_SECRET_KEY');
const contract = new StellarSdk.Contract(contractAddress);

// Build transaction
const transaction = new StellarSdk.TransactionBuilder(sourceAccount, {
  fee: StellarSdk.BASE_FEE,
  networkPassphrase: StellarSdk.Networks.TESTNET,
})
  .addOperation(
    contract.call('place_bet', 
      StellarSdk.nativeToScVal(keypair.publicKey(), { type: 'address' }),
      StellarSdk.nativeToScVal(1, { type: 'u64' }),
      StellarSdk.nativeToScVal(0, { type: 'u32' }),
      StellarSdk.nativeToScVal(10000000, { type: 'i128' }),
      StellarSdk.nativeToScVal(tokenAddress, { type: 'address' })
    )
  )
  .setTimeout(30)
  .build();

// Sign and submit
transaction.sign(keypair);
const result = await server.sendTransaction(transaction);
```

## API Endpoints

### Initialization

#### Initialize Contract

Initialize the PredictIQ contract with admin and base fee configuration.

**Endpoint**: `initialize`

**Parameters**:
- `admin` (Address): Admin account address
- `base_fee` (i128): Base fee amount in stroops

**Returns**: `Result<(), ErrorCode>`

**Example**:
```typescript
const result = await contract.call('initialize',
  StellarSdk.nativeToScVal(adminAddress, { type: 'address' }),
  StellarSdk.nativeToScVal(1000000, { type: 'i128' })
);
```

**Errors**:
- `100 AlreadyInitialized`: Contract already initialized

---

### Market Management

#### Create Market

Create a new prediction market.

**Endpoint**: `create_market`

**Parameters**:
- `creator` (Address): Market creator address
- `description` (String): Market description (max 256 chars)
- `options` (Vec<String>): Array of outcome options (2-100)
- `deadline` (u64): Unix timestamp when betting closes
- `resolution_deadline` (u64): Unix timestamp for resolution
- `oracle_config` (OracleConfig): Oracle configuration
- `tier` (MarketTier): Market tier (Basic/Pro/Institutional)
- `native_token` (Address): Token contract address
- `parent_id` (u64): Parent market ID (0 for independent)
- `parent_outcome_idx` (u32): Required parent outcome

**Returns**: `Result<u64, ErrorCode>` - Market ID

**Example**:
```typescript
const marketId = await contract.call('create_market',
  StellarSdk.nativeToScVal(creatorAddress, { type: 'address' }),
  StellarSdk.nativeToScVal('Will BTC reach $100k?', { type: 'string' }),
  StellarSdk.nativeToScVal(['Yes', 'No'], { type: 'vec' }),
  StellarSdk.nativeToScVal(1735689600, { type: 'u64' }),
  StellarSdk.nativeToScVal(1735776000, { type: 'u64' }),
  // ... oracle config, tier, token, parent info
);
```

**Errors**:
- `101 NotAuthorized`: Caller not authorized
- `106 InvalidBetAmount`: Invalid deposit amount

#### Get Market

Retrieve market details by ID.

**Endpoint**: `get_market`

**Parameters**:
- `id` (u64): Market ID

**Returns**: `Option<Market>`

**Example**:
```typescript
const market = await contract.call('get_market',
  StellarSdk.nativeToScVal(1, { type: 'u64' })
);
```

---

### Betting

#### Place Bet

Place a bet on a market outcome.

**Endpoint**: `place_bet`

**Parameters**:
- `bettor` (Address): Bettor's address
- `market_id` (u64): Market ID
- `outcome` (u32): Outcome index
- `amount` (i128): Bet amount in stroops
- `token_address` (Address): Token contract address
- `referrer` (Option<Address>): Optional referrer address

**Returns**: `Result<(), ErrorCode>`

**Example**:
```typescript
await contract.call('place_bet',
  StellarSdk.nativeToScVal(bettorAddress, { type: 'address' }),
  StellarSdk.nativeToScVal(1, { type: 'u64' }),
  StellarSdk.nativeToScVal(0, { type: 'u32' }),
  StellarSdk.nativeToScVal(10000000, { type: 'i128' }),
  StellarSdk.nativeToScVal(tokenAddress, { type: 'address' }),
  StellarSdk.nativeToScVal(null, { type: 'option' })
);
```

**Errors**:
- `102 MarketNotFound`: Market doesn't exist
- `103 MarketClosed`: Market closed for betting
- `105 InvalidOutcome`: Invalid outcome index
- `106 InvalidBetAmount`: Invalid bet amount
- `115 MarketNotActive`: Market not active
- `116 DeadlinePassed`: Betting deadline passed
- `117 CannotChangeOutcome`: Cannot change existing bet

#### Claim Winnings

Claim winnings from a resolved market.

**Endpoint**: `claim_winnings`

**Parameters**:
- `bettor` (Address): Bettor's address
- `market_id` (u64): Market ID
- `token_address` (Address): Token contract address

**Returns**: `Result<i128, ErrorCode>` - Amount claimed

**Example**:
```typescript
const winnings = await contract.call('claim_winnings',
  StellarSdk.nativeToScVal(bettorAddress, { type: 'address' }),
  StellarSdk.nativeToScVal(1, { type: 'u64' }),
  StellarSdk.nativeToScVal(tokenAddress, { type: 'address' })
);
```

---

### Voting & Disputes

#### Cast Vote

Cast a vote in a disputed market.

**Endpoint**: `cast_vote`

**Parameters**:
- `voter` (Address): Voter's address
- `market_id` (u64): Market ID
- `outcome` (u32): Outcome to vote for
- `weight` (i128): Voting weight

**Returns**: `Result<(), ErrorCode>`

**Example**:
```typescript
await contract.call('cast_vote',
  StellarSdk.nativeToScVal(voterAddress, { type: 'address' }),
  StellarSdk.nativeToScVal(1, { type: 'u64' }),
  StellarSdk.nativeToScVal(0, { type: 'u32' }),
  StellarSdk.nativeToScVal(1000000, { type: 'i128' })
);
```

**Errors**:
- `109 CircuitBreakerOpen`: System paused
- `102 MarketNotFound`: Market doesn't exist
- `118 MarketNotDisputed`: Market not disputed
- `113 AlreadyVoted`: Already voted

#### File Dispute

File a dispute for a market resolution.

**Endpoint**: `file_dispute`

**Parameters**:
- `disciplinarian` (Address): Disputer's address
- `market_id` (u64): Market ID

**Returns**: `Result<(), ErrorCode>`

**Example**:
```typescript
await contract.call('file_dispute',
  StellarSdk.nativeToScVal(disputerAddress, { type: 'address' }),
  StellarSdk.nativeToScVal(1, { type: 'u64' })
);
```

**Errors**:
- `109 CircuitBreakerOpen`: System paused
- `102 MarketNotFound`: Market doesn't exist
- `119 MarketNotPendingResolution`: Invalid state

---

### Administration

#### Set Circuit Breaker

Update circuit breaker state (admin only).

**Endpoint**: `set_circuit_breaker`

**Parameters**:
- `state` (CircuitBreakerState): New state (Closed/Open/HalfOpen/Paused)

**Returns**: `Result<(), ErrorCode>`

**Errors**:
- `101 NotAuthorized`: Not admin
- `120 AdminNotSet`: Admin not configured

#### Set Base Fee

Update base fee (admin only).

**Endpoint**: `set_base_fee`

**Parameters**:
- `amount` (i128): New base fee amount

**Returns**: `Result<(), ErrorCode>`

---

### Governance

#### Initialize Guardians

Initialize the guardian set (admin only, one-time).

**Endpoint**: `initialize_guardians`

**Parameters**:
- `guardians` (Vec<Guardian>): Array of guardian configurations

**Returns**: `Result<(), ErrorCode>`

#### Initiate Upgrade

Initiate a contract upgrade proposal.

**Endpoint**: `initiate_upgrade`

**Parameters**:
- `wasm_hash` (String): Hash of new WASM code

**Returns**: `Result<(), ErrorCode>`

#### Vote for Upgrade

Vote on a pending upgrade.

**Endpoint**: `vote_for_upgrade`

**Parameters**:
- `voter` (Address): Guardian address
- `vote_for` (bool): True to approve, false to reject

**Returns**: `Result<bool, ErrorCode>` - Whether upgrade executed

---

## Error Handling

All endpoints return `Result<T, ErrorCode>` where errors are standardized numeric codes.

### Error Code Reference

| Code | Name | Description |
|------|------|-------------|
| 100 | AlreadyInitialized | Contract already initialized |
| 101 | NotAuthorized | Insufficient permissions |
| 102 | MarketNotFound | Market doesn't exist |
| 103 | MarketClosed | Market closed for betting |
| 104 | MarketStillActive | Cannot resolve active market |
| 105 | InvalidOutcome | Invalid outcome index |
| 106 | InvalidBetAmount | Invalid bet amount |
| 107 | InsufficientBalance | Insufficient balance |
| 108 | OracleFailure | Oracle error |
| 109 | CircuitBreakerOpen | System paused |
| 110 | DisputeWindowClosed | Dispute period ended |
| 111 | VotingNotStarted | Voting not started |
| 112 | VotingEnded | Voting ended |
| 113 | AlreadyVoted | Already voted |
| 114 | FeeTooHigh | Fee too high |
| 115 | MarketNotActive | Market not active |
| 116 | DeadlinePassed | Deadline passed |
| 117 | CannotChangeOutcome | Cannot change bet |
| 118 | MarketNotDisputed | Not disputed |
| 119 | MarketNotPendingResolution | Invalid state |
| 120 | AdminNotSet | Admin not set |

### Error Handling Example

```typescript
try {
  const result = await contract.call('place_bet', ...params);
  console.log('Bet placed successfully');
} catch (error) {
  const errorCode = parseErrorCode(error);
  
  switch (errorCode) {
    case 102:
      console.error('Market not found');
      break;
    case 103:
      console.error('Market is closed');
      break;
    case 116:
      console.error('Betting deadline has passed');
      break;
    default:
      console.error('Unknown error:', errorCode);
  }
}
```

## Rate Limits

Rate limits are enforced at the Stellar network level:

- **Testnet**: ~1000 operations per ledger (5 seconds)
- **Mainnet**: ~1000 operations per ledger (5 seconds)

Best practices:
- Batch operations when possible
- Implement exponential backoff for retries
- Monitor transaction status before submitting new ones

## Code Examples

See the [examples directory](./examples/) for complete integration examples:

- [TypeScript Client](./examples/typescript-client.ts)
- [Python Client](./examples/python-client.py)
- [Rust Client](./examples/rust-client.rs)

## Client Libraries

### TypeScript/JavaScript

```bash
npm install @predictiq/sdk
```

```typescript
import { PredictIQClient } from '@predictiq/sdk';

const client = new PredictIQClient({
  network: 'testnet',
  contractId: 'CXXXXXXX...',
  secretKey: 'SXXXXXXX...'
});

// Create market
const marketId = await client.createMarket({
  description: 'Will BTC reach $100k?',
  options: ['Yes', 'No'],
  deadline: Date.now() + 86400000,
  // ...
});

// Place bet
await client.placeBet({
  marketId,
  outcome: 0,
  amount: 10000000
});
```

### Python

```bash
pip install predictiq-sdk
```

```python
from predictiq import PredictIQClient

client = PredictIQClient(
    network='testnet',
    contract_id='CXXXXXXX...',
    secret_key='SXXXXXXX...'
)

# Create market
market_id = client.create_market(
    description='Will BTC reach $100k?',
    options=['Yes', 'No'],
    deadline=int(time.time()) + 86400
)

# Place bet
client.place_bet(
    market_id=market_id,
    outcome=0,
    amount=10000000
)
```

## Versioning

The API follows semantic versioning (SemVer):

- **Major version**: Breaking changes
- **Minor version**: New features (backward compatible)
- **Patch version**: Bug fixes

Current version: **1.0.0**

### Version History

- **1.0.0** (2024-02): Initial release
  - Core market functionality
  - Oracle integration
  - Dispute resolution
  - Governance system

## Support

- **Documentation**: [https://docs.predictiq.io](https://docs.predictiq.io)
- **GitHub**: [https://github.com/predictiq/contracts](https://github.com/predictiq/contracts)
- **Discord**: [https://discord.gg/predictiq](https://discord.gg/predictiq)

## Additional Resources

- [OpenAPI Specification](../../openapi.yaml)
- [Swagger UI](./swagger-ui.html)
- [Integration Guide](./INTEGRATION_GUIDE.md)
- [Best Practices](./BEST_PRACTICES.md)
