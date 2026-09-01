'use client';

import React from 'react';
import { useRouter } from 'next/navigation';
import { getEnvConfig } from '@/lib/env';
import { getConnectedWalletAddress } from '@/lib/wallet';
import { getSupportedAsset } from '@/lib/assets';
import { TokenSelector } from '@/components/markets/TokenSelector';
import {
  marketFormSchema,
  validateMarketForm,
  type MarketFormErrors,
  type MarketFormValues,
} from '@/lib/validation/marketForm';
import './page.css';

const EMPTY_FORM: MarketFormValues = {
  title: '',
  description: '',
  outcomes: ['', ''],
  closeTime: '',
  asset: '',
};

interface CreateMarketResponse {
  market_id: number;
  tx_hash: string;
  status: string;
}

export default function CreateMarketPage() {
  const router = useRouter();
  const [values, setValues] = React.useState<MarketFormValues>(EMPTY_FORM);
  const [errors, setErrors] = React.useState<MarketFormErrors>({});
  const [submitting, setSubmitting] = React.useState(false);
  const [success, setSuccess] = React.useState(false);

  const setField = <K extends keyof MarketFormValues>(field: K, value: MarketFormValues[K]) => {
    setValues((prev) => ({ ...prev, [field]: value }));
  };

  const setOutcome = (index: number, value: string) => {
    setValues((prev) => ({
      ...prev,
      outcomes: prev.outcomes.map((o, i) => (i === index ? value : o)),
    }));
  };

  const addOutcome = () => {
    setValues((prev) => ({ ...prev, outcomes: [...prev.outcomes, ''] }));
  };

  const removeOutcome = (index: number) => {
    setValues((prev) => ({
      ...prev,
      outcomes: prev.outcomes.filter((_, i) => i !== index),
    }));
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSuccess(false);

    const fieldErrors = validateMarketForm(values);
    if (Object.keys(fieldErrors).length > 0) {
      setErrors(fieldErrors);
      return;
    }

    const wallet = getConnectedWalletAddress();
    if (!wallet) {
      setErrors({ form: 'Connect a wallet before creating a market.' });
      return;
    }

    setErrors({});
    setSubmitting(true);
    try {
      const parsed = marketFormSchema.parse(values);
      const config = getEnvConfig();
      const res = await fetch(`${config.NEXT_PUBLIC_API_URL.replace(/\/$/, '')}/api/v1/blockchain/markets`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          creator: wallet,
          title: parsed.title,
          description: parsed.description,
          outcomes: parsed.outcomes,
          ends_at: new Date(parsed.closeTime).toISOString(),
          settlement_asset: getSupportedAsset(parsed.asset)?.address,
        }),
      });

      if (!res.ok) {
        throw new Error(`Failed to create market (${res.status})`);
      }

      const data: CreateMarketResponse = await res.json();
      setSuccess(true);
      router.push(`/markets/${data.market_id}`);
    } catch (err) {
      setErrors({ form: err instanceof Error ? err.message : 'Failed to create market.' });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <main className="market-create">
      <h1 className="market-create__heading">Create a market</h1>
      <p className="market-create__intro">
        Define a question, its possible outcomes, and when betting closes.
      </p>

      {/* Placeholder success banner — replaced by the shared toast component from #17. */}
      {success && (
        <div className="market-create__success" role="status">
          Market created. Redirecting…
        </div>
      )}
      {errors.form && (
        <div className="market-create__form-error" role="alert">
          {errors.form}
        </div>
      )}

      <form onSubmit={handleSubmit} noValidate>
        <div className="market-create__field">
          <label htmlFor="market-title">Title</label>
          <input
            id="market-title"
            name="title"
            type="text"
            value={values.title}
            onChange={(e) => setField('title', e.target.value)}
            aria-invalid={Boolean(errors.title)}
            aria-describedby={errors.title ? 'market-title-error' : undefined}
          />
          {errors.title && (
            <span id="market-title-error" className="market-create__error" role="alert">
              {errors.title}
            </span>
          )}
        </div>

        <div className="market-create__field">
          <label htmlFor="market-description">Description</label>
          <textarea
            id="market-description"
            name="description"
            rows={4}
            value={values.description}
            onChange={(e) => setField('description', e.target.value)}
            aria-invalid={Boolean(errors.description)}
            aria-describedby={errors.description ? 'market-description-error' : undefined}
          />
          {errors.description && (
            <span id="market-description-error" className="market-create__error" role="alert">
              {errors.description}
            </span>
          )}
        </div>

        <div className="market-create__field">
          <label htmlFor="market-outcome-0">Outcomes</label>
          <div className="market-create__outcomes">
            {values.outcomes.map((outcome, index) => (
              <div className="market-create__outcome-row" key={index}>
                <input
                  id={`market-outcome-${index}`}
                  type="text"
                  value={outcome}
                  placeholder={`Outcome ${index + 1}`}
                  onChange={(e) => setOutcome(index, e.target.value)}
                  aria-invalid={Boolean(errors.outcomes)}
                />
                {values.outcomes.length > 1 && (
                  <button
                    type="button"
                    className="market-create__outcome-remove"
                    onClick={() => removeOutcome(index)}
                    aria-label={`Remove outcome ${index + 1}`}
                  >
                    &times;
                  </button>
                )}
              </div>
            ))}
          </div>
          <button type="button" className="market-create__add-outcome" onClick={addOutcome}>
            + Add outcome
          </button>
          {errors.outcomes && (
            <span className="market-create__error" role="alert">
              {errors.outcomes}
            </span>
          )}
        </div>

        <div className="market-create__field">
          <label htmlFor="market-close-time">Close time</label>
          <input
            id="market-close-time"
            name="closeTime"
            type="datetime-local"
            value={values.closeTime}
            onChange={(e) => setField('closeTime', e.target.value)}
            aria-invalid={Boolean(errors.closeTime)}
            aria-describedby={errors.closeTime ? 'market-close-time-error' : undefined}
          />
          {errors.closeTime && (
            <span id="market-close-time-error" className="market-create__error" role="alert">
              {errors.closeTime}
            </span>
          )}
        </div>

        <div className="market-create__field">
          <label htmlFor="market-asset">Settlement asset</label>
          <TokenSelector
            id="market-asset"
            value={values.asset}
            onChange={(assetId) => setField('asset', assetId)}
            aria-invalid={Boolean(errors.asset)}
            aria-describedby={errors.asset ? 'market-asset-error' : undefined}
          />
          {errors.asset && (
            <span id="market-asset-error" className="market-create__error" role="alert">
              {errors.asset}
            </span>
          )}
        </div>

        <button type="submit" className="market-create__submit" disabled={submitting}>
          {submitting ? 'Creating…' : 'Create market'}
        </button>
      </form>
    </main>
  );
}
