'use client';

import { useEffect, useState } from 'react';
import type { Market } from '../../lib/types';
import { listMarkets, getFeaturedMarkets } from '../../lib/mock/markets';
import { EmptyState } from '../ui';
import { MarketCard } from './MarketCard';
import { MarketFilters, type MarketFilterValues, type StatusFilter } from './MarketFilters';
import './marketsBrowse.css';

const SEARCH_DEBOUNCE_MS = 250;
const SKELETON_COUNT = 6;

const INITIAL_FILTERS: MarketFilterValues = { category: '', status: '', query: '' };

function CardSkeleton() {
  return <div className="market-card-skeleton" aria-hidden="true" />;
}

export function MarketsExplorer() {
  const [filters, setFilters] = useState<MarketFilterValues>(INITIAL_FILTERS);
  const [debouncedQuery, setDebouncedQuery] = useState('');

  const [featured, setFeatured] = useState<Market[]>([]);
  const [markets, setMarkets] = useState<Market[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Featured strip loads once on mount.
  useEffect(() => {
    let active = true;
    getFeaturedMarkets(3)
      .then((data) => {
        if (active) setFeatured(data);
      })
      .catch(() => {
        /* featured is non-critical; the main grid surfaces errors */
      });
    return () => {
      active = false;
    };
  }, []);

  // Debounce only the free-text query so typing doesn't thrash the data layer.
  useEffect(() => {
    const id = setTimeout(() => setDebouncedQuery(filters.query), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(id);
  }, [filters.query]);

  // Re-query whenever a filter settles.
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    listMarkets({
      category: filters.category || undefined,
      status: filters.status || undefined,
      query: debouncedQuery || undefined,
    })
      .then((data) => {
        if (active) setMarkets(data);
      })
      .catch(() => {
        if (active) setError('We could not load markets. Please try again.');
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [filters.category, filters.status, debouncedQuery]);

  const updateCategory = (category: string) => setFilters((f) => ({ ...f, category }));
  const updateStatus = (status: StatusFilter) => setFilters((f) => ({ ...f, status }));
  const updateQuery = (query: string) => setFilters((f) => ({ ...f, query }));

  const hasResults = markets.length > 0;

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Markets</h1>
          <p>Trade on the outcomes that matter. Back your conviction with XLM.</p>
        </div>
      </div>

      {featured.length > 0 && (
        <section className="markets-featured" aria-labelledby="featured-heading">
          <h2 id="featured-heading" className="markets-featured__title">
            Featured
          </h2>
          <div className="markets-featured__strip">
            {featured.map((m) => (
              <MarketCard key={m.id} market={m} featured />
            ))}
          </div>
        </section>
      )}

      <section aria-labelledby="all-markets-heading">
        <h2 id="all-markets-heading" className="markets-section__title">
          All markets
        </h2>

        <MarketFilters
          values={filters}
          onCategoryChange={updateCategory}
          onStatusChange={updateStatus}
          onQueryChange={updateQuery}
        />

        {error ? (
          <EmptyState title="Something went wrong" message={error} />
        ) : loading ? (
          <div className="markets-grid" aria-busy="true" aria-label="Loading markets">
            {Array.from({ length: SKELETON_COUNT }).map((_, i) => (
              <CardSkeleton key={i} />
            ))}
          </div>
        ) : hasResults ? (
          <div className="markets-grid">
            {markets.map((m) => (
              <MarketCard key={m.id} market={m} />
            ))}
          </div>
        ) : (
          <EmptyState
            title="No markets match your filters"
            message="Try clearing the search or choosing a different category."
          />
        )}
      </section>
    </>
  );
}
