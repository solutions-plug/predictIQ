import React from 'react';
import { useAsync } from '../lib/hooks/useAsync';
import { useOnlineStatus } from '../lib/hooks/useOnlineStatus';
import { api } from '../lib/api/public-client';
import { LoadingSpinner } from './LoadingSpinner';
import { Skeleton } from './Skeleton';
import './Statistics.css';

// Auto-refresh cadence while online; entirely paused while offline (see
// useOnlineStatus / OfflineBanner) so we don't spend the retry budget on
// requests that are known to fail.
const REFRESH_INTERVAL_MS = 30_000;

// Field names match the backend's Statistics struct (services/api/src/db.rs),
// which serializes as snake_case like every other typed response in this
// client. `total_volume` is an exact decimal string on the wire; the other
// counts are integers. Every field is optional here because /api/v1/statistics
// is typed as an untyped AnyObject and a fresh deployment may omit values.
interface StatisticsData {
  total_markets?: number | string;
  total_volume?: number | string;
  active_markets?: number | string;
  resolved_markets?: number | string;
  [key: string]: unknown;
}

// === Metric tiles
// The endpoint's response is not yet formalized, so the component renders a
// fixed set of known metrics and coerces every one to a number, defaulting to
// 0 for anything missing or non-numeric (issue #1351: a not-yet-populated
// field must render as `0`, never `undefined`/blank).
interface MetricTile {
  key: keyof StatisticsData;
  label: string;
  prefix?: string;
}

const METRIC_TILES: readonly MetricTile[] = [
  { key: 'total_markets', label: 'Total Markets' },
  { key: 'total_volume', label: 'Total Volume', prefix: '$' },
  { key: 'active_markets', label: 'Active Markets' },
  { key: 'resolved_markets', label: 'Resolved Markets' },
];

// Coerce a wire value (number, decimal string, or absent) to a finite number,
// falling back to 0 so a tile is never blank.
function toNumber(value: unknown): number {
  if (typeof value === 'number') return Number.isFinite(value) ? value : 0;
  if (typeof value === 'string' && value.trim() !== '') {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }
  return 0;
}

export const Statistics: React.FC = () => {
  const fetchStatistics = React.useCallback(
    (signal: AbortSignal) => api.getStatistics(signal),
    [],
  );
  const { data, status, error, retry } = useAsync<StatisticsData>(fetchStatistics, {
    immediate: true,
  });
  const isOnline = useOnlineStatus();
  const isLoading = status === 'loading';

  // Pause auto-refresh while offline (requests would just fail/retry for no
  // reason) and resume - with an immediate refetch - once connectivity is
  // restored.
  const wasOnlineRef = React.useRef(isOnline);
  React.useEffect(() => {
    if (isOnline && !wasOnlineRef.current) {
      void retry();
    }
    wasOnlineRef.current = isOnline;

    if (!isOnline) return undefined;

    const id = setInterval(() => {
      void retry();
    }, REFRESH_INTERVAL_MS);

    return () => clearInterval(id);
  }, [isOnline, retry]);

  const displayValues = React.useMemo(
    () =>
      METRIC_TILES.map((tile) => ({
        ...tile,
        display: `${tile.prefix ?? ''}${toNumber(data?.[tile.key]).toLocaleString()}`,
      })),
    [data],
  );

  const handleRetry = () => {
    void retry();
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
        {displayValues.map((tile) => (
          <div className="stat-item" key={String(tile.key)}>
            <h3>{tile.label}</h3>
            {isLoading ? (
              <Skeleton
                className="stat-skeleton"
                aria-label={`Loading ${tile.label.toLowerCase()}`}
              />
            ) : (
              <p className="stat-value" aria-live="polite">
                {tile.display}
              </p>
            )}
          </div>
        ))}
      </div>
      {isLoading && (
        <div className="loading-overlay" aria-live="polite">
          <LoadingSpinner size="large" aria-label="Loading statistics data" />
          <p>Loading statistics...</p>
        </div>
      )}
    </section>
  );
};
