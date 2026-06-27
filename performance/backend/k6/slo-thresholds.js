/**
 * SLO Thresholds Configuration
 * 
 * Centralized SLO thresholds derived from performance/config/slo.json
 * Used by all k6 tests to ensure consistent SLO enforcement
 */

export const sloThresholds = {
  // API Availability: 99.9% target
  'http_req_failed': ['rate<0.001'],  // 0.1% error rate
  
  // API Latency P95: 200ms target
  'http_req_duration': ['p(95)<200', 'p(99)<500'],
  
  // Endpoint-specific thresholds
  'http_req_duration{endpoint:health}': ['p(95)<50'],
  'http_req_duration{endpoint:markets}': ['p(95)<200'],
  'http_req_duration{endpoint:bets}': ['p(95)<250'],
  'http_req_duration{endpoint:users}': ['p(95)<200'],
  
  // Cache availability: 99.95% target
  'cache_hit_rate': ['rate>0.8'],
};

/**
 * Get thresholds for a specific test
 * @param {string} testName - Name of the test (e.g., 'smoke', 'load', 'cache')
 * @returns {Object} Thresholds configuration
 */
export function getThresholdsForTest(testName) {
  const baseThresholds = {
    'http_req_failed': ['rate<0.001'],
    'http_req_duration': ['p(95)<200', 'p(99)<500'],
  };

  switch (testName) {
    case 'smoke':
      return {
        ...baseThresholds,
        'http_req_duration': ['p(95)<200'],
      };
    case 'load':
      return {
        ...baseThresholds,
        'http_req_duration{endpoint:health}': ['p(95)<50'],
        'http_req_duration{endpoint:markets}': ['p(95)<200'],
        'http_req_duration{endpoint:bets}': ['p(95)<250'],
      };
    case 'cache':
      return {
        'cache_hit_rate': ['rate>0.8'],
        'http_req_duration': ['p(95)<100'],
      };
    case 'stress':
      return {
        ...baseThresholds,
        'http_req_duration': ['p(95)<300'],  // Relaxed for stress test
      };
    case 'spike':
      return {
        ...baseThresholds,
        'http_req_duration': ['p(95)<300'],  // Relaxed for spike test
      };
    case 'rate-limit':
      return {
        'http_req_failed': ['rate<0.05'],  // Allow higher error rate for rate limit test
      };
    case 'blockchain':
      return {
        ...baseThresholds,
        'http_req_duration': ['p(95)<500'],  // Blockchain operations may be slower
      };
    default:
      return baseThresholds;
  }
}

/**
 * Check if thresholds were breached and return details
 * @param {Object} data - k6 summary data
 * @returns {Object} Breach details
 */
export function checkThresholdBreaches(data) {
  const breaches = [];
  
  if (data.metrics.http_req_failed && data.metrics.http_req_failed.values.rate > 0.001) {
    breaches.push({
      metric: 'Error Rate',
      threshold: '< 0.1%',
      actual: `${(data.metrics.http_req_failed.values.rate * 100).toFixed(2)}%`,
      severity: 'critical',
    });
  }
  
  if (data.metrics.http_req_duration && data.metrics.http_req_duration.values['p(95)'] > 200) {
    breaches.push({
      metric: 'P95 Response Time',
      threshold: '< 200ms',
      actual: `${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms`,
      severity: 'warning',
    });
  }
  
  if (data.metrics.http_req_duration && data.metrics.http_req_duration.values['p(99)'] > 500) {
    breaches.push({
      metric: 'P99 Response Time',
      threshold: '< 500ms',
      actual: `${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms`,
      severity: 'warning',
    });
  }
  
  return breaches;
}
