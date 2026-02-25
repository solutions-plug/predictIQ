import http from 'k6/http';
import { check } from 'k6';
import { Counter, Rate } from 'k6/metrics';

const cacheHits = new Counter('cache_hits');
const cacheMisses = new Counter('cache_misses');
const cacheHitRate = new Rate('cache_hit_rate');

export const options = {
  vus: 50,
  duration: '2m',
  thresholds: {
    cache_hit_rate: ['rate>0.8'],  // Expect >80% cache hit rate
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  // Request same market multiple times to test caching
  const marketId = Math.floor(Math.random() * 10) + 1;  // Only 10 markets to increase cache hits
  
  const res = http.get(`${BASE_URL}/api/v1/markets/${marketId}`, {
    tags: { endpoint: 'markets', market_id: marketId },
  });
  
  // Check for cache headers
  const isCacheHit = res.headers['X-Cache'] === 'HIT' || 
                     res.headers['X-Cache-Status'] === 'HIT' ||
                     res.headers['Age'] !== undefined;
  
  if (isCacheHit) {
    cacheHits.add(1);
    cacheHitRate.add(1);
  } else {
    cacheMisses.add(1);
    cacheHitRate.add(0);
  }
  
  check(res, {
    'status is 200 or 404': (r) => r.status === 200 || r.status === 404,
    'response time < 100ms for cache hit': (r) => !isCacheHit || r.timings.duration < 100,
  });
}

export function handleSummary(data) {
  const hits = data.metrics.cache_hits.values.count || 0;
  const misses = data.metrics.cache_misses.values.count || 0;
  const total = hits + misses;
  const hitRate = total > 0 ? (hits / total) * 100 : 0;
  
  console.log(`\nCache Performance:`);
  console.log(`Total Requests: ${total}`);
  console.log(`Cache Hits: ${hits} (${hitRate.toFixed(2)}%)`);
  console.log(`Cache Misses: ${misses} (${(100-hitRate).toFixed(2)}%)`);
  console.log(`Target: >80% hit rate - ${hitRate >= 80 ? 'PASS' : 'FAIL'}`);
  
  return {
    'backend/reports/cache-test-summary.json': JSON.stringify(data),
  };
}
