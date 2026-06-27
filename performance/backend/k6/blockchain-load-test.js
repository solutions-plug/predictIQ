/**
 * Blockchain Data Endpoints Load Test
 *
 * Covers:
 *  - Market data queries (list, by-status, detail, outcome stakes)
 *  - Platform stats (revenue, fee config, circuit breaker state)
 *  - User bet queries (bets, winnings eligibility, referral rewards)
 *
 * Cache scenarios:
 *  - Cold start: first pass with no warm cache
 *  - Cache-warmed: repeated requests against a small hot-set of market IDs
 *
 * Results feed into the performance dashboard via handleSummary JSON output.
 */

import http from "k6/http";
import { check, group, sleep } from "k6";
import { Counter, Rate, Trend } from "k6/metrics";
import { randomIntBetween } from "https://jslib.k6.io/k6-utils/1.2.0/index.js";
import { getThresholdsForTest } from './slo-thresholds.js';

// ---------------------------------------------------------------------------
// Custom metrics
// ---------------------------------------------------------------------------

// Response time trends per endpoint group
const marketDataTrend = new Trend("blockchain_market_data_duration", true);
const statsTrend = new Trend("blockchain_stats_duration", true);
const userBetsTrend = new Trend("blockchain_user_bets_duration", true);
const outcomeTrend = new Trend("blockchain_outcome_stake_duration", true);

// Cache effectiveness
const cacheHits = new Counter("blockchain_cache_hits");
const cacheMisses = new Counter("blockchain_cache_misses");
const cacheHitRate = new Rate("blockchain_cache_hit_rate");

// Error tracking
const errorRate = new Rate("blockchain_errors");
const totalRequests = new Counter("blockchain_total_requests");

// ---------------------------------------------------------------------------
// Test configuration
// ---------------------------------------------------------------------------

const BASE_URL = __ENV.API_URL || "http://localhost:8080";

// Small hot-set of market IDs used in the cache-warmed scenario.
// Keeping this narrow (10 IDs) maximises cache hit probability.
const HOT_MARKET_IDS = Array.from({ length: 10 }, (_, i) => i + 1);

// Wider cold-set simulates first-time access patterns.
const COLD_MARKET_POOL_SIZE = 500;

// User pool for bet/stats queries
const USER_POOL_SIZE = 200;

// Market statuses exposed by the API
const MARKET_STATUSES = [
  "Active",
  "PendingResolution",
  "Disputed",
  "Resolved",
  "Cancelled",
];

export const options = {
  scenarios: {
    // -----------------------------------------------------------------------
    // Scenario 1 – Cache-warmed steady load
    // Simulates normal production traffic where popular markets are cached.
    // -----------------------------------------------------------------------
    cache_warmed: {
      executor: "ramping-vus",
      startVUs: 0,
      stages: [
        { duration: "1m", target: 50 }, // warm up
        { duration: "4m", target: 50 }, // sustained warmed load
        { duration: "1m", target: 0 }, // ramp down
      ],
      env: { SCENARIO: "warmed" },
      tags: { scenario: "cache_warmed" },
    },

    // -----------------------------------------------------------------------
    // Scenario 2 – Cold cache burst
    // Simulates cache invalidation events or a fresh deployment where the
    // cache is empty and every request hits the data layer.
    // -----------------------------------------------------------------------
    cold_cache: {
      executor: "ramping-vus",
      startVUs: 0,
      stages: [
        { duration: "30s", target: 20 }, // short ramp — cold requests are expensive
        { duration: "2m", target: 20 }, // sustained cold load
        { duration: "30s", target: 0 }, // ramp down
      ],
      startTime: "6m30s", // run after warmed scenario completes
      env: { SCENARIO: "cold" },
      tags: { scenario: "cold_cache" },
    },
  },

  thresholds: {
    ...getThresholdsForTest('blockchain'),
    // Cache effectiveness — warmed traffic should hit cache >80% of the time
    blockchain_cache_hit_rate: ['rate>0.8'],

    // Per-group latency targets
    blockchain_market_data_duration: ['p(95)<200', 'p(99)<400'],
    blockchain_stats_duration: ['p(95)<150', 'p(99)<300'],
    blockchain_user_bets_duration: ['p(95)<250', 'p(99)<500'],
    blockchain_outcome_stake_duration: ['p(95)<150', 'p(99)<300'],

    // Warmed-scenario latency should be tighter (cache serving responses)
    'http_req_duration{scenario:cache_warmed}': ['p(95)<150'],

    // Cold-scenario latency is allowed to be higher (cache miss penalty)
    'http_req_duration{scenario:cold_cache}': ['p(95)<400'],
  },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Pick a market ID from the hot-set (warmed) or the full cold pool.
 */
function pickMarketId() {
  if (__ENV.SCENARIO === "warmed") {
    return HOT_MARKET_IDS[randomIntBetween(0, HOT_MARKET_IDS.length - 1)];
  }
  return randomIntBetween(1, COLD_MARKET_POOL_SIZE);
}

/**
 * Inspect response headers to determine whether the response was served
 * from cache. Supports common cache header conventions.
 */
function detectCacheHit(res) {
  const xCache = (res.headers["X-Cache"] || "").toUpperCase();
  const xCacheStatus = (res.headers["X-Cache-Status"] || "").toUpperCase();
  const cfCacheStatus = (res.headers["Cf-Cache-Status"] || "").toUpperCase();
  const age = res.headers["Age"];

  return (
    xCache.includes("HIT") ||
    xCacheStatus === "HIT" ||
    cfCacheStatus === "HIT" ||
    (age !== undefined && parseInt(age, 10) > 0)
  );
}

/**
 * Record cache hit/miss and add to the rate metric.
 */
function recordCacheMetric(res) {
  const hit = detectCacheHit(res);
  if (hit) {
    cacheHits.add(1);
    cacheHitRate.add(1);
  } else {
    cacheMisses.add(1);
    cacheHitRate.add(0);
  }
  return hit;
}

/**
 * Shared GET helper — records totals, cache metrics, and returns the response.
 */
function get(url, params) {
  totalRequests.add(1);
  const res = http.get(url, params);
  recordCacheMetric(res);
  return res;
}

// ---------------------------------------------------------------------------
// Endpoint groups
// ---------------------------------------------------------------------------

/**
 * Market data queries — the highest-traffic blockchain read path.
 */
function marketDataRequests() {
  group("market_data", () => {
    // 1. Paginated market list
    const listRes = get(`${BASE_URL}/api/v1/markets?offset=0&limit=20`, {
      tags: { endpoint: "markets_list", scenario: __ENV.SCENARIO },
    });
    marketDataTrend.add(listRes.timings.duration);
    const listOk = check(listRes, {
      "markets list: status 200": (r) => r.status === 200,
      "markets list: response time <200ms": (r) => r.timings.duration < 200,
      "markets list: body is array": (r) => {
        try {
          return Array.isArray(JSON.parse(r.body));
        } catch {
          return false;
        }
      },
    });
    if (!listOk) errorRate.add(1);

    sleep(0.2);

    // 2. Markets filtered by status
    const status =
      MARKET_STATUSES[randomIntBetween(0, MARKET_STATUSES.length - 1)];
    const byStatusRes = get(
      `${BASE_URL}/api/v1/markets?status=${status}&offset=0&limit=20`,
      { tags: { endpoint: "markets_by_status", scenario: __ENV.SCENARIO } },
    );
    marketDataTrend.add(byStatusRes.timings.duration);
    const byStatusOk = check(byStatusRes, {
      "markets by status: status 200 or 404": (r) =>
        r.status === 200 || r.status === 404,
      "markets by status: response time <200ms": (r) =>
        r.timings.duration < 200,
    });
    if (!byStatusOk) errorRate.add(1);

    sleep(0.2);

    // 3. Individual market detail
    const marketId = pickMarketId();
    const detailRes = get(`${BASE_URL}/api/v1/markets/${marketId}`, {
      tags: { endpoint: "market_detail", scenario: __ENV.SCENARIO },
    });
    marketDataTrend.add(detailRes.timings.duration);
    const detailOk = check(detailRes, {
      "market detail: status 200 or 404": (r) =>
        r.status === 200 || r.status === 404,
      "market detail: response time <200ms": (r) => r.timings.duration < 200,
    });
    if (!detailOk) errorRate.add(1);
  });
}

/**
 * Outcome stake queries — per-outcome totals and bettor counts.
 */
function outcomeStakeRequests() {
  group("outcome_stakes", () => {
    const marketId = pickMarketId();
    const outcomeIdx = randomIntBetween(0, 3); // markets typically have 2–4 outcomes

    // Total stake on an outcome
    const stakeRes = get(
      `${BASE_URL}/api/v1/markets/${marketId}/outcomes/${outcomeIdx}/stake`,
      { tags: { endpoint: "outcome_stake", scenario: __ENV.SCENARIO } },
    );
    outcomeTrend.add(stakeRes.timings.duration);
    const stakeOk = check(stakeRes, {
      "outcome stake: status 200 or 404": (r) =>
        r.status === 200 || r.status === 404,
      "outcome stake: response time <150ms": (r) => r.timings.duration < 150,
    });
    if (!stakeOk) errorRate.add(1);

    sleep(0.1);

    // Unique bettor count for the same outcome
    const countRes = get(
      `${BASE_URL}/api/v1/markets/${marketId}/outcomes/${outcomeIdx}/bets/count`,
      { tags: { endpoint: "outcome_bet_count", scenario: __ENV.SCENARIO } },
    );
    outcomeTrend.add(countRes.timings.duration);
    const countOk = check(countRes, {
      "outcome bet count: status 200 or 404": (r) =>
        r.status === 200 || r.status === 404,
      "outcome bet count: response time <150ms": (r) =>
        r.timings.duration < 150,
    });
    if (!countOk) errorRate.add(1);
  });
}

/**
 * Platform stats — fee config, revenue, circuit breaker state.
 * These are read-heavy, low-cardinality endpoints that should be well-cached.
 */
function statsRequests() {
  group("platform_stats", () => {
    // Base fee
    const feeRes = get(`${BASE_URL}/api/v1/config/base-fee`, {
      tags: { endpoint: "base_fee", scenario: __ENV.SCENARIO },
    });
    statsTrend.add(feeRes.timings.duration);
    const feeOk = check(feeRes, {
      "base fee: status 200": (r) => r.status === 200,
      "base fee: response time <150ms": (r) => r.timings.duration < 150,
    });
    if (!feeOk) errorRate.add(1);

    sleep(0.1);

    // Minimum bet amount
    const minBetRes = get(`${BASE_URL}/api/v1/config/minimum-bet`, {
      tags: { endpoint: "minimum_bet", scenario: __ENV.SCENARIO },
    });
    statsTrend.add(minBetRes.timings.duration);
    const minBetOk = check(minBetRes, {
      "minimum bet: status 200": (r) => r.status === 200,
      "minimum bet: response time <150ms": (r) => r.timings.duration < 150,
    });
    if (!minBetOk) errorRate.add(1);

    sleep(0.1);

    // Circuit breaker state
    const cbRes = get(`${BASE_URL}/api/v1/config/circuit-breaker`, {
      tags: { endpoint: "circuit_breaker", scenario: __ENV.SCENARIO },
    });
    statsTrend.add(cbRes.timings.duration);
    const cbOk = check(cbRes, {
      "circuit breaker: status 200": (r) => r.status === 200,
      "circuit breaker: response time <150ms": (r) => r.timings.duration < 150,
    });
    if (!cbOk) errorRate.add(1);

    sleep(0.1);

    // Dispute window config
    const dwRes = get(`${BASE_URL}/api/v1/config/dispute-window`, {
      tags: { endpoint: "dispute_window", scenario: __ENV.SCENARIO },
    });
    statsTrend.add(dwRes.timings.duration);
    const dwOk = check(dwRes, {
      "dispute window: status 200": (r) => r.status === 200,
      "dispute window: response time <150ms": (r) => r.timings.duration < 150,
    });
    if (!dwOk) errorRate.add(1);
  });
}

/**
 * User bet queries — per-user bets, winnings, and referral rewards.
 * Higher cardinality than stats; cache hit rate will be lower.
 */
function userBetsRequests() {
  group("user_bets", () => {
    const userId = `user_${randomIntBetween(1, USER_POOL_SIZE)}`;
    const marketId = pickMarketId();

    // User's bets on a specific market
    const betsRes = get(
      `${BASE_URL}/api/v1/users/${userId}/markets/${marketId}/bets`,
      { tags: { endpoint: "user_market_bets", scenario: __ENV.SCENARIO } },
    );
    userBetsTrend.add(betsRes.timings.duration);
    const betsOk = check(betsRes, {
      "user market bets: status 200 or 404": (r) =>
        r.status === 200 || r.status === 404,
      "user market bets: response time <250ms": (r) => r.timings.duration < 250,
    });
    if (!betsOk) errorRate.add(1);

    sleep(0.2);

    // User's overall stats
    const statsRes = get(`${BASE_URL}/api/v1/users/${userId}/stats`, {
      tags: { endpoint: "user_stats", scenario: __ENV.SCENARIO },
    });
    userBetsTrend.add(statsRes.timings.duration);
    const statsOk = check(statsRes, {
      "user stats: status 200 or 404": (r) =>
        r.status === 200 || r.status === 404,
      "user stats: response time <250ms": (r) => r.timings.duration < 250,
    });
    if (!statsOk) errorRate.add(1);

    sleep(0.2);

    // Referral rewards balance
    const refRes = get(`${BASE_URL}/api/v1/users/${userId}/referral-rewards`, {
      tags: { endpoint: "referral_rewards", scenario: __ENV.SCENARIO },
    });
    userBetsTrend.add(refRes.timings.duration);
    const refOk = check(refRes, {
      "referral rewards: status 200 or 404": (r) =>
        r.status === 200 || r.status === 404,
      "referral rewards: response time <250ms": (r) => r.timings.duration < 250,
    });
    if (!refOk) errorRate.add(1);
  });
}

// ---------------------------------------------------------------------------
// Default function — weighted traffic mix
// ---------------------------------------------------------------------------

export default function () {
  const roll = randomIntBetween(1, 100);

  if (roll <= 45) {
    // 45% — market data (most common read path)
    marketDataRequests();
  } else if (roll <= 65) {
    // 20% — outcome stakes (analytics / UI rendering)
    outcomeStakeRequests();
  } else if (roll <= 80) {
    // 15% — platform stats (config reads, heavily cached)
    statsRequests();
  } else {
    // 20% — user bet queries (personalised, lower cache hit rate)
    userBetsRequests();
  }

  sleep(randomIntBetween(1, 3));
}

// ---------------------------------------------------------------------------
// Summary report
// ---------------------------------------------------------------------------

export function handleSummary(data) {
  const m = data.metrics;

  const hits =
    (m.blockchain_cache_hits && m.blockchain_cache_hits.values.count) || 0;
  const misses =
    (m.blockchain_cache_misses && m.blockchain_cache_misses.values.count) || 0;
  const total = hits + misses;
  const hitPct = total > 0 ? ((hits / total) * 100).toFixed(2) : "N/A";

  const p95Overall = m.http_req_duration
    ? m.http_req_duration.values["p(95)"].toFixed(2)
    : "N/A";

  const p95Warmed = m["http_req_duration{scenario:cache_warmed}"]
    ? m["http_req_duration{scenario:cache_warmed}"].values["p(95)"].toFixed(2)
    : "N/A";

  const p95Cold = m["http_req_duration{scenario:cold_cache}"]
    ? m["http_req_duration{scenario:cold_cache}"].values["p(95)"].toFixed(2)
    : "N/A";

  const errorPct = m.blockchain_errors
    ? (m.blockchain_errors.values.rate * 100).toFixed(2)
    : "N/A";

  const summary = {
    generated_at: new Date().toISOString(),
    cache: {
      hits,
      misses,
      total,
      hit_rate_pct: hitPct,
      threshold_pct: 80,
      passed: parseFloat(hitPct) >= 80,
    },
    latency: {
      p95_overall_ms: p95Overall,
      p95_warmed_ms: p95Warmed,
      p95_cold_ms: p95Cold,
      threshold_warmed_ms: 150,
      threshold_cold_ms: 400,
    },
    errors: {
      rate_pct: errorPct,
      threshold_pct: 1,
      passed: parseFloat(errorPct) < 1,
    },
    raw: data,
  };

  return {
    "backend/reports/blockchain-load-test-summary.json": JSON.stringify(
      summary,
      null,
      2,
    ),
    stdout: buildConsoleReport(summary),
  };
}

function buildConsoleReport(s) {
  const pass = (ok) => (ok ? "✓ PASS" : "✗ FAIL");

  return `
╔══════════════════════════════════════════════════════════╗
║         Blockchain Data Endpoints — Load Test            ║
╚══════════════════════════════════════════════════════════╝

  Generated : ${s.generated_at}

  Cache Effectiveness
  ───────────────────
  Total requests : ${s.cache.total}
  Cache hits     : ${s.cache.hits}  (${s.cache.hit_rate_pct}%)
  Cache misses   : ${s.cache.misses}
  Threshold      : >${s.cache.threshold_pct}%   ${pass(s.cache.passed)}

  Latency (p95)
  ─────────────
  Overall        : ${s.latency.p95_overall_ms} ms
  Cache-warmed   : ${s.latency.p95_warmed_ms} ms  (threshold <${s.latency.threshold_warmed_ms} ms)
  Cold cache     : ${s.latency.p95_cold_ms} ms  (threshold <${s.latency.threshold_cold_ms} ms)

  Errors
  ──────
  Error rate     : ${s.errors.rate_pct}%  (threshold <${s.errors.threshold_pct}%)  ${pass(s.errors.passed)}

  Full JSON report → backend/reports/blockchain-load-test-summary.json
`;
}
