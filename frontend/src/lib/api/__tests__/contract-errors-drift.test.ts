import { readFileSync } from 'fs';
import { join } from 'path';
import { getContractError, knownContractErrorCodes } from '../contractErrors';
import { CONTRACT_ERROR_MESSAGES, getContractErrorMessage } from '../admin-client';

/**
 * #1338 - drift detection: every code documented in docs/CONTRACT_ERRORS.md must have
 * both a message (admin-client.ts) and a short label (contractErrors.ts), and neither
 * table may carry a code the doc does not. Renaming or removing a variant in
 * contracts/predict-iq/src/errors.rs (and regenerating the doc) fails this test until
 * the frontend tables catch up.
 */

function documentedCodes(): number[] {
  // __tests__ -> api -> lib -> src -> frontend -> monorepo root -> docs/
  const doc = readFileSync(
    join(__dirname, '..', '..', '..', '..', '..', 'docs', 'CONTRACT_ERRORS.md'),
    'utf8',
  );
  const codes: number[] = [];
  for (const line of doc.split('\n')) {
    const m = /^\|\s*(\d+)\s*\|\s*`[A-Za-z0-9]+`\s*\|/.exec(line);
    if (m) codes.push(Number(m[1]));
  }
  return codes.sort((a, b) => a - b);
}

describe('contract error tables track docs/CONTRACT_ERRORS.md', () => {
  const docCodes = documentedCodes();

  it('the doc actually parsed (sanity)', () => {
    expect(docCodes.length).toBeGreaterThanOrEqual(60);
    expect(docCodes).toContain(101);
    expect(docCodes).toContain(160);
  });

  it('every documented code has a message', () => {
    const missing = docCodes.filter((c) => !(c in CONTRACT_ERROR_MESSAGES));
    expect(missing).toEqual([]);
  });

  it('every documented code has a short label', () => {
    const labelled = new Set(knownContractErrorCodes());
    const missing = docCodes.filter((c) => !labelled.has(c));
    expect(missing).toEqual([]);
  });

  it('the message table has no code the doc does not', () => {
    const docSet = new Set(docCodes);
    const orphans = Object.keys(CONTRACT_ERROR_MESSAGES)
      .map(Number)
      .filter((c) => !docSet.has(c));
    expect(orphans).toEqual([]);
  });

  it('the label table has no code the doc does not', () => {
    const docSet = new Set(docCodes);
    const orphans = knownContractErrorCodes().filter((c) => !docSet.has(c));
    expect(orphans).toEqual([]);
  });
});

describe('unmapped codes fall back safely', () => {
  it('getContractErrorMessage returns a generic string, never throws', () => {
    expect(() => getContractErrorMessage(99999)).not.toThrow();
    expect(getContractErrorMessage(99999)).toMatch(/contract error/i);
  });

  it('getContractError returns a full label+message with no undefined', () => {
    const err = getContractError(99999);
    expect(err.code).toBe(99999);
    expect(typeof err.label).toBe('string');
    expect(err.label.length).toBeGreaterThan(0);
    expect(typeof err.message).toBe('string');
    expect(err.message.length).toBeGreaterThan(0);
  });

  it('getContractError resolves a known code to its documented pair', () => {
    const err = getContractError(102);
    expect(err.label).toBe('Market not found');
    expect(err.message).toBe(CONTRACT_ERROR_MESSAGES[102]);
  });
});
