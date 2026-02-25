import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export const options = {
  vus: 1,
  duration: '1m',
  thresholds: {
    errors: ['rate<0.01'],
    http_req_duration: ['p(95)<200'],
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  // Health check
  let res = http.get(`${BASE_URL}/health`, {
    tags: { endpoint: 'health' },
  });
  
  check(res, {
    'health check status is 200': (r) => r.status === 200,
    'health check response time < 50ms': (r) => r.timings.duration < 50,
  }) || errorRate.add(1);

  sleep(1);

  // Get markets list
  res = http.get(`${BASE_URL}/api/v1/markets`, {
    tags: { endpoint: 'markets' },
  });
  
  check(res, {
    'markets status is 200': (r) => r.status === 200,
    'markets response time < 200ms': (r) => r.timings.duration < 200,
    'markets returns array': (r) => Array.isArray(JSON.parse(r.body)),
  }) || errorRate.add(1);

  sleep(1);

  // Get metrics
  res = http.get(`${BASE_URL}/metrics`, {
    tags: { endpoint: 'metrics' },
  });
  
  check(res, {
    'metrics status is 200': (r) => r.status === 200,
  }) || errorRate.add(1);

  sleep(1);
}

export function handleSummary(data) {
  return {
    'backend/reports/smoke-test-summary.json': JSON.stringify(data),
    stdout: textSummary(data, { indent: ' ', enableColors: true }),
  };
}

function textSummary(data, options) {
  const indent = options.indent || '';
  const enableColors = options.enableColors || false;
  
  return `
${indent}Smoke Test Results
${indent}==================
${indent}
${indent}Checks................: ${data.metrics.checks.values.passes}/${data.metrics.checks.values.fails + data.metrics.checks.values.passes}
${indent}HTTP Req Duration.....: avg=${data.metrics.http_req_duration.values.avg.toFixed(2)}ms p(95)=${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms
${indent}HTTP Req Failed.......: ${(data.metrics.http_req_failed.values.rate * 100).toFixed(2)}%
${indent}Iterations............: ${data.metrics.iterations.values.count}
${indent}VUs...................: ${data.metrics.vus.values.value}
  `;
}
