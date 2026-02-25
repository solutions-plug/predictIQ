# Getting Started with PredictIQ API

## Quick Start Guide

This guide will help you get started with the PredictIQ prediction market platform in under 10 minutes.

## Prerequisites

- Node.js 16+ or Python 3.8+
- Basic knowledge of JavaScript/TypeScript or Python
- Stellar testnet account (we'll create one)

## Step 1: Install Dependencies

### TypeScript/JavaScript

```bash
npm install @stellar/stellar-sdk dotenv
```

### Python

```bash
pip install stellar-sdk python-dotenv
```

## Step 2: Create Testnet Account

Visit the [Stellar Laboratory](https://laboratory.stellar.org/#account-creator?network=test) to create and fund a testnet account.

Or use the Stellar SDK:

```typescript
import * as StellarSdk from '@stellar/stellar-sdk';

// Generate keypair
const keypair = StellarSdk.Keypair.random();
console.log('Public Key:', keypair.publicKey());
console.log('Secret Key:', keypair.secret());

// Fund account at: https://laboratory.stellar.org/#account-creator?network=test
```

## Step 3: Configure Environment

Create a `.env` file:

```bash
# Network
NETWORK=testnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org:443

# Your account
SECRET_KEY=SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
PUBLIC_KEY=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Contract (use testnet deployment)
CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
TOKEN_ADDRESS=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

## Step 4: Initialize Client

```typescript
import * as StellarSdk from '@stellar/stellar-sdk';
import dotenv from 'dotenv';

dotenv.config();

// Setup
const server = new StellarSdk.SorobanRpc.Server(
  process.env.SOROBAN_RPC_URL!
);
const keypair = StellarSdk.Keypair.fromSecret(process.env.SECRET_KEY!);
const contract = new StellarSdk.Contract(process.env.CONTRACT_ID!);

console.log('Connected to PredictIQ!');
```

## Step 5: Create Your First Market

```typescript
async function createMarket() {
  const account = await server.getAccount(keypair.publicKey());
  
  // Build oracle config
  const oracleConfig = StellarSdk.xdr.ScVal.scvMap([
    new StellarSdk.xdr.ScMapEntry({
      key: StellarSdk.nativeToScVal('oracle_address', { type: 'symbol' }),
      val: StellarSdk.nativeToScVal(
        'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        { type: 'address' }
      )
    }),
    new StellarSdk.xdr.ScMapEntry({
      key: StellarSdk.nativeToScVal('feed_id', { type: 'symbol' }),
      val: StellarSdk.nativeToScVal('BTC/USD', { type: 'string' })
    })
  ]);
  
  const transaction = new StellarSdk.TransactionBuilder(account, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: StellarSdk.Networks.TESTNET,
  })
    .addOperation(
      contract.call(
        'create_market',
        StellarSdk.nativeToScVal(keypair.publicKey(), { type: 'address' }),
        StellarSdk.nativeToScVal('Will BTC reach $100k?', { type: 'string' }),
        StellarSdk.nativeToScVal(['Yes', 'No'], { type: 'vec' }),
        StellarSdk.nativeToScVal(
          Math.floor(Date.now() / 1000) + 86400,
          { type: 'u64' }
        ), // 24 hours
        StellarSdk.nativeToScVal(
          Math.floor(Date.now() / 1000) + 172800,
          { type: 'u64' }
        ), // 48 hours
        oracleConfig,
        StellarSdk.nativeToScVal('Basic', { type: 'symbol' }),
        StellarSdk.nativeToScVal(process.env.TOKEN_ADDRESS!, { type: 'address' }),
        StellarSdk.nativeToScVal(0, { type: 'u64' }),
        StellarSdk.nativeToScVal(0, { type: 'u32' })
      )
    )
    .setTimeout(30)
    .build();
  
  transaction.sign(keypair);
  const result = await server.sendTransaction(transaction);
  
  console.log('Market created! Transaction:', result.hash);
}

createMarket().catch(console.error);
```

## Step 6: Place a Bet

```typescript
async function placeBet(marketId: bigint, outcome: number, amount: bigint) {
  const account = await server.getAccount(keypair.publicKey());
  
  const transaction = new StellarSdk.TransactionBuilder(account, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: StellarSdk.Networks.TESTNET,
  })
    .addOperation(
      contract.call(
        'place_bet',
        StellarSdk.nativeToScVal(keypair.publicKey(), { type: 'address' }),
        StellarSdk.nativeToScVal(marketId, { type: 'u64' }),
        StellarSdk.nativeToScVal(outcome, { type: 'u32' }),
        StellarSdk.nativeToScVal(amount, { type: 'i128' }),
        StellarSdk.nativeToScVal(process.env.TOKEN_ADDRESS!, { type: 'address' }),
        StellarSdk.xdr.ScVal.scvOption(undefined) // no referrer
      )
    )
    .setTimeout(30)
    .build();
  
  transaction.sign(keypair);
  const result = await server.sendTransaction(transaction);
  
  console.log('Bet placed! Transaction:', result.hash);
}

// Bet 1 XLM on outcome 0 (Yes)
placeBet(BigInt(1), 0, BigInt(10000000)).catch(console.error);
```

## Step 7: Query Market Data

```typescript
async function getMarket(marketId: bigint) {
  const account = await server.getAccount(keypair.publicKey());
  
  const transaction = new StellarSdk.TransactionBuilder(account, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: StellarSdk.Networks.TESTNET,
  })
    .addOperation(
      contract.call(
        'get_market',
        StellarSdk.nativeToScVal(marketId, { type: 'u64' })
      )
    )
    .setTimeout(30)
    .build();
  
  const result = await server.simulateTransaction(transaction);
  
  if (result.results && result.results.length > 0) {
    const market = StellarSdk.scValToNative(result.results[0].retval);
    console.log('Market:', JSON.stringify(market, null, 2));
    return market;
  }
  
  return null;
}

getMarket(BigInt(1)).catch(console.error);
```

## Common Use Cases

### 1. Create a Binary Market

```typescript
// Simple Yes/No market
const marketId = await createMarket({
  description: 'Will it rain tomorrow?',
  options: ['Yes', 'No'],
  deadline: Date.now() + 86400000, // 24 hours
  // ...
});
```

### 2. Create a Multi-Outcome Market

```typescript
// Multiple choice market
const marketId = await createMarket({
  description: 'Who will win the election?',
  options: ['Candidate A', 'Candidate B', 'Candidate C', 'Other'],
  deadline: Date.now() + 2592000000, // 30 days
  // ...
});
```

### 3. Place Multiple Bets

```typescript
// Bet on multiple outcomes
await placeBet(marketId, 0, BigInt(5000000)); // 0.5 XLM on outcome 0
await placeBet(marketId, 1, BigInt(3000000)); // 0.3 XLM on outcome 1
```

### 4. Monitor Market Events

```typescript
async function watchMarket(marketId: bigint) {
  const latestLedger = await server.getLatestLedger();
  let cursor = latestLedger.sequence.toString();
  
  setInterval(async () => {
    const events = await server.getEvents({
      startLedger: cursor,
      filters: [
        {
          type: 'contract',
          contractIds: [process.env.CONTRACT_ID!]
        }
      ]
    });
    
    for (const event of events.events) {
      console.log('Event:', event);
    }
    
    if (events.latestLedger) {
      cursor = events.latestLedger.toString();
    }
  }, 5000);
}

watchMarket(BigInt(1));
```

## Error Handling

Always wrap contract calls in try-catch blocks:

```typescript
try {
  await placeBet(marketId, outcome, amount);
  console.log('Success!');
} catch (error: any) {
  // Parse error code
  const errorCode = parseErrorCode(error);
  
  switch (errorCode) {
    case 102:
      console.error('Market not found');
      break;
    case 103:
      console.error('Market is closed');
      break;
    case 116:
      console.error('Deadline has passed');
      break;
    default:
      console.error('Error:', error.message);
  }
}
```

## Helper Functions

### Parse Error Code

```typescript
function parseErrorCode(error: any): number {
  // Extract error code from Stellar error
  // Implementation depends on error structure
  return 0;
}
```

### Format Amount

```typescript
function toStroops(xlm: number): bigint {
  return BigInt(Math.floor(xlm * 10000000));
}

function fromStroops(stroops: bigint): number {
  return Number(stroops) / 10000000;
}

// Usage
const amount = toStroops(1.5); // 1.5 XLM = 15000000 stroops
console.log(fromStroops(amount)); // 1.5
```

### Format Timestamp

```typescript
function toUnixTimestamp(date: Date): number {
  return Math.floor(date.getTime() / 1000);
}

function fromUnixTimestamp(timestamp: number): Date {
  return new Date(timestamp * 1000);
}

// Usage
const deadline = toUnixTimestamp(new Date('2024-12-31'));
```

## Next Steps

1. **Read the full [API Documentation](./API_DOCUMENTATION.md)**
2. **Explore [Integration Guide](./INTEGRATION_GUIDE.md)** for advanced features
3. **Check out [Code Examples](./examples/)** for more use cases
4. **Review [OpenAPI Specification](../../openapi.yaml)** for complete API reference
5. **Join our [Discord](https://discord.gg/predictiq)** for support

## Troubleshooting

### Common Issues

**Issue**: Transaction fails with "insufficient balance"
- **Solution**: Ensure your account has enough XLM for fees and operations

**Issue**: "Contract not found" error
- **Solution**: Verify CONTRACT_ID in .env file is correct

**Issue**: "Market not found" error
- **Solution**: Check that the market ID exists using `get_market`

**Issue**: "Deadline passed" error
- **Solution**: Market betting period has ended, cannot place new bets

### Getting Help

- **Documentation**: [docs.predictiq.io](https://docs.predictiq.io)
- **GitHub Issues**: [github.com/predictiq/contracts/issues](https://github.com/predictiq/contracts/issues)
- **Discord**: [discord.gg/predictiq](https://discord.gg/predictiq)

## Resources

- [Stellar Documentation](https://developers.stellar.org/)
- [Soroban Documentation](https://soroban.stellar.org/)
- [Stellar SDK Reference](https://stellar.github.io/js-stellar-sdk/)
- [PredictIQ GitHub](https://github.com/predictiq/contracts)

---

**Ready to build?** Start with the [Integration Guide](./INTEGRATION_GUIDE.md) for more advanced features!
