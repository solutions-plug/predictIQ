/**
 * Contract test (#3 — API client paths must match the generated OpenAPI schema).
 *
 * public-client.ts / admin-client.ts previously hardcoded paths like
 * `/api/statistics`, `/api/markets/featured`, `/api/content`,
 * `/api/blockchain/...`, and `/api/markets/{id}/resolve`, while
 * schema.d.ts — auto-generated from services/api/openapi.yaml, the source
 * of truth — defines all of these under an `/api/v1/...` prefix. Every one
 * of those calls 404'd against the real backend.
 *
 * This test reads the literal path keys out of the `paths` interface in
 * schema.d.ts (rather than a mocked api.* function) and asserts that every
 * request the clients actually make resolves to one of those schema-defined
 * paths, catching prefix/path drift instead of a runtime 404.
 */

import fs from 'fs';
import path from 'path';
import { api as publicApi } from '../public-client';
import { api as adminApi } from '../admin-client';

function schemaPathKeys(): Set<string> {
  const schemaSrc = fs.readFileSync(path.join(__dirname, '../schema.d.ts'), 'utf8');
  const pathsBlock = schemaSrc.match(/export interface paths \{([\s\S]*?)\n\}\n/);
  if (!pathsBlock) {
    throw new Error('Could not locate the `paths` interface in schema.d.ts');
  }
  const keys = new Set<string>();
  const keyPattern = /^\s{4}"([^"]+)":\s*\{/gm;
  let match: RegExpExecArray | null;
  while ((match = keyPattern.exec(pathsBlock[1]))) {
    keys.add(match[1]);
  }
  return keys;
}

function fillTemplate(template: string, params: Record<string, string>): string {
  let filled = template;
  for (const [name, value] of Object.entries(params)) {
    filled = filled.replace(`{${name}}`, encodeURIComponent(value));
  }
  return filled;
}

describe('API client paths match schema.d.ts (contract test, #3)', () => {
  const schemaPaths = schemaPathKeys();

  const mockOk = (data: unknown = {}) =>
    ((global.fetch as jest.Mock) = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify(data),
    }));

  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  const cases: Array<{
    name: string;
    template: string;
    params: Record<string, string>;
    call: () => Promise<unknown>;
  }> = [
    { name: 'getStatistics', template: '/api/v1/statistics', params: {}, call: () => publicApi.getStatistics() },
    {
      name: 'getFeaturedMarkets',
      template: '/api/v1/markets/featured',
      params: {},
      call: () => publicApi.getFeaturedMarkets(),
    },
    { name: 'getContent', template: '/api/v1/content', params: {}, call: () => publicApi.getContent() },
    {
      name: 'getBlockchainHealth',
      template: '/api/v1/blockchain/health',
      params: {},
      call: () => publicApi.getBlockchainHealth(),
    },
    {
      name: 'getBlockchainMarket',
      template: '/api/v1/blockchain/markets/{market_id}',
      params: { market_id: '42' },
      call: () => publicApi.getBlockchainMarket('42'),
    },
    {
      name: 'getBlockchainStats',
      template: '/api/v1/blockchain/stats',
      params: {},
      call: () => publicApi.getBlockchainStats(),
    },
    {
      name: 'getUserBets',
      template: '/api/v1/blockchain/users/{user}/bets',
      params: { user: 'GABC' },
      call: () => publicApi.getUserBets('GABC'),
    },
    {
      name: 'getOracleResult',
      template: '/api/v1/blockchain/oracle/{market_id}',
      params: { market_id: '42' },
      call: () => publicApi.getOracleResult('42'),
    },
    {
      name: 'getTransactionStatus',
      template: '/api/v1/blockchain/tx/{tx_hash}',
      params: { tx_hash: '0xdead' },
      call: () => publicApi.getTransactionStatus('0xdead'),
    },
    {
      name: 'resolveMarket',
      template: '/api/v1/markets/{market_id}/resolve',
      params: { market_id: '42' },
      call: () => adminApi.resolveMarket('42'),
    },
  ];

  it('every path used by the client is declared in schema.d.ts', () => {
    for (const { template } of cases) {
      expect(schemaPaths.has(template)).toBe(true);
    }
  });

  it.each(cases.map((c) => [c.name, c] as const))(
    '%s requests exactly the schema-defined path',
    async (_name, { template, params, call }) => {
      mockOk();
      await call();
      const calledUrl = (global.fetch as jest.Mock).mock.calls[0][0] as string;
      const calledPath = new URL(calledUrl).pathname;
      expect(calledPath).toBe(fillTemplate(template, params));
    },
  );
});
