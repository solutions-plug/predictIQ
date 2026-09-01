'use client';

import React, { Suspense } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useAsync } from '../../lib/hooks/useAsync';
import { api } from '../../lib/api/public-client';
import { LoadingSpinner } from '../../components/LoadingSpinner';
import { ExportButton, type ExportSection } from '../../components/statistics/ExportButton';
import { VolumeChart } from '../../components/statistics/VolumeChart';
import './statistics-dashboard.css';

interface CategoryBreakdown {
  category: string;
  count: number;
  volume: number;
  [key: string]: string | number;
}

interface HistoryPoint {
  date: string;
  markets: number;
  volume: number;
  [key: string]: string | number;
}

interface StatisticsData {
  total_markets?: number;
  total_volume?: number;
  active_markets?: number;
  by_category?: unknown;
  history?: unknown;
  [key: string]: unknown;
}

const ISO_DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

function toIsoDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

function isValidIsoDate(value: string): boolean {
  if (!ISO_DATE_RE.test(value)) return false;
  const parsed = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(parsed.getTime()) && toIsoDate(parsed) === value;
}

const DEFAULT_RANGE_DAYS = 30;

function defaultDateRange(): { start: string; end: string } {
  const end = toIsoDate(new Date());
  const start = toIsoDate(new Date(Date.now() - DEFAULT_RANGE_DAYS * 24 * 60 * 60 * 1000));
  return { start, end };
}

/** Falls back to the default 30-day range on any malformed or out-of-range input (e.g. a future end-date). */
function resolveDateRange(startParam: string | null, endParam: string | null): { start: string; end: string } {
  const defaults = defaultDateRange();
  if (!startParam || !endParam) return defaults;
  if (!isValidIsoDate(startParam) || !isValidIsoDate(endParam)) return defaults;
  if (endParam > defaults.end) return defaults;
  if (startParam > endParam) return defaults;
  return { start: startParam, end: endParam };
}

function resolveCategory(categoryParam: string | null): string {
  const trimmed = categoryParam?.trim() ?? '';
  return trimmed.length > 0 && trimmed.length <= 64 ? trimmed : 'all';
}

function isCategoryBreakdown(value: unknown): value is CategoryBreakdown {
  const row = value as Partial<CategoryBreakdown> | null;
  return (
    !!row &&
    typeof row.category === 'string' &&
    typeof row.count === 'number' &&
    typeof row.volume === 'number'
  );
}

function isHistoryPoint(value: unknown): value is HistoryPoint {
  const row = value as Partial<HistoryPoint> | null;
  return (
    !!row &&
    typeof row.date === 'string' &&
    isValidIsoDate(row.date.slice(0, 10)) &&
    typeof row.markets === 'number' &&
    typeof row.volume === 'number'
  );
}

// The backend's statistics payload is an untyped AnyObject (services/api/src/db.rs
// doesn't yet formalize a breakdown/history shape), so these fields are read
// defensively and simply omitted from the dashboard when absent.
function normalizeStatistics(data: StatisticsData | null) {
  const summary = {
    totalMarkets: typeof data?.total_markets === 'number' ? data.total_markets : 0,
    totalVolume: typeof data?.total_volume === 'number' ? data.total_volume : 0,
    activeMarkets: typeof data?.active_markets === 'number' ? data.active_markets : 0,
  };
  const categories = Array.isArray(data?.by_category)
    ? (data?.by_category as unknown[]).filter(isCategoryBreakdown)
    : [];
  const history = Array.isArray(data?.history) ? (data?.history as unknown[]).filter(isHistoryPoint) : [];
  return { summary, categories, history };
}

function StatisticsDashboard() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const { start, end } = resolveDateRange(searchParams.get('start'), searchParams.get('end'));
  const category = resolveCategory(searchParams.get('category'));

  const fetchStatistics = React.useCallback((signal: AbortSignal) => api.getStatistics(signal), []);
  const { data, status, error, retry } = useAsync<StatisticsData>(fetchStatistics, { immediate: true });
  const loading = status === 'loading';

  const { summary, categories, history } = React.useMemo(() => normalizeStatistics(data ?? null), [data]);

  const availableCategories = React.useMemo(
    () => Array.from(new Set(categories.map((row) => row.category))).sort(),
    [categories],
  );

  const filteredCategories = React.useMemo(
    () => (category === 'all' ? categories : categories.filter((row) => row.category === category)),
    [categories, category],
  );

  const filteredHistory = React.useMemo(
    () => history.filter((row) => row.date.slice(0, 10) >= start && row.date.slice(0, 10) <= end),
    [history, start, end],
  );

  // Normalize an invalid/incomplete URL to the resolved (fallback) filters instead of leaving
  // a malformed query string in the address bar.
  React.useEffect(() => {
    const next = new URLSearchParams();
    next.set('start', start);
    next.set('end', end);
    if (category !== 'all') next.set('category', category);
    const nextQuery = next.toString();
    if (nextQuery === searchParams.toString()) return;
    router.replace(`/statistics?${nextQuery}`, { scroll: false });
    // Only re-run when the resolved filter values themselves change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [start, end, category]);

  const updateFilters = (next: { start?: string; end?: string; category?: string }) => {
    const params = new URLSearchParams();
    params.set('start', next.start ?? start);
    params.set('end', next.end ?? end);
    const nextCategory = next.category ?? category;
    if (nextCategory !== 'all') params.set('category', nextCategory);
    router.push(`/statistics?${params.toString()}`, { scroll: false });
  };

  const exportSections: ExportSection[] = [
    {
      title: 'Summary',
      rows: [
        {
          total_markets: summary.totalMarkets,
          total_volume: summary.totalVolume,
          active_markets: summary.activeMarkets,
        },
      ],
    },
    { title: 'Categories', rows: filteredCategories },
    { title: 'History', rows: filteredHistory },
  ];

  if (error) {
    return (
      <section className="statistics-dashboard" aria-labelledby="statistics-dashboard-heading">
        <h1 id="statistics-dashboard-heading">Statistics Dashboard</h1>
        <div className="error-message" role="alert">
          <p>Failed to load statistics. Please try again.</p>
          <button onClick={() => retry()} className="retry-button" type="button">
            Retry
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className="statistics-dashboard" aria-labelledby="statistics-dashboard-heading">
      <header className="statistics-dashboard__header">
        <h1 id="statistics-dashboard-heading">Statistics Dashboard</h1>
        <ExportButton sections={exportSections} filenamePrefix="predictiq-statistics" disabled={loading} />
      </header>

      <form className="statistics-filters" aria-label="Statistics filters" onSubmit={(e) => e.preventDefault()}>
        <label>
          Start date
          <input
            type="date"
            value={start}
            max={end}
            onChange={(e) => updateFilters({ start: e.target.value })}
          />
        </label>
        <label>
          End date
          <input
            type="date"
            value={end}
            min={start}
            max={defaultDateRange().end}
            onChange={(e) => updateFilters({ end: e.target.value })}
          />
        </label>
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
      </form>

      {loading ? (
        <LoadingSpinner size="large" aria-label="Loading statistics" />
      ) : (
        <>
          <div className="stats-grid">
            <div className="stat-item">
              <h3>Total Markets</h3>
              <p className="stat-value">{summary.totalMarkets.toLocaleString()}</p>
            </div>
            <div className="stat-item">
              <h3>Total Volume</h3>
              <p className="stat-value">${summary.totalVolume.toLocaleString()}</p>
            </div>
            <div className="stat-item">
              <h3>Active Markets</h3>
              <p className="stat-value">{summary.activeMarkets.toLocaleString()}</p>
            </div>
          </div>

          <table className="statistics-table">
            <caption>Markets by category</caption>
            <thead>
              <tr>
                <th scope="col">Category</th>
                <th scope="col">Markets</th>
                <th scope="col">Volume</th>
              </tr>
            </thead>
            <tbody>
              {filteredCategories.length === 0 ? (
                <tr>
                  <td colSpan={3}>No category data for the current filter.</td>
                </tr>
              ) : (
                filteredCategories.map((row) => (
                  <tr key={row.category}>
                    <td>{row.category}</td>
                    <td>{row.count.toLocaleString()}</td>
                    <td>${row.volume.toLocaleString()}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>

          <section className="statistics-trends" aria-labelledby="statistics-trends-heading">
            <h2 id="statistics-trends-heading">Trends</h2>
            <VolumeChart data={filteredHistory} />
          </section>

          <table className="statistics-table">
            <caption>
              Volume history ({start} to {end})
            </caption>
            <thead>
              <tr>
                <th scope="col">Date</th>
                <th scope="col">Markets</th>
                <th scope="col">Volume</th>
              </tr>
            </thead>
            <tbody>
              {filteredHistory.length === 0 ? (
                <tr>
                  <td colSpan={3}>No history data for the selected date range.</td>
                </tr>
              ) : (
                filteredHistory.map((row) => (
                  <tr key={row.date}>
                    <td>{row.date}</td>
                    <td>{row.markets.toLocaleString()}</td>
                    <td>${row.volume.toLocaleString()}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </>
      )}
    </section>
  );
}

export default function StatisticsPage() {
  return (
    <Suspense fallback={<LoadingSpinner size="large" aria-label="Loading statistics" />}>
      <StatisticsDashboard />
    </Suspense>
  );
}
