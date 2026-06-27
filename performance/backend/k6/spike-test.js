import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';
import { getThresholdsForTest } from './slo-thresholds.js';

const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '10s', target: 100 },   // Normal load
    { duration: '1m', target: 100 },    // Stable
    { duration: '10s', target: 1000 },  // Spike!
    { duration: '3m', target: 1000 },   // Sustain spike
    { duration: '10s', target: 100 },   // Return to normal
    { duration: '3m', target: 100 },    // Recover
    { duration: '10s', target: 0 },     // Ramp down
  ],
  thresholds: getThresholdsForTest('spike'),
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  const res = http.get(`${BASE_URL}/api/v1/markets`, {
    tags: { endpoint: 'markets' },
  });
  
  check(res, {
    'status is 200': (r) => r.status === 200,
  }) || errorRate.add(1);
  
  sleep(1);
}

export function handleSummary(data) {
  return {
    'backend/reports/spike-test-summary.json': JSON.stringify(data),
  };
}
