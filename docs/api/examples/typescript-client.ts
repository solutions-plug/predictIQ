/**
 * PredictIQ TypeScript Client Example
 * 
 * This example demonstrates how to interact with the PredictIQ contract
 * using the Stellar SDK in TypeScript.
 */

import * as StellarSdk from '@stellar/stellar-sdk';
import dotenv from 'dotenv';

dotenv.config();

// Configuration
const CONFIG = {
  network: process.env.NETWORK || 'testnet',
  rpcUrl: process.env.SOROBAN_RPC_URL || 'https://soroban-testnet.stellar.org:443',
  contractId: process.env.CONTRACT_ID!,
  secretKey: process.env.SECRET_KEY!,
  tokenAddress: process.env.TOKEN_ADDRESS!,
};

// Network passphrase
const NETWORK_PASSPHRASE = CONFIG.network === 'mainnet'
  ? StellarSdk.Networks.PUBLIC
  : StellarSdk.Networks.TESTNET;

/**
 * PredictIQ Client Class
 */
export class PredictIQClient {
  private server: StellarSdk.SorobanRpc.Server;
  private keypair: StellarSdk.Keypair;
  private contract: StellarSdk.Contract;

  constructor(
    rpcUrl: string,
    contractId: string,
    secretKey: string
  ) {
    this.server = new StellarSdk.SorobanRpc.Server(rpcUrl);
    this.keypair = StellarSdk.Keypair.fromSecret(secretKey);
    this.contract = new StellarSdk.Contract(contractId);
  }

  /**
   * Get account from network
   */
  private async getAccount(): Promise<StellarSdk.Account> {
    return await this.server.getAccount(this.keypair.publicKey());
  }

  /**
   * Build and submit transaction
   */
  private async submitTransaction(
    operation: StellarSdk.xdr.Operation
  ): Promise<StellarSdk.SorobanRpc.Api.SendTransactionResponse> {
    const account = await this.getAccount();
    
    const transaction = new StellarSdk.TransactionBuilder(account, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();
    
    transaction.sign(this.keypair);
    return await this.server.sendTransaction(transaction);
  }

  /**
   * Simulate transaction (for read-only operations)
   */
  private async simulateTransaction(
    operation: StellarSdk.xdr.Operation
  ): Promise<any> {
    const account = await this.getAccount();
    
    const transaction = new StellarSdk.TransactionBuilder(account, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: NETWORK_PASSPHRASE,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();
    
    const result = await this.server.simulateTransaction(transaction);
    
    if (result.results && result.results.length > 0) {
      return StellarSdk.scValToNative(result.results[0].retval);
    }
    
    return null;
  }

  /**
   * Initialize contract
   */
  async initialize(adminAddress: string, baseFee: bigint): Promise<string> {
    const operation = this.contract.call(
      'initialize',
      StellarSdk.nativeToScVal(adminAddress, { type: 'address' }),
      StellarSdk.nativeToScVal(baseFee, { type: 'i128' })
    );
    
    const result = await this.submitTransaction(operation);
    return result.hash;
  }

  /**
   * Create a new market
   */
  async createMarket(params: {
    description: string;
    options: string[];
    deadline: number;
    resolutionDeadline: number;
    oracleAddress: string;
    feedId: string;
    tier?: 'Basic' | 'Pro' | 'Institutional';
    tokenAddress: string;
    parentId?: bigint;
    parentOutcomeIdx?: number;
  }): Promise<bigint> {
    // Build oracle config
    const oracleConfig = StellarSdk.xdr.ScVal.scvMap([
      new StellarSdk.xdr.ScMapEntry({
        key: StellarSdk.nativeToScVal('oracle_address', { type: 'symbol' }),
        val: StellarSdk.nativeToScVal(params.oracleAddress, { type: 'address' })
      }),
      new StellarSdk.xdr.ScMapEntry({
        key: StellarSdk.nativeToScVal('feed_id', { type: 'symbol' }),
        val: StellarSdk.nativeToScVal(params.feedId, { type: 'string' })
      }),
      new StellarSdk.xdr.ScMapEntry({
        key: StellarSdk.nativeToScVal('min_responses', { type: 'symbol' }),
        val: StellarSdk.xdr.ScVal.scvOption(
          StellarSdk.nativeToScVal(1, { type: 'u32' })
        )
      })
    ]);
    
    const operation = this.contract.call(
      'create_market',
      StellarSdk.nativeToScVal(this.keypair.publicKey(), { type: 'address' }),
      StellarSdk.nativeToScVal(params.description, { type: 'string' }),
      StellarSdk.nativeToScVal(params.options, { type: 'vec' }),
      StellarSdk.nativeToScVal(params.deadline, { type: 'u64' }),
      StellarSdk.nativeToScVal(params.resolutionDeadline, { type: 'u64' }),
      oracleConfig,
      StellarSdk.nativeToScVal(params.tier || 'Basic', { type: 'symbol' }),
      StellarSdk.nativeToScVal(params.tokenAddress, { type: 'address' }),
      StellarSdk.nativeToScVal(params.parentId || BigInt(0), { type: 'u64' }),
      StellarSdk.nativeToScVal(params.parentOutcomeIdx || 0, { type: 'u32' })
    );
    
    const result = await this.submitTransaction(operation);
    
    // In a real implementation, parse the market ID from the result
    // For now, return a placeholder
    return BigInt(1);
  }

  /**
   * Get market details
   */
  async getMarket(marketId: bigint): Promise<any> {
    const operation = this.contract.call(
      'get_market',
      StellarSdk.nativeToScVal(marketId, { type: 'u64' })
    );
    
    return await this.simulateTransaction(operation);
  }

  /**
   * Place a bet
   */
  async placeBet(params: {
    marketId: bigint;
    outcome: number;
    amount: bigint;
    tokenAddress: string;
    referrer?: string;
  }): Promise<string> {
    const operation = this.contract.call(
      'place_bet',
      StellarSdk.nativeToScVal(this.keypair.publicKey(), { type: 'address' }),
      StellarSdk.nativeToScVal(params.marketId, { type: 'u64' }),
      StellarSdk.nativeToScVal(params.outcome, { type: 'u32' }),
      StellarSdk.nativeToScVal(params.amount, { type: 'i128' }),
      StellarSdk.nativeToScVal(params.tokenAddress, { type: 'address' }),
      params.referrer
        ? StellarSdk.xdr.ScVal.scvOption(
            StellarSdk.nativeToScVal(params.referrer, { type: 'address' })
          )
        : StellarSdk.xdr.ScVal.scvOption(undefined)
    );
    
    const result = await this.submitTransaction(operation);
    return result.hash;
  }

  /**
   * Claim winnings
   */
  async claimWinnings(
    marketId: bigint,
    tokenAddress: string
  ): Promise<{ hash: string; amount: bigint }> {
    const operation = this.contract.call(
      'claim_winnings',
      StellarSdk.nativeToScVal(this.keypair.publicKey(), { type: 'address' }),
      StellarSdk.nativeToScVal(marketId, { type: 'u64' }),
      StellarSdk.nativeToScVal(tokenAddress, { type: 'address' })
    );
    
    const result = await this.submitTransaction(operation);
    
    // In a real implementation, parse the amount from the result
    return {
      hash: result.hash,
      amount: BigInt(0)
    };
  }

  /**
   * Cast a vote in a disputed market
   */
  async castVote(params: {
    marketId: bigint;
    outcome: number;
    weight: bigint;
  }): Promise<string> {
    const operation = this.contract.call(
      'cast_vote',
      StellarSdk.nativeToScVal(this.keypair.publicKey(), { type: 'address' }),
      StellarSdk.nativeToScVal(params.marketId, { type: 'u64' }),
      StellarSdk.nativeToScVal(params.outcome, { type: 'u32' }),
      StellarSdk.nativeToScVal(params.weight, { type: 'i128' })
    );
    
    const result = await this.submitTransaction(operation);
    return result.hash;
  }

  /**
   * File a dispute
   */
  async fileDispute(marketId: bigint): Promise<string> {
    const operation = this.contract.call(
      'file_dispute',
      StellarSdk.nativeToScVal(this.keypair.publicKey(), { type: 'address' }),
      StellarSdk.nativeToScVal(marketId, { type: 'u64' })
    );
    
    const result = await this.submitTransaction(operation);
    return result.hash;
  }

  /**
   * Get admin address
   */
  async getAdmin(): Promise<string | null> {
    const operation = this.contract.call('get_admin');
    return await this.simulateTransaction(operation);
  }

  /**
   * Get revenue for a token
   */
  async getRevenue(tokenAddress: string): Promise<bigint> {
    const operation = this.contract.call(
      'get_revenue',
      StellarSdk.nativeToScVal(tokenAddress, { type: 'address' })
    );
    
    const result = await this.simulateTransaction(operation);
    return BigInt(result || 0);
  }

  /**
   * Listen for contract events
   */
  async *watchEvents(startLedger?: string): AsyncGenerator<any> {
    let cursor = startLedger;
    
    if (!cursor) {
      const latestLedger = await this.server.getLatestLedger();
      cursor = latestLedger.sequence.toString();
    }
    
    while (true) {
      const events = await this.server.getEvents({
        startLedger: cursor,
        filters: [
          {
            type: 'contract',
            contractIds: [this.contract.contractId()]
          }
        ]
      });
      
      for (const event of events.events) {
        yield event;
      }
      
      if (events.latestLedger) {
        cursor = events.latestLedger.toString();
      }
      
      // Wait 5 seconds before next poll
      await new Promise(resolve => setTimeout(resolve, 5000));
    }
  }
}

/**
 * Example usage
 */
async function main() {
  // Initialize client
  const client = new PredictIQClient(
    CONFIG.rpcUrl,
    CONFIG.contractId,
    CONFIG.secretKey
  );

  console.log('PredictIQ Client initialized');

  // Example 1: Create a market
  console.log('\n--- Creating Market ---');
  try {
    const marketId = await client.createMarket({
      description: 'Will BTC reach $100k by end of 2024?',
      options: ['Yes', 'No'],
      deadline: Math.floor(Date.now() / 1000) + 86400, // 24 hours
      resolutionDeadline: Math.floor(Date.now() / 1000) + 172800, // 48 hours
      oracleAddress: 'CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
      feedId: 'BTC/USD',
      tier: 'Basic',
      tokenAddress: CONFIG.tokenAddress
    });
    
    console.log('Market created with ID:', marketId);
  } catch (error) {
    console.error('Failed to create market:', error);
  }

  // Example 2: Get market details
  console.log('\n--- Getting Market Details ---');
  try {
    const market = await client.getMarket(BigInt(1));
    console.log('Market details:', JSON.stringify(market, null, 2));
  } catch (error) {
    console.error('Failed to get market:', error);
  }

  // Example 3: Place a bet
  console.log('\n--- Placing Bet ---');
  try {
    const txHash = await client.placeBet({
      marketId: BigInt(1),
      outcome: 0, // Betting on "Yes"
      amount: BigInt(10000000), // 1 XLM
      tokenAddress: CONFIG.tokenAddress
    });
    
    console.log('Bet placed, transaction hash:', txHash);
  } catch (error) {
    console.error('Failed to place bet:', error);
  }

  // Example 4: Watch for events
  console.log('\n--- Watching Events ---');
  try {
    let eventCount = 0;
    for await (const event of client.watchEvents()) {
      console.log('Event received:', event);
      
      // Stop after 5 events for demo purposes
      if (++eventCount >= 5) break;
    }
  } catch (error) {
    console.error('Failed to watch events:', error);
  }

  // Example 5: Claim winnings
  console.log('\n--- Claiming Winnings ---');
  try {
    const result = await client.claimWinnings(
      BigInt(1),
      CONFIG.tokenAddress
    );
    
    console.log('Winnings claimed:', result.amount, 'stroops');
    console.log('Transaction hash:', result.hash);
  } catch (error) {
    console.error('Failed to claim winnings:', error);
  }
}

// Run examples if this file is executed directly
if (require.main === module) {
  main().catch(console.error);
}

export default PredictIQClient;
