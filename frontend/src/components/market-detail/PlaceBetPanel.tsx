'use client';

import React from 'react';
import { Button, Card, Field, Input, Modal } from '../ui';
import { useWallet } from '../../lib/wallet/WalletProvider';
import { placeBet } from '../../lib/mock/markets';
import { formatXLM, outcomeOdds, shortAddress } from '../../lib/format';
import type { Bet, Market } from '../../lib/types';

interface PlaceBetPanelProps {
  market: Market;
  /** Called after a bet is successfully placed so the parent can refresh. */
  onBetPlaced: () => void;
}

/**
 * Estimate the pro-rata payout for staking `amount` on `outcomeId`, using the
 * same settlement maths as the mock resolver (winners split the whole pool).
 */
function estimatePayout(market: Market, outcomeId: number, amount: number): number {
  if (amount <= 0) return 0;
  const outcomePool = (market.poolByOutcome[outcomeId] ?? 0) + amount;
  const totalPool = market.totalVolume + amount;
  return (amount / outcomePool) * totalPool;
}

export function PlaceBetPanel({ market, onBetPlaced }: PlaceBetPanelProps) {
  const { isConnected, address, connect, authorize, isConnecting } = useWallet();

  const [outcomeId, setOutcomeId] = React.useState<number>(market.outcomes[0]?.id ?? 0);
  const [amountInput, setAmountInput] = React.useState('');
  const [submitting, setSubmitting] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [confirmed, setConfirmed] = React.useState<Bet | null>(null);

  const amount = Number.parseFloat(amountInput);
  const hasValidAmount = Number.isFinite(amount) && amount > 0;
  const selectedLabel = market.outcomes.find((o) => o.id === outcomeId)?.label ?? '';
  const potentialReturn = hasValidAmount ? estimatePayout(market, outcomeId, amount) : 0;

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    setError(null);

    if (!isConnected) {
      await connect();
      return;
    }
    if (!hasValidAmount) {
      setError('Enter a stake greater than zero.');
      return;
    }

    setSubmitting(true);
    try {
      const signature = await authorize(`Bet ${amount} XLM on "${selectedLabel}"`);
      if (!signature) {
        setError('Signature was rejected. Your bet was not placed.');
        return;
      }
      const bet = await placeBet({
        marketId: market.id,
        outcomeId,
        amount,
        user: address ?? '',
        txHash: signature.slice(0, 24),
      });
      setConfirmed(bet);
      setAmountInput('');
      onBetPlaced();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not place your bet. Try again.');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Card as="section" className="bet-panel" aria-label="Place a bet">
      <h2 className="bet-panel__title">Place your bet</h2>

      <form onSubmit={handleSubmit} noValidate>
        <fieldset
          className="bet-panel__outcomes"
          style={{ border: 'none', padding: 0, margin: '0 0 1.1rem' }}
        >
          <legend className="bet-panel__hint" style={{ marginBottom: '0.5rem' }}>
            Pick an outcome
          </legend>
          {market.outcomes.map((outcome) => {
            const active = outcome.id === outcomeId;
            return (
              <button
                key={outcome.id}
                type="button"
                className={`bet-option ${active ? 'bet-option--active' : ''}`.trim()}
                aria-pressed={active}
                onClick={() => setOutcomeId(outcome.id)}
              >
                <span className="bet-option__label">{outcome.label}</span>
                <span className="bet-option__odds">{outcomeOdds(market, outcome.id)}%</span>
              </button>
            );
          })}
        </fieldset>

        <Field label="Stake (XLM)" htmlFor="bet-amount" hint="How much XLM to stake on this outcome.">
          <Input
            id="bet-amount"
            name="amount"
            type="number"
            inputMode="decimal"
            min="0"
            step="any"
            placeholder="0.00"
            value={amountInput}
            onChange={(e) => setAmountInput(e.target.value)}
          />
        </Field>

        {hasValidAmount && (
          <p className="bet-panel__hint">
            If <strong>{selectedLabel}</strong> wins, you could receive about{' '}
            <strong>{formatXLM(potentialReturn, { compact: true })}</strong>.
          </p>
        )}

        {error && (
          <p className="bet-panel__error" role="alert">
            {error}
          </p>
        )}

        <Button
          type="submit"
          block
          size="lg"
          loading={submitting || isConnecting}
          disabled={submitting}
        >
          {isConnected ? 'Place Bet' : 'Connect Wallet to Bet'}
        </Button>
      </form>

      <Modal
        open={confirmed !== null}
        onClose={() => setConfirmed(null)}
        title="Bet placed"
      >
        {confirmed && (
          <div className="bet-success">
            <p className="bet-panel__hint">
              Your stake of <strong>{formatXLM(confirmed.amount)}</strong> on{' '}
              <strong>{market.outcomes.find((o) => o.id === confirmed.outcomeId)?.label}</strong> is
              in. If it wins you could receive about{' '}
              <strong>
                {formatXLM(estimatePayout(market, confirmed.outcomeId, confirmed.amount), {
                  compact: true,
                })}
              </strong>
              .
            </p>
            <div className="bet-success__row">
              <span>Bettor</span>
              <span>{shortAddress(confirmed.user)}</span>
            </div>
            <div className="bet-success__row">
              <span>Tx hash</span>
              <span className="bet-success__tx">{confirmed.txHash}</span>
            </div>
            <Button variant="secondary" block onClick={() => setConfirmed(null)}>
              Done
            </Button>
          </div>
        )}
      </Modal>
    </Card>
  );
}
