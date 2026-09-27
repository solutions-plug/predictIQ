'use client';

import React from 'react';
import { api, ApiError } from '../../lib/api/public-client';
import { useWallet, WALLET_NOT_INSTALLED } from '../../lib/wallet/WalletProvider';
import { fetchNativeBalance } from '../../lib/wallet/balance';
import { INSUFFICIENT_BALANCE_MESSAGE, isInsufficientBalanceError } from '../../lib/wallet/errors';
import { useI18n } from '../../lib/hooks/useI18n';
import './PlaceBetForm.css';

export interface MarketOutcome {
  index: number;
  label: string;
}

export interface BetMarket {
  id: number | string;
  title: string;
  outcomes: MarketOutcome[];
}

interface PlaceBetFormProps {
  market: BetMarket;
  /** Called after the bet's transaction is submitted (still pending). */
  onSubmitted?: (result: { txHash: string; outcome: number; amount: string }) => void;
}

const TX_POLL_INTERVAL_MS = 2000;
const TX_POLL_MAX_ATTEMPTS = 10;

export const PlaceBetForm: React.FC<PlaceBetFormProps> = ({ market, onSubmitted }) => {
  const wallet = useWallet();
  const { t } = useI18n();
  const [selectedOutcome, setSelectedOutcome] = React.useState<number | null>(
    market.outcomes[0]?.index ?? null
  );
  const [amount, setAmount] = React.useState('');
  const [submitting, setSubmitting] = React.useState(false);
  const [formError, setFormError] = React.useState<string | null>(null);
  const [txHash, setTxHash] = React.useState<string | null>(null);
  const [txStatus, setTxStatus] = React.useState<string | null>(null);
  const [txPollingExhausted, setTxPollingExhausted] = React.useState(false);
  const [insufficientBalance, setInsufficientBalance] = React.useState<{
    required: string;
    current: string | null;
  } | null>(null);

  // Poll the submitted transaction until it leaves the "pending" state.
  React.useEffect(() => {
    if (!txHash || txStatus !== 'pending') return undefined;

    let cancelled = false;
    let attempts = 0;

    const poll = async () => {
      attempts += 1;
      try {
        const status = await api.getTransactionStatus(txHash);
        const nextStatus = (status as { status?: string })?.status;
        if (!cancelled && nextStatus) setTxStatus(nextStatus);
      } catch {
        // Transient — the interval below retries.
      }
      if (!cancelled && attempts < TX_POLL_MAX_ATTEMPTS) {
        timer = setTimeout(poll, TX_POLL_INTERVAL_MS);
      } else if (!cancelled && attempts >= TX_POLL_MAX_ATTEMPTS) {
        // Polling exhausted; mark so UI can show retry option.
        setTxPollingExhausted(true);
      }
    };

    let timer = setTimeout(poll, TX_POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [txHash, txStatus]);

  const validate = (): string | null => {
    if (selectedOutcome === null) return t('placeBetForm.validationOutcome');
    const parsed = Number(amount);
    if (!amount || Number.isNaN(parsed) || parsed <= 0) {
      return t('placeBetForm.validationAmount');
    }
    return null;
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setFormError(null);

    // No connected wallet yet — trigger the connect flow inline instead of
    // silently failing or leaving the button disabled with no explanation.
    if (!wallet.address) {
      await wallet.connect();
      return;
    }

    const validationError = validate();
    if (validationError) {
      setFormError(validationError);
      return;
    }

    setSubmitting(true);
    setTxHash(null);
    setTxStatus(null);
    setTxPollingExhausted(false);
    setInsufficientBalance(null);

    try {
      const result = await api.placeBet(market.id, {
        wallet: wallet.address,
        outcome: selectedOutcome as number,
        amount,
      });
      setTxHash(result.tx_hash);
      setTxStatus(result.status);
      onSubmitted?.({ txHash: result.tx_hash, outcome: selectedOutcome as number, amount });
    } catch (err) {
      if (err instanceof ApiError && isInsufficientBalanceError(err.details)) {
        // Read the balance fresh at the moment of failure — never the value
        // cached from page load — since the wallet's actual asset state is
        // what the user needs to see to understand the error (see #79).
        const currentBalance = await fetchNativeBalance(wallet.address, wallet.network);
        setInsufficientBalance({ required: amount, current: currentBalance });
      } else {
        const message =
          err instanceof ApiError ? err.message : 'Failed to submit your bet. Please try again.';
        setFormError(message);
      }
    } finally {
      setSubmitting(false);
    }
  };

  const isNotInstalled = !wallet.address && wallet.error === WALLET_NOT_INSTALLED;

  const handleRetryStatus = async () => {
    if (!txHash) return;
    setTxPollingExhausted(false);
    setTxStatus('pending');
  };

  return (
    <form className="place-bet-form" aria-labelledby="place-bet-heading" onSubmit={handleSubmit}>
      <h3 id="place-bet-heading">{t('placeBetForm.title')}</h3>

      {isNotInstalled && (
        <p className="place-bet-form__install-prompt" role="alert">
          {t('placeBetForm.walletNotInstalled')}{' '}
          <a href="https://www.freighter.app/" target="_blank" rel="noreferrer">
            {t('placeBetForm.installFreighter')}
          </a>{' '}
          to place a bet.
        </p>
      )}

      {!wallet.address && !isNotInstalled && (
        <p className="place-bet-form__hint">{t('placeBetForm.connectWalletHint')}</p>
      )}

      {wallet.address && (
        <p className="place-bet-form__connected">
          {t('placeBetForm.connected').replace('{address}', `${wallet.address.slice(0, 4)}…${wallet.address.slice(-4)}`)}
        </p>
      )}

      <fieldset disabled={submitting}>
        <legend>{t('placeBetForm.heading')}</legend>
        <div className="place-bet-form__outcomes" role="radiogroup" aria-label="Outcome">
          {market.outcomes.map((outcome) => (
            <label key={outcome.index} className="place-bet-form__outcome">
              <input
                type="radio"
                name="outcome"
                value={outcome.index}
                checked={selectedOutcome === outcome.index}
                onChange={() => setSelectedOutcome(outcome.index)}
              />
              {outcome.label}
            </label>
          ))}
        </div>

        <label htmlFor="bet-amount" className="place-bet-form__amount-label">
          {t('placeBetForm.amountLabel')}
        </label>
        <input
          id="bet-amount"
          name="amount"
          type="number"
          min="0"
          step="0.0000001"
          inputMode="decimal"
          value={amount}
          onChange={(event) => setAmount(event.target.value)}
          placeholder="0.00"
        />
      </fieldset>

      {formError && (
        <p className="place-bet-form__error" role="alert">
          {formError}
        </p>
      )}

      {insufficientBalance && (
        <p className="place-bet-form__error" role="alert">
          {INSUFFICIENT_BALANCE_MESSAGE} Your balance is{' '}
          {insufficientBalance.current ?? 'unavailable'} XLM, but this bet requires{' '}
          {insufficientBalance.required} XLM.
        </p>
      )}

      {wallet.error && wallet.error !== WALLET_NOT_INSTALLED && (
        <p className="place-bet-form__error" role="alert">
          {wallet.error}
        </p>
      )}

      {txHash && (
        <>
          <p className="place-bet-form__status" role="status">
            {txPollingExhausted && txStatus === 'pending'
              ? t('placeBetForm.betStillPending')
              : t('placeBetForm.betSubmitted').replace('{status}', txStatus ?? 'pending')}
            {' '}{t('placeBetForm.transactionLabel').replace('{hash}', txHash.slice(0, 10))}…
          </p>
          {txPollingExhausted && txStatus === 'pending' && (
            <button
              type="button"
              onClick={handleRetryStatus}
              className="place-bet-form__retry-btn"
              disabled={submitting}
            >
              {t('placeBetForm.checkStatusAgain')}
            </button>
          )}
        </>
      )}

      <button type="submit" disabled={submitting || wallet.isConnecting}>
        {!wallet.address
          ? wallet.isConnecting
            ? t('placeBetForm.connecting')
            : t('placeBetForm.connectWallet')
          : submitting
            ? t('placeBetForm.placingBet')
            : t('placeBetForm.placeBet')}
      </button>
    </form>
  );
};
