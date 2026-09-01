import { readdirSync, readFileSync } from 'fs';
import { join } from 'path';
import { fillPath, fillPathParams } from '../paths';

const API_DIR = join(__dirname, '..');

describe('fillPath', () => {
  it('URI-encodes the substituted value', () => {
    expect(fillPath('/api/v1/markets/{market_id}/resolve', 'market_id', 'a/b?c#d')).toBe(
      '/api/v1/markets/a%2Fb%3Fc%23d/resolve',
    );
  });

  it('encodes exactly once (no double-encoding through a chain)', () => {
    const once = fillPath('/x/{id}', 'id', 'a b');
    expect(once).toBe('/x/a%20b');
    // Feeding an already-filled path back in is a no-op (nothing left to replace).
    expect(fillPath(once, 'id', 'ignored')).toBe(once);
  });

  it('fillPathParams substitutes every placeholder', () => {
    expect(
      fillPathParams('/u/{user}/tx/{hash}', { user: 'a/b', hash: 'x#y' }),
    ).toBe('/u/a%2Fb/tx/x%23y');
  });
});

describe('path values containing / ? # round-trip through a request', () => {
  const originalFetch = global.fetch;

  beforeEach(() => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ ok: true }),
    });
  });
  afterEach(() => {
    global.fetch = originalFetch;
  });

  const requestedUrl = () => (global.fetch as jest.Mock).mock.calls[0][0] as string;

  it('a market id with special characters lands in the path, not as a new segment or query', async () => {
    const { api } = await import('../public-client');
    await api.getBlockchainMarket('m/1?x#y');

    const url = new URL(requestedUrl());
    // The whole value is one encoded path segment - no stray `/`, `?`, or `#`.
    expect(url.pathname).toBe('/api/v1/blockchain/markets/m%2F1%3Fx%23y');
    expect(url.search).toBe('');
  });

  it('a tx hash with a slash does not escape the path', async () => {
    const { api } = await import('../public-client');
    await api.getTransactionStatus('deadbeef/../secret');

    const url = new URL(requestedUrl());
    expect(url.pathname).toBe('/api/v1/blockchain/tx/deadbeef%2F..%2Fsecret');
  });
});

describe('encodeURIComponent is centralized in paths.ts', () => {
  it('no other src/lib/api module calls encodeURIComponent directly', () => {
    const offenders: string[] = [];
    for (const entry of readdirSync(API_DIR, { withFileTypes: true })) {
      if (!entry.isFile() || !entry.name.endsWith('.ts')) continue;
      if (entry.name === 'paths.ts' || entry.name.endsWith('.d.ts')) continue;
      const source = readFileSync(join(API_DIR, entry.name), 'utf8');
      if (source.includes('encodeURIComponent')) offenders.push(entry.name);
    }
    expect(offenders).toEqual([]);
  });
});
