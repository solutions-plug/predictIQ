import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';

const errorRate = new Rate('errors');
const marketLoadTime = new Trend('market_load_time');
const betPlacementTime = new Trend('bet_placement_time');
const apiCalls = new Counter('api_calls');

export const options = {
  stages: [
    { duration: '2m', target: 100 },  // Ramp up to 100 users
    { duration: '5m', target: 100 },  // Stay at 100 users
    { duration: '2m', target: 0 },    // Ramp down
  ],
  thresholds: {
    errors: ['rate<0.001'],
    http_req_duration: ['p(95)<200', 'p(99)<500'],
    'http_req_duration{endpoint:health}': ['p(95)<50'],
    'http_req_duration{endpoint:markets}': ['p(95)<200'],
    market_load_time: ['p(95)<200'],
    bet_placement_time: ['p(95)<250'],
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  // Simulate user behavior patterns
  const scenario = randomIntBetween(1, 100);
  
  if (scenario <= 40) {
    // 40% - Browse markets
    browseMarkets();
  } else if (scenario <= 70) {
    // 30% - View specific market
    viewMarket();
  } else if (scenario <= 85) {
    // 15% - Check user stats
    checkUserStats();
  } else {
    // 15% - Place bet (write operation)
    placeBet();
  }
  
  sleep(randomIntBetween(1, 3));
}

function browseMarkets() {
  const res = http.get(`${BASE_URL}/api/v1/markets`, {
    tags: { endpoint: 'markets', operation: 'list' },
  });
  
  apiCalls.add(1);
  marketLoadTime.add(res.timings.duration);
  
  check(res, {
    'markets list status is 200': (r) => r.status === 200,
    'markets list response time < 200ms': (r) => r.timings.duration < 200,
  }) || errorRate.add(1);
}

function viewMarket() {
  const marketId = randomIntBetween(1, 100);
  const res = http.get(`${BASE_URL}/api/v1/markets/${marketId}`, {
    tags: { endpoint: 'markets', operation: 'get' },
  });
  
  apiCalls.add(1);
  marketLoadTime.add(res.timings.duration);
  
  check(res, {
    'market detail status is 200 or 404': (r) => r.status === 200 || r.status === 404,
    'market detail response time < 200ms': (r) => r.timings.duration < 200,
  }) || errorRate.add(1);
}

function checkUserStats() {
  const userId = `user_${randomIntBetween(1, 1000)}`;
  const res = http.get(`${BASE_URL}/api/v1/users/${userId}/stats`, {
    tags: { endpoint: 'users', operation: 'stats' },
  });
  
  apiCalls.add(1);
  
  check(res, {
    'user stats response time < 200ms': (r) => r.timings.duration < 200,
  }) || errorRate.add(1);
}

function placeBet() {
  const payload = JSON.stringify({
    market_id: randomIntBetween(1, 100),
    outcome: randomIntBetween(0, 1),
    amount: randomIntBetween(10, 1000),
  });
  
  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
    tags: { endpoint: 'bets', operation: 'create' },
  };
  
  const res = http.post(`${BASE_URL}/api/v1/bets`, payload, params);
  
  apiCalls.add(1);
  betPlacementTime.add(res.timings.duration);
  
  check(res, {
    'bet placement response time < 250ms': (r) => r.timings.duration < 250,
  }) || errorRate.add(1);
}

export function handleSummary(data) {
  return {
    'backend/reports/load-test-summary.json': JSON.stringify(data),
    'backend/reports/load-test-summary.html': htmlReport(data),
  };
}

function htmlReport(data) {
  const metrics = data.metrics;
  return `
<!DOCTYPE html>
<html>
<head>
  <title>Load Test Report</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 20px; }
    h1 { color: #333; }
    table { border-collapse: collapse; width: 100%; margin: 20px 0; }
    th, td { border: 1px solid #ddd; padding: 12px; text-align: left; }
    th { background-color: #4CAF50; color: white; }
    .pass { color: green; }
    .fail { color: red; }
  </style>
</head>
<body>
  <h1>Load Test Report - ${new Date().toISOString()}</h1>
  
  <h2>Summary</h2>
  <table>
    <tr><th>Metric</th><th>Value</th></tr>
    <tr><td>Total Requests</td><td>${metrics.http_reqs.values.count}</td></tr>
    <tr><td>Failed Requests</td><td class="${metrics.http_req_failed.values.rate < 0.001 ? 'pass' : 'fail'}">${(metrics.http_req_failed.values.rate * 100).toFixed(2)}%</td></tr>
    <tr><td>Avg Response Time</td><td>${metrics.http_req_duration.values.avg.toFixed(2)}ms</td></tr>
    <tr><td>P95 Response Time</td><td class="${metrics.http_req_duration.values['p(95)'] < 200 ? 'pass' : 'fail'}">${metrics.http_req_duration.values['p(95)'].toFixed(2)}ms</td></tr>
    <tr><td>P99 Response Time</td><td class="${metrics.http_req_duration.values['p(99)'] < 500 ? 'pass' : 'fail'}">${metrics.http_req_duration.values['p(99)'].toFixed(2)}ms</td></tr>
    <tr><td>Throughput</td><td>${metrics.http_reqs.values.rate.toFixed(2)} req/s</td></tr>
  </table>
  
  <h2>Performance Targets</h2>
  <table>
    <tr><th>Target</th><th>Expected</th><th>Actual</th><th>Status</th></tr>
    <tr>
      <td>P95 Response Time</td>
      <td>&lt; 200ms</td>
      <td>${metrics.http_req_duration.values['p(95)'].toFixed(2)}ms</td>
      <td class="${metrics.http_req_duration.values['p(95)'] < 200 ? 'pass' : 'fail'}">${metrics.http_req_duration.values['p(95)'] < 200 ? 'PASS' : 'FAIL'}</td>
    </tr>
    <tr>
      <td>Error Rate</td>
      <td>&lt; 0.1%</td>
      <td>${(metrics.http_req_failed.values.rate * 100).toFixed(2)}%</td>
      <td class="${metrics.http_req_failed.values.rate < 0.001 ? 'pass' : 'fail'}">${metrics.http_req_failed.values.rate < 0.001 ? 'PASS' : 'FAIL'}</td>
    </tr>
  </table>
</body>
</html>
  `;
}
