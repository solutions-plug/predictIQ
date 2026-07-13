'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import type { Market } from '../../lib/types';
import { getMarket, resolveMarket } from '../../lib/mock/markets';
import { useWallet } from '../../lib/wallet/WalletProvider';
import { Button, Card, EmptyState, StatusBadge } from '../ui';

interface ResolvePanelProps {
  id: string;
}

export function ResolvePanel({ id }: ResolvePanelProps) {
  const { isConnected, connect, isConnecting, authorize } = useWallet();

  const [market, setMarket] = useState<Market | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [selected, setSelected] = useState<number | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [settled, setSettled] = useState<Market | null>(null);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);

    getMarket(id)
      .then((data) => {
        if (!active) return;
        setMarket(data);
        if (data?.status === 'resolved') setSettled(data);
      })
      .catch(() => {
        if (active) setError('We could not load this market. Please try again.');
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [id]);

  async function handleResolve() {
    if (!market || selected === null) return;
    setSubmitError(null);
    setSubmitting(true);
    try {
      await authorize(`Resolve ${market.title}`);
      const updated = await resolveMarket(market.id, selected);
      setSettled(updated);
      setMarket(updated);
    } catch (e) {
      setSubmitError(e instanceof Error ? e.message : 'Could not resolve the market.');
    } finally {
      setSubmitting(false);
    }
  }

  const resolvedMarket = settled ?? (market?.status === 'resolved' ? market : null);

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Resolve market</h1>
          <p>Oracle settlement (demo).</p>
        </div>
      </div>

      {error ? (
        <EmptyState title="Something went wrong" message={error} />
      ) : loading ? (
        <div aria-busy="true" aria-live="polite" className="empty-state">
          Loading market…
        </div>
      ) : !market ? (
        <EmptyState
          title="Market not found"
          message="This market does not exist or may have been removed."
          action={
            <Link href="/markets">
              <Button>Back to markets</Button>
            </Link>
          }
        />
      ) : (
        <Card as="section" style={{ maxWidth: '640px', padding: '2rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', flexWrap: 'wrap' }}>
            <h2 style={{ margin: 0 }}>{market.title}</h2>
            <StatusBadge status={market.status} />
          </div>

          {resolvedMarket ? (
            <div style={{ marginTop: '1.5rem' }}>
              <p role="status">
                This market is settled. Winning outcome:{' '}
                <strong>
                  {resolvedMarket.outcomes.find((o) => o.id === resolvedMarket.resolvedOutcome)?.label ??
                    `Outcome ${resolvedMarket.resolvedOutcome}`}
                </strong>
                .
              </p>
              <div style={{ marginTop: '1.25rem' }}>
                <Link href="/portfolio">
                  <Button>View your portfolio</Button>
                </Link>
              </div>
            </div>
          ) : (
            <>
              <p style={{ color: 'var(--fg-muted)', marginTop: '1rem' }}>
                Oracle resolution (demo): select the winning outcome to settle this market.
              </p>

              <fieldset
                style={{ border: 'none', padding: 0, margin: '1.25rem 0 0' }}
                disabled={submitting}
              >
                <legend className="visually-hidden">Winning outcome</legend>
                {market.outcomes.map((outcome) => (
                  <label
                    key={outcome.id}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '0.75rem',
                      padding: '0.75rem 1rem',
                      border: '1px solid var(--border)',
                      borderRadius: 'var(--radius)',
                      marginBottom: '0.6rem',
                      cursor: 'pointer',
                    }}
                  >
                    <input
                      type="radio"
                      name="winning-outcome"
                      value={outcome.id}
                      checked={selected === outcome.id}
                      onChange={() => setSelected(outcome.id)}
                    />
                    <span>{outcome.label}</span>
                  </label>
                ))}
              </fieldset>

              {!isConnected ? (
                <div style={{ marginTop: '1.25rem' }}>
                  <p style={{ color: 'var(--fg-muted)', marginBottom: '0.75rem' }}>
                    Connect your wallet to settle this market.
                  </p>
                  <Button onClick={connect} loading={isConnecting} disabled={isConnecting}>
                    Connect wallet
                  </Button>
                </div>
              ) : (
                <div style={{ marginTop: '1.25rem' }}>
                  <Button
                    onClick={handleResolve}
                    loading={submitting}
                    disabled={submitting || selected === null}
                  >
                    Resolve market
                  </Button>
                  {submitError && (
                    <p role="alert" style={{ color: 'var(--destructive, var(--fg-muted))', marginTop: '0.75rem' }}>
                      {submitError}
                    </p>
                  )}
                </div>
              )}
            </>
          )}
        </Card>
      )}
    </>
  );
}
