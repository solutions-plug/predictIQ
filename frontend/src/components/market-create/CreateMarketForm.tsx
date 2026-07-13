'use client';

import React from 'react';
import { useRouter } from 'next/navigation';
import { Button, Card, Field, Input, Textarea, Select, EmptyState } from '../ui';
import { useWallet } from '../../lib/wallet/WalletProvider';
import { createMarket } from '../../lib/mock/markets';
import { MARKET_CATEGORIES, type CreateMarketInput } from '../../lib/types';
import './marketCreate.css';

const TITLE_MIN = 8;
const TITLE_MAX = 120;
const DESC_MIN = 20;
const DESC_MAX = 500;
const OUTCOMES_MIN = 2;

interface OutcomeRow {
  id: string;
  value: string;
}

interface FieldErrors {
  title?: string;
  description?: string;
  category?: string;
  outcomes?: string;
  endsAt?: string;
}

let rowSeq = 0;
function makeRow(value: string): OutcomeRow {
  rowSeq += 1;
  return { id: `outcome-${rowSeq}`, value };
}

/** Today's date as a YYYY-MM-DD string for the date input's `min`. */
function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

function validate(
  title: string,
  description: string,
  category: string,
  outcomes: OutcomeRow[],
  endsAt: string,
): FieldErrors {
  const errors: FieldErrors = {};

  const trimmedTitle = title.trim();
  if (trimmedTitle.length < TITLE_MIN || trimmedTitle.length > TITLE_MAX) {
    errors.title = `Title must be between ${TITLE_MIN} and ${TITLE_MAX} characters.`;
  }

  const trimmedDesc = description.trim();
  if (trimmedDesc.length < DESC_MIN || trimmedDesc.length > DESC_MAX) {
    errors.description = `Description must be between ${DESC_MIN} and ${DESC_MAX} characters.`;
  }

  if (!category) {
    errors.category = 'Pick a category.';
  }

  const values = outcomes.map((o) => o.value.trim());
  if (values.some((v) => v.length === 0)) {
    errors.outcomes = 'Every outcome needs a label.';
  } else if (values.length < OUTCOMES_MIN) {
    errors.outcomes = `Add at least ${OUTCOMES_MIN} outcomes.`;
  } else {
    const lower = values.map((v) => v.toLowerCase());
    if (new Set(lower).size !== lower.length) {
      errors.outcomes = 'Outcomes must be unique.';
    }
  }

  if (!endsAt) {
    errors.endsAt = 'Pick an end date.';
  } else {
    const end = new Date(`${endsAt}T23:59:59`);
    if (Number.isNaN(end.getTime()) || end.getTime() <= Date.now()) {
      errors.endsAt = 'End date must be in the future.';
    }
  }

  return errors;
}

export function CreateMarketForm() {
  const router = useRouter();
  const { address, isConnected, isConnecting, connect, authorize } = useWallet();

  const [title, setTitle] = React.useState('');
  const [description, setDescription] = React.useState('');
  const [category, setCategory] = React.useState<string>(MARKET_CATEGORIES[0]);
  const [outcomes, setOutcomes] = React.useState<OutcomeRow[]>(() => [
    makeRow('Yes'),
    makeRow('No'),
  ]);
  const [endsAt, setEndsAt] = React.useState('');

  const [errors, setErrors] = React.useState<FieldErrors>({});
  const [formError, setFormError] = React.useState<string | null>(null);
  const [submitting, setSubmitting] = React.useState(false);

  function updateOutcome(id: string, value: string) {
    setOutcomes((prev) => prev.map((o) => (o.id === id ? { ...o, value } : o)));
  }

  function addOutcome() {
    setOutcomes((prev) => [...prev, makeRow('')]);
  }

  function removeOutcome(id: string) {
    setOutcomes((prev) =>
      prev.length <= OUTCOMES_MIN ? prev : prev.filter((o) => o.id !== id),
    );
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);

    const nextErrors = validate(title, description, category, outcomes, endsAt);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) {
      return;
    }

    const input: CreateMarketInput = {
      title: title.trim(),
      description: description.trim(),
      category,
      outcomes: outcomes.map((o) => o.value.trim()),
      endsAt: new Date(`${endsAt}T23:59:59`).toISOString(),
      createdBy: address ?? undefined,
    };

    setSubmitting(true);
    try {
      const signature = await authorize(`Create market: ${input.title}`);
      if (!signature) {
        setFormError('Authorization was declined. Please sign to create the market.');
        return;
      }
      const market = await createMarket(input);
      router.push(`/markets/${market.id}`);
    } catch (err) {
      setFormError(
        err instanceof Error ? err.message : 'Something went wrong creating the market.',
      );
    } finally {
      setSubmitting(false);
    }
  }

  if (!isConnected) {
    return (
      <div className="mc-shell">
        <div className="page-head">
          <h1>Create a market</h1>
        </div>
        <Card className="mc-card">
          <EmptyState
            title="Connect your wallet to create a market"
            message="Creating a market requires a connected Stellar wallet to sign and attribute your market."
            action={
              <Button onClick={() => void connect()} loading={isConnecting}>
                Connect wallet
              </Button>
            }
          />
        </Card>
      </div>
    );
  }

  return (
    <div className="mc-shell">
      <div className="page-head">
        <h1>Create a market</h1>
      </div>

      <Card className="mc-card">
        <form className="mc-form" onSubmit={handleSubmit} noValidate>
          <Field
            label="Title"
            htmlFor="market-title"
            required
            hint={`${TITLE_MIN}–${TITLE_MAX} characters`}
            error={errors.title}
          >
            <Input
              id="market-title"
              name="title"
              value={title}
              maxLength={TITLE_MAX}
              placeholder="Will ETH flip BTC by market cap in 2026?"
              aria-invalid={Boolean(errors.title)}
              aria-describedby={errors.title ? 'market-title-error' : 'market-title-hint'}
              onChange={(e) => setTitle(e.target.value)}
            />
          </Field>

          <Field
            label="Description"
            htmlFor="market-description"
            required
            hint={`${DESC_MIN}–${DESC_MAX} characters — explain the resolution criteria`}
            error={errors.description}
          >
            <Textarea
              id="market-description"
              name="description"
              rows={5}
              value={description}
              maxLength={DESC_MAX}
              placeholder="Resolves YES if…"
              aria-invalid={Boolean(errors.description)}
              aria-describedby={
                errors.description ? 'market-description-error' : 'market-description-hint'
              }
              onChange={(e) => setDescription(e.target.value)}
            />
          </Field>

          <Field label="Category" htmlFor="market-category" required error={errors.category}>
            <Select
              id="market-category"
              name="category"
              value={category}
              aria-invalid={Boolean(errors.category)}
              aria-describedby={errors.category ? 'market-category-error' : undefined}
              onChange={(e) => setCategory(e.target.value)}
            >
              {MARKET_CATEGORIES.map((cat) => (
                <option key={cat} value={cat}>
                  {cat}
                </option>
              ))}
            </Select>
          </Field>

          <div className="mc-outcomes" role="group" aria-labelledby="mc-outcomes-label">
            <div className="mc-outcomes-head">
              <span className="mc-outcomes-label" id="mc-outcomes-label">
                Outcomes
                <span aria-label="required" style={{ color: 'var(--destructive)', marginLeft: 4 }}>
                  *
                </span>
              </span>
              <span className="field-hint">At least {OUTCOMES_MIN}, each unique</span>
            </div>

            {outcomes.map((outcome, index) => (
              <div className="mc-outcome-row" key={outcome.id}>
                <Input
                  id={outcome.id}
                  aria-label={`Outcome ${index + 1}`}
                  value={outcome.value}
                  placeholder={`Outcome ${index + 1}`}
                  aria-invalid={Boolean(errors.outcomes)}
                  onChange={(e) => updateOutcome(outcome.id, e.target.value)}
                />
                <button
                  type="button"
                  className="mc-outcome-remove"
                  aria-label={`Remove outcome ${index + 1}`}
                  disabled={outcomes.length <= OUTCOMES_MIN}
                  onClick={() => removeOutcome(outcome.id)}
                >
                  ×
                </button>
              </div>
            ))}

            {errors.outcomes && (
              <span className="field-error" id="market-outcomes-error" role="alert">
                {errors.outcomes}
              </span>
            )}

            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="mc-outcomes-add"
              onClick={addOutcome}
            >
              + Add outcome
            </Button>
          </div>

          <Field label="End date" htmlFor="market-ends-at" required error={errors.endsAt}>
            <Input
              id="market-ends-at"
              name="endsAt"
              type="date"
              value={endsAt}
              min={todayIso()}
              aria-invalid={Boolean(errors.endsAt)}
              aria-describedby={errors.endsAt ? 'market-ends-at-error' : undefined}
              onChange={(e) => setEndsAt(e.target.value)}
            />
          </Field>

          {formError && (
            <p className="mc-form-error" role="alert">
              {formError}
            </p>
          )}

          <div className="mc-actions">
            <Button type="submit" loading={submitting} block>
              Create market
            </Button>
          </div>
        </form>
      </Card>
    </div>
  );
}
