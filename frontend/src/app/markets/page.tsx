'use client';

import React, { Suspense } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useAsync } from '../../lib/hooks/useAsync';
import { api } from '../../lib/api/public-client';
import { LoadingSpinner } from '../../components/LoadingSpinner';
import type { components } from '../../lib/api/schema';
import './markets.css';

// The backend's FeaturedMarketView doesn't define a `category` field yet, so it's
// read defensively when present rather than assumed by the type.
type FeaturedMarket = components['schemas']['FeaturedMarketView'] & { category?: string };

type MarketStatus = 'active' | 'closed' | 'resolved';

const PAGE_SIZE = 12;

function deriveStatus(market: FeaturedMarket): MarketStatus {
  if (market.resolved_outcome !== null && market.resolved_outcome !== undefined) {
    return 'resolved';
  }
  const endsAt = new Date(market.ends_at).getTime();
  if (!Number.isNaN(endsAt) && endsAt < Date.now()) {
    return 'closed';
  }
  return 'active';
}

function resolvePage(pageParam: string | null): number {
  const parsed = Number.parseInt(pageParam ?? '1', 10);
  return Number.isFinite(parsed) && parsed >= 1 ? parsed : 1;
}

function MarketsListing() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const category = searchParams.get('category')?.trim() || 'all';
  const status = (searchParams.get('status') as MarketStatus | 'all' | null) ?? 'all';
  const page = resolvePage(searchParams.get('page'));

  const fetchMarkets = React.useCallback((signal: AbortSignal) => api.getFeaturedMarkets(signal), []);
  const { data, status: loadStatus, error, retry } = useAsync<FeaturedMarket[]>(fetchMarkets, { immediate: true });
  const loading = loadStatus === 'loading';

  const markets = data ?? [];

  const availableCategories = React.useMemo(
    () => Array.from(new Set(markets.map((m) => m.category).filter((c): c is string => !!c))).sort(),
    [markets],
  );

  const filteredMarkets = React.useMemo(() => {
    return markets.filter((market) => {
      const matchesCategory = category === 'all' || market.category === category;
      const matchesStatus = status === 'all' || deriveStatus(market) === status;
      return matchesCategory && matchesStatus;
    });
  }, [markets, category, status]);

  const totalPages = Math.max(1, Math.ceil(filteredMarkets.length / PAGE_SIZE));
  const currentPage = Math.min(page, totalPages);
  const pageMarkets = filteredMarkets.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  const updateFilters = (next: { category?: string; status?: string; page?: number }) => {
    const params = new URLSearchParams();
    const nextCategory = next.category ?? category;
    const nextStatus = next.status ?? status;
    const nextPage = next.page ?? 1;
    if (nextCategory !== 'all') params.set('category', nextCategory);
    if (nextStatus !== 'all') params.set('status', nextStatus);
    if (nextPage > 1) params.set('page', String(nextPage));
    const query = params.toString();
    router.push(query ? `/markets?${query}` : '/markets', { scroll: false });
  };

  if (error) {
    return (
      <section className="markets-page" aria-labelledby="markets-heading">
        <h1 id="markets-heading">Markets</h1>
        <div className="error-message" role="alert">
          <p>Failed to load markets. Please try again.</p>
          <button onClick={() => retry()} className="retry-button" type="button">
            Retry
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className="markets-page" aria-labelledby="markets-heading">
      <header className="markets-page__header">
        <h1 id="markets-heading">Markets</h1>
      </header>

      <form className="markets-filters" aria-label="Market filters" onSubmit={(e) => e.preventDefault()}>
        <label>
          Category
          <select value={category} onChange={(e) => updateFilters({ category: e.target.value })}>
            <option value="all">All categories</option>
            {availableCategories.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <label>
          Status
          <select value={status} onChange={(e) => updateFilters({ status: e.target.value })}>
            <option value="all">All statuses</option>
            <option value="active">Active</option>
            <option value="closed">Closed</option>
            <option value="resolved">Resolved</option>
          </select>
        </label>
      </form>

      {loading ? (
        <LoadingSpinner size="large" aria-label="Loading markets" />
      ) : filteredMarkets.length === 0 ? (
        <div className="markets-empty-state" role="status">
          <p>No markets found for the current filters.</p>
        </div>
      ) : (
        <>
          <div className="markets-grid">
            {pageMarkets.map((market) => {
              const marketStatus = deriveStatus(market);
              return (
                <article className="market-card" key={market.id}>
                  <span className={`market-status market-status--${marketStatus}`}>{marketStatus}</span>
                  <h2>{market.title}</h2>
                  <dl>
                    <div>
                      <dt>Volume</dt>
                      <dd>${market.volume.toLocaleString()}</dd>
                    </div>
                    <div>
                      <dt>Ends</dt>
                      <dd>{new Date(market.ends_at).toLocaleDateString()}</dd>
                    </div>
                  </dl>
                </article>
              );
            })}
          </div>

          <nav className="markets-pagination" aria-label="Markets pagination">
            <button
              type="button"
              onClick={() => updateFilters({ page: currentPage - 1 })}
              disabled={currentPage <= 1}
            >
              Previous
            </button>
            <span>
              Page {currentPage} of {totalPages}
            </span>
            <button
              type="button"
              onClick={() => updateFilters({ page: currentPage + 1 })}
              disabled={currentPage >= totalPages}
            >
              Next
            </button>
          </nav>
        </>
      )}
    </section>
  );
}

export default function MarketsPage() {
  return (
    <Suspense fallback={<LoadingSpinner size="large" aria-label="Loading markets" />}>
      <MarketsListing />
    </Suspense>
  );
}
