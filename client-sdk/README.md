# PredictIQ TypeScript SDK

Official TypeScript/JavaScript SDK for the PredictIQ prediction market platform on Stellar.

## Installation

```bash
npm install @predictiq/sdk
```

Or with yarn:

```bash
yarn add @predictiq/sdk
```

## Quick Start

```typescript
import { PredictIQClient } from '@predictiq/sdk';

// Initialize client
const client = new PredictIQClient({
  network: 'testnet',
  contractId: 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
  secretKey: 'SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX'
});

// Create a market
const marketId = await client.createMarket({
  description: 'Will BTC reach $100k by end of 2024?',
  options: ['Yes', 'No'],
  deadline: Date.now() + 86400000, // 24 hours
  resolutionDeadline: Date.now() + 172800000, // 48 hours
  oracleAddress: 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
  feedId: 'BTC/USD',
  tokenAddress: 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX'
});

// Place a bet
await client.placeBet({
  marketId,
  outcome: 0, // Betting on "Yes"
  amount: BigInt(10000000) // 1 XLM
});

// Get market details
const market = await client.getMarket(marketId);
console.log(market);
```

## Features

- ✅ Full TypeScript support with type definitions
- ✅ Promise-based async/await API
- ✅ Automatic transaction building and signing
- ✅ Event streaming support
- ✅ Comprehensive error handling
- ✅ Network abstraction (testnet/mainnet)
- ✅ Built on official Stellar SDK

## API Reference

### Client Initialization

```typescript
const client = new PredictIQClient({
  network: 'testnet' | 'mainnet',
  contractId: string,
  secretKey: string,
  rpcUrl?: string // Optional custom RPC URL
});
```

### Market Operations

#### Create Market

```typescript
const marketId = await client.createMarket({
  description: string,
  options: string[],
  deadline: number, // Unix timestamp
  resolutionDeadline: number, // Unix timestamp
  oracleAddress: string,
  feedId: string,
  tier?: 'Basic' | 'Pro' | 'Institutional',
  tokenAddress: string,
  parentId?: bigint,
  parentOutcomeIdx?: number
});
```

#### Get Market

```typescript
const market = await client.getMarket(marketId: bigint);
```

### Betting Operations

#### Place Bet

```typescript
await client.placeBet({
  marketId: bigint,
  outcome: number,
  amount: bigint,
  tokenAddress: string,
  referrer?: string
});
```

#### Claim Winnings

```typescript
const { hash, amount } = await client.claimWinnings(
  marketId: bigint,
  tokenAddress: string
);
```

### Voting & Disputes

#### Cast Vote

```typescript
await client.castVote({
  marketId: bigint,
  outcome: number,
  weight: bigint
});
```

#### File Dispute

```typescript
await client.fileDispute(marketId: bigint);
```

### Query Operations

#### Get Admin

```typescript
const admin = await client.getAdmin();
```

#### Get Revenue

```typescript
const revenue = await client.getRevenue(tokenAddress: string);
```

### Event Streaming

```typescript
for await (const event of client.watchEvents()) {
  console.log('Event:', event);
  
  // Handle different event types
  if (event.topic[0] === 'market_created') {
    console.log('New market created!');
  }
}
```

## Error Handling

The SDK throws typed errors that you can catch and handle:

```typescript
import { PredictIQError } from '@predictiq/sdk';

try {
  await client.placeBet({ ... });
} catch (error) {
  if (error instanceof PredictIQError) {
    console.error(`Error ${error.code}: ${error.message}`);
    
    switch (error.code) {
      case 102:
        console.error('Market not found');
        break;
      case 116:
        console.error('Deadline has passed');
        break;
      default:
        console.error('Unknown error');
    }
  }
}
```

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 100 | AlreadyInitialized | Contract already initialized |
| 101 | NotAuthorized | Insufficient permissions |
| 102 | MarketNotFound | Market doesn't exist |
| 103 | MarketClosed | Market closed for betting |
| 116 | DeadlinePassed | Betting deadline passed |

See [full error code reference](../docs/api/API_DOCUMENTATION.md#error-handling) for complete list.

## Examples

### Create and Bet on Market

```typescript
import { PredictIQClient } from '@predictiq/sdk';

async function example() {
  const client = new PredictIQClient({
    network: 'testnet',
    contractId: process.env.CONTRACT_ID!,
    secretKey: process.env.SECRET_KEY!
  });
  
  // Create market
  const marketId = await client.createMarket({
    description: 'Will it rain tomorrow?',
    options: ['Yes', 'No'],
    deadline: Date.now() + 86400000,
    resolutionDeadline: Date.now() + 172800000,
    oracleAddress: process.env.ORACLE_ADDRESS!,
    feedId: 'WEATHER/RAIN',
    tokenAddress: process.env.TOKEN_ADDRESS!
  });
  
  console.log('Market created:', marketId);
  
  // Place bet
  await client.placeBet({
    marketId,
    outcome: 0,
    amount: BigInt(10000000),
    tokenAddress: process.env.TOKEN_ADDRESS!
  });
  
  console.log('Bet placed!');
  
  // Get market details
  const market = await client.getMarket(marketId);
  console.log('Market:', market);
}

example().catch(console.error);
```

### Monitor Market Events

```typescript
async function monitorMarket(marketId: bigint) {
  const client = new PredictIQClient({ ... });
  
  for await (const event of client.watchEvents()) {
    const topic = event.topic[0];
    const eventMarketId = event.topic[1];
    
    if (eventMarketId === marketId) {
      switch (topic) {
        case 'bet_placed':
          console.log('New bet placed!');
          break;
        case 'market_resolved':
          console.log('Market resolved!');
          break;
      }
    }
  }
}
```

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Run tests
npm test

# Lint
npm run lint

# Format
npm run format
```

## Documentation

- [API Documentation](../docs/api/API_DOCUMENTATION.md)
- [Integration Guide](../docs/api/INTEGRATION_GUIDE.md)
- [Getting Started](../docs/api/GETTING_STARTED.md)
- [OpenAPI Specification](../openapi.yaml)

## Support

- **GitHub Issues**: [Report bugs](https://github.com/predictiq/contracts/issues)
- **Discord**: [Join community](https://discord.gg/predictiq)
- **Documentation**: [docs.predictiq.io](https://docs.predictiq.io)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please read our [Contributing Guide](../CONTRIBUTING.md) for details.
