'use client';

import React from 'react';
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

const volumeFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  notation: 'compact',
  maximumFractionDigits: 1,
});

function formatVolume(raw: string): string {
  const value = Number.parseFloat(raw);
  return Number.isFinite(value) ? volumeFormatter.format(value) : raw;
}

export function LiveMarketsTicker() {
  const fetchMarkets = React.useCallback((signal: AbortSignal) => api.getFeaturedMarkets(signal), []);
  const { data, status, retry } = useAsync<FeaturedMarket[]>(fetchMarkets, { immediate: true });
  const markets = (Array.isArray(data) ? data : []).slice(0, MAX_ROWS);

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
        <a href="/markets" className="live-ticker__view-all">
          View all
        </a>
      </div>

      {status === 'loading' && (
        <p className="live-ticker__status" role="status" aria-live="polite">
          Loading live markets…
        </p>
      )}

      {status === 'error' && (
        <div className="live-ticker__status" role="alert" aria-live="assertive">
          <p>Unable to load live markets right now.</p>
          <button type="button" className="retry-button" onClick={retry}>
            Retry
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
                <a href={`/markets/${market.id}`} className="live-ticker__row-link">
                  <span className="live-ticker__title">{market.title}</span>
                  <span className={`live-ticker__tag live-ticker__tag--${rowStatus}`}>
                    {STATUS_LABEL[rowStatus]}
                  </span>
                  <span className="live-ticker__volume mono tabular-nums">
                    {formatVolume(market.volume)}
                  </span>
                </a>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

export default LiveMarketsTicker;
