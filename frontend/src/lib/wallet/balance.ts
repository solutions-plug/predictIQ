import {
  Horizon,
  Networks,
  Asset,
  Keypair,
  TransactionBuilder,
  Operation,
  BASE_FEE,
} from '@stellar/stellar-sdk';

const HORIZON_URLS: Record<string, string> = {
  PUBLIC: 'https://horizon.stellar.org',
  TESTNET: 'https://horizon-testnet.stellar.org',
  FUTURENET: 'https://horizon-futurenet.stellar.org',
};

/**
 * Resolve the Horizon base URL for the given network.
 *
 * An optional `NEXT_PUBLIC_HORIZON_URL_OVERRIDE` environment variable can be
 * used to point this module at a custom/self-hosted Horizon instance. When it
 * is unset or empty, the hardcoded map above is used.
 */
export function getHorizonUrl(network: string): string {
  const override = process.env.NEXT_PUBLIC_HORIZON_URL_OVERRIDE;
  if (override && override.trim().length > 0) {
    return override.trim();
  }
  return HORIZON_URLS[network] ?? HORIZON_URLS.PUBLIC;
}

export function getHorizonServer(network: string): Horizon.Server {
  return new Horizon.Server(getHorizonUrl(network));
}

export function getNetworkPassphrase(network: string): string {
  switch (network) {
    case 'PUBLIC':
      return Networks.PUBLIC;
    case 'TESTNET':
      return Networks.TESTNET;
    case 'FUTURENET':
      return Networks.FUTURENET;
    default:
      return Networks.TESTNET;
  }
}

export async function getBalance(
  publicKey: string,
  network: string = 'TESTNET'
): Promise<string> {
  const server = getHorizonServer(network);
  const account = await server.loadAccount(publicKey);
  const nativeBalance = account.balances.find(
    (balance) => balance.asset_type === 'native'
  );
  return nativeBalance ? nativeBalance.balance : '0';
}

export async function sendPayment(
  sourceSecret: string,
  destination: string,
  amount: string,
  network: string = 'TESTNET'
): Promise<string> {
  const server = getHorizonServer(network);
  const sourceKeypair = Keypair.fromSecret(sourceSecret);
  const sourceAccount = await server.loadAccount(sourceKeypair.publicKey());

  const transaction = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase: getNetworkPassphrase(network),
  })
    .addOperation(
      Operation.payment({
        destination,
        asset: Asset.native(),
        amount,
      })
    )
    .setTimeout(30)
    .build();

  transaction.sign(sourceKeypair);
  const result = await server.submitTransaction(transaction);
  return result.hash;
}
