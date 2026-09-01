import React from 'react';
import { useAsync } from '../../lib/hooks/useAsync';
import { api, ApiError, isMarketNotResolvedError } from '../../lib/api/public-client';
import { LoadingSpinner } from '../LoadingSpinner';
import { ResolutionPendingNotice } from './ResolutionPendingNotice';
import './MarketDetailView.css';

interface MarketData {
  status?: string;
  winning_outcome?: number | null;
  description?: string;
  [key: string]: unknown;
}

interface PayoutViewState {
  pending: boolean;
  message?: string;
}

interface MarketDetailViewProps {
  marketId: string;
}

/**
 * Minimal market detail page: fetches the on-chain market and exposes a
 * "View payout details" action. Before resolution, error 147
 * (`MarketNotResolved`) is a normal, expected response — this maps it (and
 * the equivalent client-side "not resolved yet" status check) to the
 * informational `ResolutionPendingNotice` instead of a red error toast (see
 * #1369). Any other failure is shown as a real error.
 */
export const MarketDetailView: React.FC<MarketDetailViewProps> = ({ marketId }) => {
  const fetchMarket = React.useCallback(
    (signal: AbortSignal) => api.getBlockchainMarket(marketId, signal),
    [marketId],
  );
  const { data, status, error, retry } = useAsync<MarketData>(fetchMarket, { immediate: true });
  const loading = status === 'loading';

  const [payoutState, setPayoutState] = React.useState<PayoutViewState | null>(null);
  const [payoutLoading, setPayoutLoading] = React.useState(false);

  const isResolved =
    data?.status === 'Resolved' || (data?.winning_outcome !== null && data?.winning_outcome !== undefined);

  const handleViewPayoutDetails = React.useCallback(async () => {
    if (!isResolved) {
      // Expected, routine state — don't even attempt the request.
      setPayoutState({ pending: true });
      return;
    }

    setPayoutLoading(true);
    setPayoutState(null);
    try {
      // No dedicated payout-details endpoint exists yet (#58); the oracle
      // result read is the closest available public signal today and
      // surfaces the same contract error 147 while the market is unresolved.
      await api.getOracleResult(marketId);
      setPayoutState(null);
    } catch (err) {
      if (isMarketNotResolvedError(err)) {
        setPayoutState({ pending: true });
      } else {
        setPayoutState({
          pending: false,
          message: err instanceof ApiError ? err.message : 'Failed to load payout details.',
        });
      }
    } finally {
      setPayoutLoading(false);
    }
  }, [marketId, isResolved]);

  if (loading && !data) {
    return (
      <section className="market-detail" aria-label="Market details">
        <LoadingSpinner aria-label="Loading market" />
      </section>
    );
  }

  if (error) {
    return (
      <section className="market-detail" aria-label="Market details">
        <div className="market-detail__error" role="alert">
          <p>Failed to load this market. Please try again.</p>
          <button type="button" className="retry-button" onClick={() => retry()}>
            Retry
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className="market-detail" aria-label="Market details">
      {data?.description != null && <h1 className="market-detail__title">{String(data.description)}</h1>}

      <div className="market-detail__payout">
        <button
          type="button"
          className="retry-button"
          onClick={handleViewPayoutDetails}
          disabled={payoutLoading}
        >
          {payoutLoading ? 'Loading…' : 'View payout details'}
        </button>

        {payoutState?.pending && <ResolutionPendingNotice className="market-detail__payout-notice" />}

        {payoutState && !payoutState.pending && (
          <div className="market-detail__payout-error" role="alert">
            {payoutState.message}
          </div>
        )}
      </div>
    </section>
  );
};
