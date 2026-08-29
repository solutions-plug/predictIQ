'use client';

import { useCallback, useState } from 'react';
import { useAsync } from '@/lib/hooks/useAsync';
import { useTransaction } from '@/lib/hooks/useTransaction';
import { describeWriteError } from '@/lib/api/contractErrors';
import { ApiError } from '@/lib/api/public-client';
import { getEnvConfig } from '@/lib/env';

interface DisputedOutcome {
  outcome_index: number;
  label: string;
  vote_count: number;
}

interface DisputeStatusResponse {
  market_id: string;
  is_disputed: boolean;
  /** May contain more than one entry — a dispute can produce multiple winning outcomes. */
  disputed_outcomes: DisputedOutcome[];
}

const config = getEnvConfig();
const BASE_URL = config.NEXT_PUBLIC_API_URL.replace(/\/$/, '');

async function fetchDisputeStatus(marketId: string, signal: AbortSignal): Promise<DisputeStatusResponse> {
  const res = await fetch(`${BASE_URL}/api/v1/blockchain/markets/${encodeURIComponent(marketId)}/disputes`, {
    signal,
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new ApiError(body.message ?? `Failed to load dispute status (${res.status})`, res.status, body.code);
  }
  return res.json();
}

interface DisputePageProps {
  params: { marketId: string };
}

/**
 * Lets an eligible party raise a dispute on a market and shows every
 * currently disputed winner outcome — a dispute can resolve to more than
 * one winner (see contracts/predict-iq/src/test_disputes_winner_count.rs),
 * so this must render the full list, not just the first result.
 */
export default function DisputePage({ params }: DisputePageProps) {
  const { marketId } = params;
  const [reason, setReason] = useState('');
  const tx = useTransaction<{ marketId: string; reason: string }>();

  const load = useCallback((signal: AbortSignal) => fetchDisputeStatus(marketId, signal), [marketId]);
  const { data, loading, error: loadError, execute: reload } = useAsync(load, { immediate: true });

  const filingError = tx.status === 'failed' && tx.error ? describeWriteError(tx.error, 'resolve') : null;

  async function handleFileDispute() {
    await tx.run({
      sign: async () => ({ marketId, reason }),
      submit: async (signed) => {
        const res = await fetch(`${BASE_URL}/api/v1/blockchain/markets/${encodeURIComponent(signed.marketId)}/disputes`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ reason: signed.reason }),
        });
        if (!res.ok) {
          const body = await res.json().catch(() => ({}));
          throw new ApiError(body.message ?? `Failed to file dispute (${res.status})`, res.status, body.code, body.details);
        }
        const body = await res.json();
        return body.tx_hash as string;
      },
    });
    if (tx.status !== 'failed') {
      setReason('');
      void reload();
    }
  }

  const disputedOutcomes = data?.disputed_outcomes ?? [];

  return (
    <main className="dispute-page">
      <h1>Dispute resolution</h1>

      {loading && <p>Loading dispute status…</p>}
      {loadError && <p role="alert">Unable to load dispute status: {loadError.message}</p>}

      {data && (
        <section aria-labelledby="disputed-outcomes-heading">
          <h2 id="disputed-outcomes-heading">Disputed winner outcomes</h2>
          {disputedOutcomes.length === 0 ? (
            <p>No dispute has been filed for this market yet.</p>
          ) : (
            <ul className="dispute-page__outcomes">
              {disputedOutcomes.map((o) => (
                <li key={o.outcome_index}>
                  <span className="dispute-page__outcome-label">{o.label}</span>
                  <span className="dispute-page__outcome-votes">{o.vote_count} votes</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      <section aria-labelledby="file-dispute-heading">
        <h2 id="file-dispute-heading">Raise a dispute</h2>
        <label htmlFor="dispute-reason">Reason</label>
        <textarea
          id="dispute-reason"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          disabled={tx.status === 'pending' || tx.status === 'signed' || tx.status === 'submitted'}
          required
        />
        <button
          type="button"
          onClick={handleFileDispute}
          disabled={!reason || tx.status === 'pending' || tx.status === 'signed' || tx.status === 'submitted'}
        >
          {tx.status === 'pending'
            ? 'Awaiting signature…'
            : tx.status === 'signed' || tx.status === 'submitted'
            ? 'Submitting dispute…'
            : 'File dispute'}
        </button>

        {tx.status === 'confirmed' && <p role="status">Dispute filed successfully.</p>}
        {filingError && <p role="alert">{filingError}</p>}
      </section>
    </main>
  );
}
