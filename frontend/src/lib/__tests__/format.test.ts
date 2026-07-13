import {
  formatXLM,
  formatNumber,
  formatDate,
  timeUntil,
  outcomeOdds,
  shortAddress,
} from '../format';
import type { Market } from '../types';

function market(pools: Record<number, number>): Market {
  const totalVolume = Object.values(pools).reduce((a, b) => a + b, 0);
  return {
    id: 'm',
    title: 't',
    description: 'd',
    category: 'Crypto',
    outcomes: Object.keys(pools).map((id) => ({ id: Number(id), label: `o${id}` })),
    poolByOutcome: pools,
    totalVolume,
    endsAt: new Date().toISOString(),
    status: 'open',
    createdAt: new Date().toISOString(),
  };
}

describe('format helpers', () => {
  it('formats XLM with and without compact notation', () => {
    expect(formatXLM(1234.5)).toContain('XLM');
    expect(formatXLM(1_500_000, { compact: true })).toMatch(/M XLM$/);
    expect(formatXLM(50, { compact: true })).toBe('50 XLM');
  });

  it('formats plain numbers', () => {
    expect(formatNumber(1000)).toBe('1,000');
    expect(formatNumber(1500, true)).toMatch(/1.5K/);
  });

  it('formats an ISO date', () => {
    expect(formatDate('2026-01-15T00:00:00.000Z')).toMatch(/2026/);
  });

  it('describes time until a future or past date', () => {
    expect(timeUntil(new Date(Date.now() - 1000).toISOString())).toBe('Ended');
    expect(timeUntil(new Date(Date.now() + 3 * 86_400_000).toISOString())).toBe('3d left');
    expect(timeUntil(new Date(Date.now() + 5 * 3_600_000).toISOString())).toBe('5h left');
    expect(timeUntil(new Date(Date.now() + 10 * 60_000).toISOString())).toMatch(/m left$/);
  });

  it('computes implied odds from pools, splitting evenly with no liquidity', () => {
    const m = market({ 0: 25, 1: 75 });
    expect(outcomeOdds(m, 1)).toBe(75);
    expect(outcomeOdds(m, 0)).toBe(25);

    const empty = market({ 0: 0, 1: 0 });
    expect(outcomeOdds(empty, 0)).toBe(50);
  });

  it('shortens long addresses and leaves short ones intact', () => {
    expect(shortAddress('GABCDEFGHIJKLMNOP')).toBe('GABC…MNOP');
    expect(shortAddress('SHORT')).toBe('SHORT');
  });
});
