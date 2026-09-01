'use client';

import { use, useEffect, useState } from 'react';
import { api } from '@/lib/api/public-client';
import { getEnvConfig } from '@/lib/env';
import { useWalletAddress } from '@/lib/hooks/useWalletAddress';
import { canCancelMarket, parseMarketView, type MarketView } from '@/lib/markets/marketState';
import { Modal } from '@/components/Modal';
import './cancel.css';

/**
 * Market cancellation flow (#1376).
 *
 * `contracts/predict-iq/src/test_cancellation.rs` defines when cancellation
 * is contractually valid: only the creator/admin, only while `Active`, never
 * once bets are locked in. This page mirrors that state machine client-side
 * (`lib/markets/marketState.ts::canCancelMarket`) so the cancel action is
 * disabled/hidden instead of failing after a wallet has already signed.
 *
 * There is no `POST /api/v1/markets/{id}/cancel` route in
 * `services/api/openapi.yaml` yet (only `.../resolve` exists today), so the
 * submit call below is a plain `fetch` rather than a `lib/api/*-client.ts`
 * method — see the same note on `DepositTierBadge.tsx`. Swap it for the
 * typed client once that endpoint is generated into `schema.d.ts`.
 */

type SubmitState = 'idle' | 'submitting' | 'success' | 'error';

async function cancelMarket(marketId: string, signal: AbortSignal): Promise<void> {
  const base = getEnvConfig().NEXT_PUBLIC_API_URL.replace(/\/$/, '');
  const res = await fetch(`${base}/api/v1/markets/${encodeURIComponent(marketId)}/cancel`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    signal,
  });

  if (!res.ok) {
    let message = `Failed to cancel market (HTTP ${res.status})`;
    try {
      const body = (await res.json()) as { message?: string };
      if (body.message) message = body.message;
    } catch {
      // no JSON body — keep the generic message
    }
    throw new Error(message);
  }
}

export default function CancelMarketPage({ params }: { params: Promise<{ marketId: string }> }) {
  const { marketId } = use(params);
  const { address: walletAddress } = useWalletAddress();

  const [market, setMarket] = useState<MarketView | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
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

  const gate = canCancelMarket(market, walletAddress);

  const handleConfirm = async () => {
    setSubmitState('submitting');
    setSubmitError(null);
    try {
      await cancelMarket(marketId, new AbortController().signal);
      setSubmitState('success');
      setConfirmOpen(false);
    } catch (err) {
      setSubmitState('error');
      setSubmitError(err instanceof Error ? err.message : 'Failed to cancel market.');
    }
  };

  if (loadError) {
    return (
      <main className="cancel-market-page">
        <p role="alert" className="cancel-market-page__error">{loadError}</p>
      </main>
    );
  }

  if (submitState === 'success') {
    return (
      <main className="cancel-market-page">
        <p role="status">This market has been cancelled. Bettors can withdraw their refund.</p>
      </main>
    );
  }

  return (
    <main className="cancel-market-page">
      <h1 className="cancel-market-page__title">Cancel market</h1>

      {!market ? (
        <p role="status" aria-live="polite">Loading market…</p>
      ) : (
        <>
          {!gate.allowed && (
            <p role="alert" className="cancel-market-page__blocked">{gate.reason}</p>
          )}

          <button
            type="button"
            disabled={!gate.allowed}
            onClick={() => setConfirmOpen(true)}
            className="cancel-market-page__action"
          >
            Cancel this market
          </button>

          {submitError && (
            <p role="alert" className="cancel-market-page__error">{submitError}</p>
          )}
        </>
      )}

      <Modal open={confirmOpen} onClose={() => setConfirmOpen(false)} title="Cancel this market?">
        <p>
          This action is irreversible. Once cancelled, no more bets can be placed and existing
          bettors will be able to withdraw a full refund.
        </p>
        <div className="cancel-market-page__modal-actions">
          <button type="button" onClick={() => setConfirmOpen(false)} disabled={submitState === 'submitting'}>
            Keep market
          </button>
          <button
            type="button"
            className="cancel-market-page__confirm"
            onClick={handleConfirm}
            disabled={submitState === 'submitting'}
          >
            {submitState === 'submitting' ? 'Cancelling…' : 'Yes, cancel market'}
          </button>
        </div>
      </Modal>
    </main>
  );
}
