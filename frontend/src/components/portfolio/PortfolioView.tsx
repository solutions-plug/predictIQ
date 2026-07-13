'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import type { Bet, Market } from '../../lib/types';
import { getUserBets, getMarket } from '../../lib/mock/markets';
import { useWallet } from '../../lib/wallet/WalletProvider';
import { formatXLM } from '../../lib/format';
import { Button, Card, Stat, EmptyState } from '../ui';
import { BetRow } from './BetRow';
import './portfolio.css';

type MarketMap = Record<string, Market | null>;

export function PortfolioView() {
  const { isConnected, address, connect, isConnecting } = useWallet();

  const [bets, setBets] = useState<Bet[]>([]);
  const [markets, setMarkets] = useState<MarketMap>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isConnected || !address) {
      setLoading(false);
      return;
    }

    let active = true;
    setLoading(true);
    setError(null);

    (async () => {
      try {
        const userBets = await getUserBets(address);
        if (!active) return;
        setBets(userBets);

        const uniqueIds = Array.from(new Set(userBets.map((b) => b.marketId)));
        const fetched = await Promise.all(uniqueIds.map((id) => getMarket(id)));
        if (!active) return;

        const map: MarketMap = {};
        uniqueIds.forEach((id, i) => {
          map[id] = fetched[i];
        });
        setMarkets(map);
      } catch {
        if (active) setError('We could not load your positions. Please try again.');
      } finally {
        if (active) setLoading(false);
      }
    })();

    return () => {
      active = false;
    };
  }, [isConnected, address]);

  const handleClaimed = useCallback((updated: Bet) => {
    setBets((prev) => prev.map((b) => (b.id === updated.id ? updated : b)));
  }, []);

  const summary = useMemo(() => {
    let totalStaked = 0;
    let activePositions = 0;
    let claimable = 0;

    for (const bet of bets) {
      totalStaked += bet.amount;
      const market = markets[bet.marketId];
      if (market?.status === 'open') {
        activePositions += 1;
      }
      const isWinner =
        market?.status === 'resolved' && market.resolvedOutcome === bet.outcomeId;
      if (isWinner && !bet.claimed) {
        claimable += bet.payout ?? 0;
      }
    }

    return { totalStaked, activePositions, claimable };
  }, [bets, markets]);

  if (!isConnected) {
    return (
      <>
        <div className="page-head">
          <div>
            <h1>Portfolio</h1>
            <p>Your positions, settled bets, and claimable winnings.</p>
          </div>
        </div>
        <Card>
          <EmptyState
            title="Connect your wallet to view your positions"
            message="Your bets and claimable winnings live with your Stellar wallet."
            action={
              <Button onClick={connect} loading={isConnecting} disabled={isConnecting}>
                Connect wallet
              </Button>
            }
          />
        </Card>
      </>
    );
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Portfolio</h1>
          <p>Track your open positions, settled bets, and claimable winnings.</p>
        </div>
      </div>

      {error ? (
        <EmptyState title="Something went wrong" message={error} />
      ) : loading ? (
        <div aria-busy="true" aria-live="polite" className="empty-state">
          Loading your positions…
        </div>
      ) : bets.length === 0 ? (
        <EmptyState
          title="No positions yet"
          message="You haven't placed any bets. Explore the markets to get started."
          action={
            <Link href="/markets">
              <Button>Explore markets</Button>
            </Link>
          }
        />
      ) : (
        <>
          <div className="portfolio-summary">
            <Card>
              <Stat label="Total staked" value={formatXLM(summary.totalStaked, { compact: true })} />
            </Card>
            <Card>
              <Stat label="Active positions" value={summary.activePositions} />
            </Card>
            <Card>
              <Stat
                label="Claimable"
                value={
                  <span className="stat-value--gold">
                    {formatXLM(summary.claimable, { compact: true })}
                  </span>
                }
              />
            </Card>
          </div>

          <div className="portfolio-table-wrap">
            <table className="portfolio-table">
              <caption className="visually-hidden">Your bet positions</caption>
              <thead>
                <tr>
                  <th scope="col">Market</th>
                  <th scope="col">Stake</th>
                  <th scope="col">Placed</th>
                  <th scope="col">Status</th>
                  <th scope="col" className="bet-action">
                    Action
                  </th>
                </tr>
              </thead>
              <tbody>
                {bets.map((bet) => (
                  <BetRow
                    key={bet.id}
                    bet={bet}
                    market={markets[bet.marketId] ?? null}
                    onClaimed={handleClaimed}
                  />
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </>
  );
}
