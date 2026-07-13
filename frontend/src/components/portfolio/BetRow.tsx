'use client';

import { useState } from 'react';
import Link from 'next/link';
import type { Bet, Market } from '../../lib/types';
import { claimWinnings } from '../../lib/mock/markets';
import { useWallet } from '../../lib/wallet/WalletProvider';
import { formatXLM, formatDate } from '../../lib/format';
import { Button, StatusBadge } from '../ui';

interface BetRowProps {
  bet: Bet;
  market: Market | null;
  onClaimed?: (updated: Bet) => void;
}

export function BetRow({ bet, market, onClaimed }: BetRowProps) {
  const { authorize } = useWallet();
  const [current, setCurrent] = useState<Bet>(bet);
  const [claiming, setClaiming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const outcome = market?.outcomes.find((o) => o.id === current.outcomeId);
  const outcomeLabel = outcome?.label ?? `Outcome ${current.outcomeId}`;
  const isResolved = market?.status === 'resolved';
  const isWinner = isResolved && market?.resolvedOutcome === current.outcomeId;
  const isClaimable = Boolean(isWinner) && !current.claimed;

  async function handleClaim() {
    setError(null);
    setClaiming(true);
    try {
      await authorize(`Claim winnings for bet ${current.id}`);
      const updated = await claimWinnings(current.id);
      setCurrent(updated);
      onClaimed?.(updated);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not claim winnings.');
    } finally {
      setClaiming(false);
    }
  }

  function renderAction() {
    if (!market) return <span className="bet-outcome-lost">—</span>;
    if (market.status === 'open') {
      return <span>Active</span>;
    }
    if (isResolved && !isWinner) {
      return <span className="bet-outcome-lost">Lost</span>;
    }
    if (current.claimed) {
      return (
        <span className="bet-payout">
          Claimed
          {current.payout != null ? ` · ${formatXLM(current.payout)}` : ''}
        </span>
      );
    }
    if (isClaimable) {
      return (
        <div>
          <Button size="sm" onClick={handleClaim} loading={claiming} disabled={claiming}>
            Claim
          </Button>
          {error && (
            <span role="alert" className="bet-outcome-lost" style={{ display: 'block', marginTop: '0.35rem' }}>
              {error}
            </span>
          )}
        </div>
      );
    }
    return <span>—</span>;
  }

  return (
    <tr>
      <td>
        <Link href={`/markets/${current.marketId}`} className="bet-market-link">
          {market ? market.title : current.marketId}
        </Link>
        <span className="bet-outcome">Backed: {outcomeLabel}</span>
      </td>
      <td className="bet-amount">{formatXLM(current.amount)}</td>
      <td>{formatDate(current.placedAt)}</td>
      <td>{market ? <StatusBadge status={market.status} /> : null}</td>
      <td className="bet-action">{renderAction()}</td>
    </tr>
  );
}
