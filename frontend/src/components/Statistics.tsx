import React from 'react';
import { useAsync } from '../lib/hooks/useAsync';
import { useOnlineStatus } from '../lib/hooks/useOnlineStatus';
import { api } from '../lib/api/public-client';
import { LoadingSpinner } from './LoadingSpinner';
import { Skeleton } from './Skeleton';
import './Statistics.css';

// Auto-refresh cadence while the tab is visible and online; entirely paused
// while hidden or offline (see useOnlineStatus / OfflineBanner) so we don't
// spend the retry budget on requests that are known to fail.
const REFRESH_INTERVAL_MS = 30_000;

// Soft-retry budget (#1352): transient fetch failures retry automatically with
// exponential backoff (0.5s, 1s, 2s) while the last-successful data stays
// visible (stale-while-revalidating). Only after the budget is exhausted does
// the component surface a hard error state with a manual retry action.
const MAX_RETRIES = 3;
const RETRY_BACKOFF_MS = (attempt: number): number => 2 ** attempt * 500;

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
  const { data, status, error, retry } = useAsync<StatisticsData>(fetchStatistics, {
    immediate: true,
    retries: MAX_RETRIES,
    retryDelayMs: RETRY_BACKOFF_MS,
  });
  const isOnline = useOnlineStatus();
  const loading = status === 'loading';

  // Poll /api/v1/statistics while the tab is visible and online; pause while
  // hidden or offline and resume (with an immediate refetch) on refocus or
  // reconnect. A single shared interval ref plus clear-before-set guarantees
  // rapid visibility toggling never stacks overlapping poll intervals.
  const pollTimerRef = React.useRef<ReturnType<typeof setInterval> | null>(null);
  const wasOnlineRef = React.useRef(isOnline);

  React.useEffect(() => {
    const wasOnline = wasOnlineRef.current;
    wasOnlineRef.current = isOnline;

    const stopPolling = () => {
      if (pollTimerRef.current !== null) {
        clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
    };

    if (!isOnline) return stopPolling;

    const startPolling = () => {
      stopPolling();
      if (document.visibilityState === 'visible') {
        pollTimerRef.current = setInterval(() => {
          void retry();
        }, REFRESH_INTERVAL_MS);
      }
    };

    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        void retry(); // resume with an immediate refetch
        startPolling();
      } else {
        stopPolling();
      }
    };

    // Reconnect (offline -> online): refetch right away, then poll.
    if (!wasOnline) {
      void retry();
    }
    startPolling();
    document.addEventListener('visibilitychange', handleVisibilityChange);

    return () => {
      stopPolling();
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [isOnline, retry]);

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
    retry();
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

  // Skeleton tiles (#1353) only while the initial fetch is in flight; once a
  // successful payload exists it stays visible (stale-while-revalidating)
  // through background refreshes so the grid never blanks or shifts.
  const showSkeleton = loading && !data;

  return (
    <section className="statistics" aria-labelledby="statistics-heading">
      <h2 id="statistics-heading">Platform Statistics</h2>
      <div className="stats-grid">
        <div className="stat-item">
          <h3>Total Markets</h3>
          {showSkeleton ? (
            <Skeleton className="stat-skeleton stat-skeleton--markets" aria-label="Loading total markets" />
          ) : (
            <p className="stat-value" aria-live="polite">
              {displayValues.totalMarkets}
            </p>
          )}
        </div>
        <div className="stat-item">
          <h3>Total Volume</h3>
          {showSkeleton ? (
            <Skeleton className="stat-skeleton stat-skeleton--volume" aria-label="Loading total volume" />
          ) : (
            <p className="stat-value" aria-live="polite">
              {displayValues.totalVolume}
            </p>
          )}
        </div>
        <div className="stat-item">
          <h3>Active Markets</h3>
          {showSkeleton ? (
            <Skeleton className="stat-skeleton stat-skeleton--active-markets" aria-label="Loading active markets" />
          ) : (
            <p className="stat-value" aria-live="polite">
              {displayValues.activeMarkets}
            </p>
          )}
        </div>
      </div>
      {showSkeleton && (
        <div className="loading-overlay" aria-live="polite">
          <LoadingSpinner size="large" aria-label="Loading statistics data" />
          <p>Loading statistics...</p>
        </div>
      )}
    </section>
  );
};
