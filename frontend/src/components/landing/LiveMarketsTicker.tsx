'use client';

import React from 'react';
import Link from 'next/link';
import { useAsync } from '../../lib/hooks/useAsync';
import { api } from '../../lib/api/public-client';
import type { components } from '../../lib/api/schema';

type FeaturedMarket = components['schemas']['FeaturedMarketView'];

const MAX_ROWS = 4;

type RowStatus = 'active' | 'closing-soon' | 'resolved';
const CLOSING_SOON_MS = 24 * 60 * 60 * 1000;

function deriveRowStatus(market: FeaturedMarket): RowStatus {
  if (market.resolved_outcome !== null && market.resolved_outcome !== undefined) {
    return 'resolved';
  }
  const msRemaining = new Date(market.ends_at).getTime() - Date.now();
  return msRemaining < CLOSING_SOON_MS ? 'closing-soon' : 'active';
}

const STATUS_LABEL: Record<RowStatus, string> = {
  active: 'Active',
  'closing-soon': 'Closing soon',
  resolved: 'Resolved',
};

// Assets that are fiat-equivalent and can be rendered with a currency symbol.
// Anything else (e.g. XLM or other supported tokens) falls back to a neutral
// numeric format so we never show a misleading `$` value.
const FIAT_CURRENCIES: Record<string, string> = {
  USD: 'USD',
  USDC: 'USD',
  USDT: 'USD',
};

const currencyFormatters = new Map<string, Intl.NumberFormat>();

function getCurrencyFormatter(currency: string): Intl.NumberFormat {
  let formatter = currencyFormatters.get(currency);
  if (!formatter) {
    formatter = new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency,
      notation: 'compact',
      maximumFractionDigits: 1,
    });
    currencyFormatters.set(currency, formatter);
  }
  return formatter;
}

const neutralFormatter = new Intl.NumberFormat('en-US', {
  notation: 'compact',
  maximumFractionDigits: 1,
});

function formatVolume(raw: string, asset?: string | null): string {
  const value = Number.parseFloat(raw);
  if (!Number.isFinite(value)) {
    return raw;
  }
  const currency = asset ? FIAT_CURRENCIES[asset.toUpperCase()] : undefined;
  return currency ? getCurrencyFormatter(currency).format(value) : neutralFormatter.format(value);
}

export function LiveMarketsTicker() {
  const fetchMarkets = React.useCallback((signal: AbortSignal) => api.getFeaturedMarkets(signal), []);
  const { data, status, retry } = useAsync<FeaturedMarket[]>(fetchMarkets, { immediate: true });
  const markets = (Array.isArray(data) ? data : []).slice(0, MAX_ROWS);

  // Guard against duplicate in-flight retries: while a retry is loading, the
  // button is disabled and further clicks are no-ops, so a rapid double-click
  // results in a single network call to getFeaturedMarkets.
  const retryInFlight = status === 'loading';
  const handleRetry = React.useCallback(() => {
    if (retryInFlight) {
      return;
    }
    retry();
  }, [retry, retryInFlight]);

  return (
    // A plain div, not <aside>: it already sits inside the hero <section>,
    // which itself is a labelled (and therefore landmark) region - nesting a
    // complementary landmark inside another landmark fails
    // landmark-complementary-is-top-level.
    <div className="live-ticker" aria-labelledby="live-ticker-heading">
      <div className="live-ticker__header">
        <h2 id="live-ticker-heading" className="live-ticker__heading">
          Live Markets
        </h2>
        <Link href="/markets" className="live-ticker__view-all">
          View all
        </Link>
      </div>

      {status === 'loading' && (
        <p className="live-ticker__status" role="status" aria-live="polite">
          Loading live markets…
        </p>
      )}

      {status === 'error' && (
        <div className="live-ticker__status" role="alert" aria-live="assertive">
          <p>Unable to load live markets right now.</p>
          <button
            type="button"
            className="retry-button"
            onClick={handleRetry}
            disabled={retryInFlight}
            aria-busy={retryInFlight}
          >
            {retryInFlight ? 'Retrying…' : 'Retry'}
          </button>
        </div>
      )}

      {status === 'success' && markets.length === 0 && (
        <p className="live-ticker__status">No live markets yet — check back soon.</p>
      )}

      {status === 'success' && markets.length > 0 && (
        <ul className="live-ticker__list">
          {markets.map((market) => {
            const rowStatus = deriveRowStatus(market);
            return (
              <li key={market.id} className="live-ticker__row">
                <Link href={`/markets/${market.id}`} className="live-ticker__row-link">
                  <span className="live-ticker__title">{market.title}</span>
                  <span className={`live-ticker__tag live-ticker__tag--${rowStatus}`}>
                    {STATUS_LABEL[rowStatus]}
                  </span>
                  <span className="live-ticker__volume mono tabular-nums">
                    {formatVolume(market.volume, market.settlement_asset)}
                  </span>
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

export default LiveMarketsTicker;
