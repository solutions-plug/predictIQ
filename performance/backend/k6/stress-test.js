import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';

const errorRate = new Rate('errors');
const responseTime = new Trend('response_time');

export const options = {
  stages: [
    { duration: '2m', target: 100 },   // Ramp up to 100 users
    { duration: '5m', target: 100 },   // Stay at 100
    { duration: '2m', target: 200 },   // Ramp to 200
    { duration: '5m', target: 200 },   // Stay at 200
    { duration: '2m', target: 300 },   // Ramp to 300
    { duration: '5m', target: 300 },   // Stay at 300
    { duration: '2m', target: 400 },   // Ramp to 400
    { duration: '5m', target: 400 },   // Stay at 400
    { duration: '10m', target: 0 },    // Ramp down
  ],
  thresholds: {
    errors: ['rate<0.05'],  // Allow higher error rate in stress test
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  const endpoints = [
    { url: `${BASE_URL}/health`, name: 'health' },
    { url: `${BASE_URL}/api/v1/markets`, name: 'markets' },
    { url: `${BASE_URL}/api/v1/markets/${randomIntBetween(1, 100)}`, name: 'market_detail' },
    { url: `${BASE_URL}/metrics`, name: 'metrics' },
  ];
  
  const endpoint = endpoints[randomIntBetween(0, endpoints.length - 1)];
  const res = http.get(endpoint.url, {
    tags: { endpoint: endpoint.name },
  });
  
  responseTime.add(res.timings.duration);
  
  check(res, {
    'status is 200 or 404': (r) => r.status === 200 || r.status === 404,
  }) || errorRate.add(1);
  
  sleep(randomIntBetween(1, 2));
}

export function handleSummary(data) {
  return {
    'backend/reports/stress-test-summary.json': JSON.stringify(data),
  };
}
