// k6 Configuration
export const config = {
  // API endpoint
  baseURL: __ENV.API_URL || 'http://localhost:8080',
  
  // Performance thresholds
  thresholds: {
    // HTTP errors should be less than 0.1%
    http_req_failed: ['rate<0.001'],
    
    // 95% of requests should be below 200ms
    http_req_duration: ['p(95)<200', 'p(99)<500'],
    
    // Specific endpoint thresholds
    'http_req_duration{endpoint:health}': ['p(95)<50'],
    'http_req_duration{endpoint:markets}': ['p(95)<200'],
    'http_req_duration{endpoint:bets}': ['p(95)<250'],
  },
  
  // Test scenarios
  scenarios: {
    smoke: {
      executor: 'constant-vus',
      vus: 1,
      duration: '1m',
    },
    load: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '2m', target: 100 },  // Ramp up to 100 users
        { duration: '5m', target: 100 },  // Stay at 100 users
        { duration: '2m', target: 0 },    // Ramp down
      ],
    },
    stress: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '2m', target: 100 },
        { duration: '5m', target: 100 },
        { duration: '2m', target: 200 },
        { duration: '5m', target: 200 },
        { duration: '2m', target: 300 },
        { duration: '5m', target: 300 },
        { duration: '10m', target: 0 },
      ],
    },
    peak: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '5m', target: 1000 },  // Ramp up to peak
        { duration: '10m', target: 1000 }, // Sustain peak
        { duration: '5m', target: 0 },     // Ramp down
      ],
    },
  },
};

// Helper function to add tags to requests
export function tagRequest(name, endpoint) {
  return {
    tags: {
      name: name,
      endpoint: endpoint,
    },
  };
}
