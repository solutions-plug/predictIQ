'use client';

import { use, useEffect, useState } from 'react';
import { api, ApiError, getContractErrorMessage } from '@/lib/api/admin-client';
import { canResolveMarket, parseMarketView, type MarketView } from '@/lib/markets/marketState';
import './resolve.css';

/**
 * Resolution UI for market creators (#1377).
 *
 * `contracts/predict-iq/src/test_resolution_state_machine.rs` defines the
 * allowed transitions — a market must have already left `Active` and be
 * sitting in `PendingResolution` (oracle/vote outcome recorded, dispute
 * window elapsed) before `finalize_resolution` may run. This page mirrors
 * that via `lib/markets/marketState.ts::canResolveMarket` so the control is
 * disabled/hidden for a market that is not yet eligible, rather than
 * presenting an action that would fail on submit.
 *
 * `POST /api/v1/markets/{market_id}/resolve` already exists as
 * `api.resolveMarket` in `lib/api/admin-client.ts` — this page is the first
 * UI consumer of it.
 */

type SubmitState = 'idle' | 'submitting' | 'success' | 'error';

export default function ResolveMarketPage({ params }: { params: Promise<{ marketId: string }> }) {
  const { marketId } = use(params);

  const [market, setMarket] = useState<MarketView | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [submitState, setSubmitState] = useState<SubmitState>('idle');
  const [submitError, setSubmitError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    api
      .getBlockchainMarket(marketId, controller.signal)
      .then((raw) => {
        const view = parseMarketView(raw);
        if (!view) {
          setLoadError('Could not read this market’s on-chain status.');
          return;
        }
        setMarket(view);
      })
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === 'AbortError') return;
        setLoadError(err instanceof Error ? err.message : 'Failed to load market.');
      });

    return () => controller.abort();
  }, [marketId]);

  const gate = canResolveMarket(market);

  const handleResolve = async () => {
    setSubmitState('submitting');
    setSubmitError(null);
    try {
      await api.resolveMarket(marketId);
      setSubmitState('success');
    } catch (err) {
      setSubmitState('error');
      if (err instanceof ApiError && err.code === 'CONTRACT_ERROR' && typeof err.details?.['contract_code'] === 'number') {
        setSubmitError(getContractErrorMessage(err.details['contract_code'] as number));
      } else {
        setSubmitError(err instanceof Error ? err.message : 'Failed to resolve market.');
      }
    }
  };

  if (loadError) {
    return (
      <main className="resolve-market-page">
        <p role="alert" className="resolve-market-page__error">{loadError}</p>
      </main>
    );
  }

  if (submitState === 'success') {
    return (
      <main className="resolve-market-page">
        <p role="status">This market has been resolved. Winners can now claim their payout.</p>
      </main>
    );
  }

  return (
    <main className="resolve-market-page">
      <h1 className="resolve-market-page__title">Resolve market</h1>

      {!market ? (
        <p role="status" aria-live="polite">Loading market…</p>
      ) : (
        <>
          {!gate.allowed && (
            <p role="alert" className="resolve-market-page__blocked">{gate.reason}</p>
          )}

          <button
            type="button"
            disabled={!gate.allowed || submitState === 'submitting'}
            onClick={handleResolve}
            className="resolve-market-page__action"
          >
            {submitState === 'submitting' ? 'Resolving…' : 'Resolve this market'}
          </button>

          {submitError && (
            <p role="alert" className="resolve-market-page__error">{submitError}</p>
          )}
        </>
      )}
    </main>
  );
}
