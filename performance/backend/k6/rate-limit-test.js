import http from 'k6/http';
import { check } from 'k6';
import { Rate, Counter } from 'k6/metrics';

const rateLimitHits = new Counter('rate_limit_hits');
const successfulRequests = new Counter('successful_requests');

export const options = {
  vus: 10,
  duration: '30s',
  thresholds: {
    rate_limit_hits: ['count>0'],  // Expect to hit rate limits
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  const res = http.get(`${BASE_URL}/api/v1/markets`, {
    tags: { endpoint: 'markets' },
  });
  
  if (res.status === 429) {
    rateLimitHits.add(1);
    check(res, {
      'rate limit response has retry-after header': (r) => r.headers['Retry-After'] !== undefined,
    });
  } else if (res.status === 200) {
    successfulRequests.add(1);
  }
  
  check(res, {
    'status is 200 or 429': (r) => r.status === 200 || r.status === 429,
  });
}

export function handleSummary(data) {
  const rateLimitCount = data.metrics.rate_limit_hits.values.count || 0;
  const successCount = data.metrics.successful_requests.values.count || 0;
  const total = rateLimitCount + successCount;
  
  console.log(`\nRate Limit Test Results:`);
  console.log(`Total Requests: ${total}`);
  console.log(`Successful: ${successCount} (${((successCount/total)*100).toFixed(2)}%)`);
  console.log(`Rate Limited: ${rateLimitCount} (${((rateLimitCount/total)*100).toFixed(2)}%)`);
  
  return {
    'backend/reports/rate-limit-test-summary.json': JSON.stringify(data),
  };
}
