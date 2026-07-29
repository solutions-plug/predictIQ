/**
 * Compile-time contract test (#4 — client.ts types must derive from
 * schema.d.ts, not be hand-duplicated).
 *
 * public-client.ts / admin-client.ts claimed to be "generated from the
 * OpenAPI schema" but never imported anything from schema.d.ts — every
 * request path and response shape was hand-typed, which is what let the
 * /api/v1/ prefix drift (see #50) ship undetected: nothing tied the client
 * to the backend contract.
 *
 * Both clients now declare their paths as `... satisfies Record<string,
 * keyof paths>` and their response generics as `components['schemas'][...]`.
 * `AssertPath<P>` below only compiles when `P` is a real key of the
 * generated `paths` interface, so if services/api/openapi.yaml changes and
 * schema.d.ts is regenerated without one of these paths, the corresponding
 * line fails `tsc --noEmit` / `next build` — turning what used to be a
 * silent runtime 404 into a build-time error.
 *
 * jest (via next/jest + SWC) strips types at transform time, so this file's
 * `it()` below only proves the module loads; the actual contract is
 * enforced by `npx tsc --noEmit --project tsconfig.json` and `npm run
 * build`, both required by CI and the PR checklist.
 */

import type { paths, components } from '../schema';

type AssertPath<P extends keyof paths> = P;

// Every path public-client.ts / admin-client.ts calls must resolve here —
// mirrors the `PATHS` objects in both modules.
type _Statistics = AssertPath<'/api/v1/statistics'>;
type _FeaturedMarkets = AssertPath<'/api/v1/markets/featured'>;
type _Content = AssertPath<'/api/v1/content'>;
type _BlockchainHealth = AssertPath<'/api/v1/blockchain/health'>;
type _BlockchainMarket = AssertPath<'/api/v1/blockchain/markets/{market_id}'>;
type _BlockchainStats = AssertPath<'/api/v1/blockchain/stats'>;
type _UserBets = AssertPath<'/api/v1/blockchain/users/{user}/bets'>;
type _OracleResult = AssertPath<'/api/v1/blockchain/oracle/{market_id}'>;
type _TransactionStatus = AssertPath<'/api/v1/blockchain/tx/{tx_hash}'>;
type _ResolveMarket = AssertPath<'/api/v1/markets/{market_id}/resolve'>;

// Deliberately-wrong path proving the check has teeth. Uncommenting this
// line fails `tsc --noEmit` with:
//   Type '"/api/v1/statistics/bogus"' does not satisfy the constraint 'keyof paths'.
// (verified manually — left commented so the build stays green)
// type _Mismatch = AssertPath<'/api/v1/statistics/bogus'>;

// Response shapes the clients request must also come from schema.d.ts,
// not be hand-duplicated inline object literals.
type _StatisticsShape = components['schemas']['AnyObject'];
type _FeaturedMarketShape = components['schemas']['FeaturedMarketView'];
type _BlockchainHealthShape = components['schemas']['BlockchainHealth'];
type _InvalidationResultShape = components['schemas']['InvalidationResult'];

describe('client.ts path/type contract with schema.d.ts (#4)', () => {
  it('compiles only because every AssertPath<...> above is a real schema.d.ts path', () => {
    expect(true).toBe(true);
  });
});
