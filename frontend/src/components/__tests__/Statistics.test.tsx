import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { Statistics } from '../Statistics';
import { ErrorBoundary } from '../ErrorBoundary';
import { api } from '../../lib/api/public-client';

// Mock the API
jest.mock('../../lib/api/public-client', () => ({
  api: {
    getStatistics: jest.fn(),
  },
}));

const mockApi = api as jest.Mocked<typeof api>;

describe('Statistics', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('shows loading state initially', () => {
    mockApi.getStatistics.mockResolvedValue({});
    render(<Statistics />);

    expect(screen.getByRole('status', { name: /loading statistics data/i })).toBeInTheDocument();
    expect(screen.getAllByRole('status', { name: /loading/i })).toHaveLength(5); // 4 skeletons + 1 spinner
  });

  it('renders real data from a mocked /api/v1/statistics response', async () => {
    // Shape returned by the real backend (services/api/src/db.rs Statistics
    // struct): snake_case, `total_volume` an exact decimal string, plus an
    // extra field the UI ignores to prove unknown keys are tolerated.
    const mockData = {
      total_markets: 150,
      total_volume: '2500000.50',
      active_markets: 50000,
      resolved_markets: 12,
      unmapped_future_field: 'ignored',
    };
    mockApi.getStatistics.mockResolvedValue(mockData);

    render(<Statistics />);

    await waitFor(() => {
      expect(screen.getByText('150')).toBeInTheDocument();
    });

    expect(screen.getByText('$2,500,000.5')).toBeInTheDocument();
    expect(screen.getByText('50,000')).toBeInTheDocument();
    expect(screen.getByText('12')).toBeInTheDocument();
    expect(screen.queryByRole('status', { name: /loading/i })).not.toBeInTheDocument();
  });

  it('renders every metric field as 0 for an all-zero / empty response', async () => {
    // A fresh deployment: the endpoint returns an object with no populated
    // metrics. Every tile must show `0` / `$0`, never `undefined` or blank.
    mockApi.getStatistics.mockResolvedValue({});

    render(<Statistics />);

    await waitFor(() => {
      expect(screen.queryByRole('status', { name: /loading/i })).not.toBeInTheDocument();
    });

    const values = screen.getAllByText(/^\$?0$/).map((el) => el.textContent);
    expect(values).toEqual(['0', '$0', '0', '0']);
  });

  it('shows error state and retry button on failure', async () => {
    const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
    mockApi.getStatistics.mockRejectedValue(new Error('Network error'));

    render(<Statistics />);

    await waitFor(() => {
      expect(screen.getByText(/failed to load statistics/i)).toBeInTheDocument();
    });

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();

    consoleSpy.mockRestore();
  });

  it('retries on button click', async () => {
    mockApi.getStatistics
      .mockRejectedValueOnce(new Error('Network error'))
      .mockResolvedValueOnce({ total_markets: 100 });

    render(<Statistics />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /retry/i }));

    await waitFor(() => {
      expect(screen.getByText('100')).toBeInTheDocument();
    });

    expect(mockApi.getStatistics).toHaveBeenCalledTimes(2);
  });

  it('has proper accessibility attributes', () => {
    mockApi.getStatistics.mockResolvedValue({});
    render(<Statistics />);

    expect(screen.getByRole('region', { name: /platform statistics/i })).toBeInTheDocument();
  });
});

// Component that unconditionally throws to simulate a rendering exception in Statistics
const ThrowingStatistics: React.FC = () => {
  throw new Error('Statistics rendering exception');
};

describe('Statistics wrapped in ErrorBoundary', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    // Suppress console.error for expected error boundary output
    jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('catches a rendering exception thrown inside Statistics and renders the fallback', () => {
    const fallback = (
      <section className="statistics" aria-labelledby="statistics-heading">
        <h2 id="statistics-heading">Platform Statistics</h2>
        <div className="error-message" role="alert">
          <p>Unable to load statistics at this time. Please try again later.</p>
          <button className="retry-button" onClick={() => window.location.reload()} aria-label="Retry loading statistics">
            Retry
          </button>
        </div>
      </section>
    );

    render(
      <ErrorBoundary section="statistics" fallback={fallback}>
        <ThrowingStatistics />
      </ErrorBoundary>
    );

    // Fallback heading and message should be visible
    expect(screen.getByRole('heading', { name: /platform statistics/i })).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/unable to load statistics at this time/i)).toBeInTheDocument();
  });

  it('renders a retry button in the fallback when Statistics throws', () => {
    const fallback = (
      <section className="statistics" aria-labelledby="statistics-heading">
        <h2 id="statistics-heading">Platform Statistics</h2>
        <div className="error-message" role="alert">
          <p>Unable to load statistics at this time. Please try again later.</p>
          <button className="retry-button" onClick={() => window.location.reload()} aria-label="Retry loading statistics">
            Retry
          </button>
        </div>
      </section>
    );

    render(
      <ErrorBoundary section="statistics" fallback={fallback}>
        <ThrowingStatistics />
      </ErrorBoundary>
    );

    expect(screen.getByRole('button', { name: /retry loading statistics/i })).toBeInTheDocument();
  });

  it('does not render the Statistics content when an error is thrown', () => {
    const fallback = (
      <div role="alert">
        <p>Unable to load statistics at this time. Please try again later.</p>
        <button className="retry-button" aria-label="Retry loading statistics">Retry</button>
      </div>
    );

    render(
      <ErrorBoundary section="statistics" fallback={fallback}>
        <ThrowingStatistics />
      </ErrorBoundary>
    );

    // Normal statistics content should not be present
    expect(screen.queryByText('Total Markets')).not.toBeInTheDocument();
    expect(screen.queryByText('Total Volume')).not.toBeInTheDocument();
    expect(screen.queryByText('Active Markets')).not.toBeInTheDocument();

    // Fallback should be visible instead
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});
