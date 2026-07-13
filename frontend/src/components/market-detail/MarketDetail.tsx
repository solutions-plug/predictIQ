'use client';

import React from 'react';
import Link from 'next/link';
import { Badge, Button, Card, EmptyState, StatusBadge, Stat } from '../ui';
import { getMarket } from '../../lib/mock/markets';
import { formatDate, formatXLM, outcomeOdds, timeUntil } from '../../lib/format';
import type { Market } from '../../lib/types';
import { OutcomeOdds } from './OutcomeOdds';
import { PlaceBetPanel } from './PlaceBetPanel';
import './marketDetail.css';

interface MarketDetailProps {
  id: string;
}

type LoadState =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | { status: 'ready'; market: Market | null };

export function MarketDetail({ id }: MarketDetailProps) {
  const [state, setState] = React.useState<LoadState>({ status: 'loading' });

  const load = React.useCallback(async () => {
    setState({ status: 'loading' });
    try {
      const market = await getMarket(id);
      setState({ status: 'ready', market });
    } catch (err) {
      setState({
        status: 'error',
        message: err instanceof Error ? err.message : 'Could not load this market.',
      });
    }
  }, [id]);

  React.useEffect(() => {
    void load();
  }, [load]);

  if (state.status === 'loading') {
    return (
      <>
        <div className="page-head">
          <h1>Loading market…</h1>
        </div>
        <Card aria-busy="true" aria-label="Loading market">
          <p style={{ color: 'var(--fg-muted)' }}>Fetching the latest odds and pools…</p>
        </Card>
      </>
    );
  }

  if (state.status === 'error') {
    return (
      <EmptyState
        title="Something went wrong"
        message={state.message}
        action={
          <Button variant="secondary" onClick={() => void load()}>
            Try again
          </Button>
        }
      />
    );
  }

  const { market } = state;

  if (!market) {
    return (
      <EmptyState
        title="Market not found"
        message="This market does not exist or may have been removed."
        action={
          <Link href="/markets" className="btn btn--secondary">
            Back to markets
          </Link>
        }
      />
    );
  }

  const isResolved = market.status === 'resolved';
  const winningOutcome =
    market.resolvedOutcome != null
      ? market.outcomes.find((o) => o.id === market.resolvedOutcome)
      : undefined;

  return (
    <>
      <div className="page-head">
        <p style={{ margin: 0 }}>
          <Link
            href="/markets"
            style={{ color: 'var(--fg-muted)', fontSize: 'var(--text-sm)' }}
          >
            ← All markets
          </Link>
        </p>
      </div>

      <div className="market-detail">
        <div className="market-detail__main">
          <header>
            <div className="market-detail__meta">
              <Badge tone="gold">{market.category}</Badge>
              <StatusBadge status={market.status} />
              <span style={{ color: 'var(--fg-subtle)', fontSize: 'var(--text-sm)' }}>
                {isResolved ? `Ended ${formatDate(market.endsAt)}` : timeUntil(market.endsAt)}
              </span>
            </div>
            <h1 className="market-detail__title">{market.title}</h1>
            <p className="market-detail__desc">{market.description}</p>
          </header>

          <div className="market-detail__stats">
            <Stat label="Total volume" value={formatXLM(market.totalVolume, { compact: true })} />
            <Stat label="Outcomes" value={market.outcomes.length} />
            <Stat label="Ends" value={formatDate(market.endsAt)} />
          </div>

          <section aria-label="Outcome odds">
            <h2 className="market-detail__section-label">Odds</h2>
            <div className="market-detail__outcomes">
              {market.outcomes.map((outcome) => (
                <OutcomeOdds
                  key={outcome.id}
                  label={outcome.label}
                  pool={market.poolByOutcome[outcome.id] ?? 0}
                  percent={outcomeOdds(market, outcome.id)}
                  won={isResolved && outcome.id === market.resolvedOutcome}
                />
              ))}
            </div>
          </section>

          {isResolved && (
            <div className="market-detail__resolved" role="status">
              <span className="market-detail__resolved-label">Resolved outcome</span>
              <span className="market-detail__resolved-outcome">
                {winningOutcome?.label ?? 'Settled'}
              </span>
            </div>
          )}
        </div>

        {!isResolved && <PlaceBetPanel market={market} onBetPlaced={() => void load()} />}
      </div>
    </>
  );
}
