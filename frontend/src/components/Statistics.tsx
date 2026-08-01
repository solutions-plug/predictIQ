import React from 'react';
import { useAsync } from '../lib/hooks/useAsync';
import { api } from '../lib/api/public-client';
import { LoadingSpinner } from './LoadingSpinner';
import { Skeleton } from './Skeleton';
import './Statistics.css';

// Field names match the backend's Statistics struct (services/api/src/db.rs),
// which serializes as snake_case like every other typed response in this client.
interface StatisticsData {
  total_markets?: number;
  total_volume?: number;
  active_markets?: number;
  [key: string]: unknown;
}

export const Statistics: React.FC = () => {
  const fetchStatistics = React.useCallback((signal: AbortSignal) => api.getStatistics(signal), []);
  const { data, loading, error, execute } = useAsync<StatisticsData>(
    fetchStatistics,
    { immediate: true }
  );

  const displayValues = React.useMemo(
    () => ({
      totalMarkets:
        typeof data?.total_markets === 'number'
          ? data.total_markets.toLocaleString()
          : 'N/A',
      totalVolume:
        typeof data?.total_volume === 'number'
          ? `$${data.total_volume.toLocaleString()}`
          : '$N/A',
      activeMarkets:
        typeof data?.active_markets === 'number'
          ? data.active_markets.toLocaleString()
          : 'N/A',
    }),
    [data?.active_markets, data?.total_markets, data?.total_volume],
  );

  const handleRetry = () => {
    execute();
  };

  if (error) {
    return (
      <section className="statistics" aria-labelledby="statistics-heading">
        <h2 id="statistics-heading">Platform Statistics</h2>
        <div className="error-message" role="alert">
          <p>Failed to load statistics. Please try again.</p>
          <button onClick={handleRetry} className="retry-button">
            Retry
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className="statistics" aria-labelledby="statistics-heading">
      <h2 id="statistics-heading">Platform Statistics</h2>
      <div className="stats-grid">
        <div className="stat-item">
          <h3>Total Markets</h3>
          {loading ? (
            <Skeleton className="stat-skeleton stat-skeleton--markets" aria-label="Loading total markets" />
          ) : (
            <p className="stat-value" aria-live="polite">
              {displayValues.totalMarkets}
            </p>
          )}
        </div>
        <div className="stat-item">
          <h3>Total Volume</h3>
          {loading ? (
            <Skeleton className="stat-skeleton stat-skeleton--volume" aria-label="Loading total volume" />
          ) : (
            <p className="stat-value" aria-live="polite">
              {displayValues.totalVolume}
            </p>
          )}
        </div>
        <div className="stat-item">
          <h3>Active Markets</h3>
          {loading ? (
            <Skeleton className="stat-skeleton stat-skeleton--active-markets" aria-label="Loading active markets" />
          ) : (
            <p className="stat-value" aria-live="polite">
              {displayValues.activeMarkets}
            </p>
          )}
        </div>
      </div>
      {loading && (
        <div className="loading-overlay" aria-live="polite">
          <LoadingSpinner size="large" aria-label="Loading statistics data" />
          <p>Loading statistics...</p>
        </div>
      )}
    </section>
  );
};
