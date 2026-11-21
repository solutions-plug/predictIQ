import { env } from "../lib/env";

export interface Asset {
  code: string;
  issuer: string;
  contractId: string;
  name: string;
  decimals: number;
}

type NetworkKey = "TESTNET" | "MAINNET" | "FUTURENET";

const ASSETS_BY_NETWORK: Record<NetworkKey, Asset[]> = {
  TESTNET: [
    {
      code: "XLM",
      issuer: "native",
      contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
      name: "Stellar Lumens",
      decimals: 7,
    },
    {
      code: "USDC",
      issuer: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
      contractId: "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
      name: "USD Coin",
      decimals: 7,
    },
  ],
  MAINNET: [
    {
      code: "XLM",
      issuer: "native",
      contractId: "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
      name: "Stellar Lumens",
      decimals: 7,
    },
    {
      code: "USDC",
      issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      contractId: "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
      name: "USD Coin",
      decimals: 7,
    },
  ],
  FUTURENET: [
    {
      code: "XLM",
      issuer: "native",
      contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
      name: "Stellar Lumens",
      decimals: 7,
    },
  ],
};

function resolveNetwork(): NetworkKey {
  const network = (env.network ?? "").toUpperCase();
  if (network in ASSETS_BY_NETWORK) {
    return network as NetworkKey;
  }
  throw new Error(
    `No asset configuration for network "${env.network}". ` +
      `Expected one of: ${Object.keys(ASSETS_BY_NETWORK).join(", ")}.`,
  );
}

export const SUPPORTED_ASSETS: Asset[] = ASSETS_BY_NETWORK[resolveNetwork()];
