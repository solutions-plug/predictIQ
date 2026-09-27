export interface AssetConfig {
  code: string;
  issuer?: string;
  name: string;
  decimals: number;
  /** ISO 4217 currency code when this asset is a fiat-equivalent, otherwise undefined. */
  fiatCurrency?: string;
}

export const SUPPORTED_ASSETS: AssetConfig[] = [
  {
    code: 'XLM',
    issuer: undefined,
    name: 'Stellar Lumens',
    decimals: 7,
  },
  {
    code: 'USDC',
    issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    name: 'USD Coin',
    decimals: 2,
    fiatCurrency: 'USD',
  },
  {
    code: 'yXLM',
    issuer: 'GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55',
    name: 'Yield XLM',
    decimals: 7,
  },
];

/**
 * Resolve the ISO 4217 fiat currency a settlement asset is pegged to, if any.
 * Returns undefined for non-fiat-equivalent assets (e.g. XLM), so callers can
 * fall back to a neutral numeric format instead of assuming USD.
 */
export function getFiatCurrencyForAsset(assetCode?: string | null): string | undefined {
  if (!assetCode) {
    return undefined;
  }
  const normalized = assetCode.trim().toUpperCase();
  const asset = SUPPORTED_ASSETS.find((candidate) => candidate.code.toUpperCase() === normalized);
  return asset?.fiatCurrency;
}
