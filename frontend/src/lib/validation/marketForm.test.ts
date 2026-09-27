import { describe, expect, it } from 'vitest';

import {
  MAX_OUTCOMES,
  MIN_CLOSE_TIME_LEAD_MS,
  MIN_OUTCOMES,
  marketFormSchema,
} from './marketForm';

const validValues = () => ({
  question: 'Will it rain tomorrow?',
  description: 'A simple weather market.',
  outcomes: ['Yes', 'No'],
  closeTime: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
});

describe('marketFormSchema', () => {
  it('accepts a valid market form', () => {
    const result = marketFormSchema.safeParse(validValues());
    expect(result.success).toBe(true);
  });

  it('requires a question', () => {
    const result = marketFormSchema.safeParse({
      ...validValues(),
      question: '   ',
    });
    expect(result.success).toBe(false);
  });

  it(`requires at least ${MIN_OUTCOMES} outcomes`, () => {
    const result = marketFormSchema.safeParse({
      ...validValues(),
      outcomes: ['Only one'],
    });
    expect(result.success).toBe(false);
  });

  it(`allows at most ${MAX_OUTCOMES} outcomes`, () => {
    const result = marketFormSchema.safeParse({
      ...validValues(),
      outcomes: Array.from({ length: MAX_OUTCOMES + 1 }, (_, i) => `Outcome ${i}`),
    });
    expect(result.success).toBe(false);
  });

  it('rejects a close time in the past', () => {
    const result = marketFormSchema.safeParse({
      ...validValues(),
      closeTime: new Date(Date.now() - 60 * 1000).toISOString(),
    });
    expect(result.success).toBe(false);
  });

  it('rejects a close time inside the minimum lead-time window', () => {
    const result = marketFormSchema.safeParse({
      ...validValues(),
      closeTime: new Date(Date.now() + MIN_CLOSE_TIME_LEAD_MS / 2).toISOString(),
    });
    expect(result.success).toBe(false);
  });

  it('accepts a close time beyond the minimum lead-time window', () => {
    const result = marketFormSchema.safeParse({
      ...validValues(),
      closeTime: new Date(Date.now() + MIN_CLOSE_TIME_LEAD_MS * 2).toISOString(),
    });
    expect(result.success).toBe(true);
  });
});
