import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';

const errorRate = new Rate('tts_errors');
const enqueueDuration = new Trend('tts_enqueue_duration');
const generateDuration = new Trend('tts_generate_duration');
const jobStatusDuration = new Trend('tts_job_status_duration');
const ttsRequests = new Counter('tts_requests');

// Google Cloud TTS API quota limits (as of 2024):
//   Standard voices : 1,000,000 characters/month (free tier)
//   WaveNet voices  : 1,000,000 characters/month (free tier)
//   Neural2 voices  : 1,000,000 characters/month (free tier)
//   Requests/min    : 1,000 requests/minute per project (soft limit)
// See: https://cloud.google.com/text-to-speech/quotas
// Alerts for quota consumption are defined in performance/config/alerts.yaml
// under the `tts_quota` group.

export const options = {
  stages: [
    { duration: '1m', target: 10 },   // Ramp up — TTS is quota-constrained
    { duration: '5m', target: 10 },   // Sustain
    { duration: '1m', target: 20 },   // Probe higher concurrency
    { duration: '5m', target: 20 },
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    // Error rate must stay below 1%
    tts_errors: ['rate<0.01'],
    // p95 latency targets per operation type
    tts_enqueue_duration: ['p(95)<500'],
    tts_generate_duration: ['p(95)<10000'],  // Sync generation includes TTS API call
    tts_job_status_duration: ['p(95)<200'],
    // Overall HTTP thresholds
    http_req_failed: ['rate<0.01'],
    'http_req_duration{endpoint:health}': ['p(95)<100'],
    'http_req_duration{endpoint:enqueue}': ['p(95)<500'],
    'http_req_duration{endpoint:job_status}': ['p(95)<200'],
    'http_req_duration{endpoint:list_jobs}': ['p(95)<300'],
  },
};

const BASE_URL = __ENV.TTS_URL || 'http://localhost:3000';
const API_KEY = __ENV.TTS_API_KEY || '';

const SAMPLE_TEXTS = [
  'The quick brown fox jumps over the lazy dog.',
  'PredictIQ provides real-time market predictions powered by AI.',
  'Welcome to the future of decentralized prediction markets.',
  'Your portfolio has increased by fifteen percent this week.',
  'Market volatility is expected to remain high through the quarter.',
];

const VOICE_IDS = [
  'el-rachel-en',
  'el-domi-en',
  'gc-en-us-standard-a',
  'gc-en-us-wavenet-a',
];

function headers() {
  const h = { 'Content-Type': 'application/json' };
  if (API_KEY) h['Authorization'] = `Bearer ${API_KEY}`;
  return h;
}

function randomText() {
  return SAMPLE_TEXTS[randomIntBetween(0, SAMPLE_TEXTS.length - 1)];
}

function randomVoice() {
  return VOICE_IDS[randomIntBetween(0, VOICE_IDS.length - 1)];
}

export default function () {
  const scenario = randomIntBetween(1, 100);

  if (scenario <= 40) {
    checkHealth();
  } else if (scenario <= 65) {
    enqueueJob();
  } else if (scenario <= 80) {
    listJobs();
  } else {
    pollJobStatus();
  }

  sleep(randomIntBetween(1, 3));
}

function checkHealth() {
  const res = http.get(`${BASE_URL}/health`, {
    headers: headers(),
    tags: { endpoint: 'health' },
  });

  ttsRequests.add(1);

  check(res, {
    'health status 200': (r) => r.status === 200,
    'health response time < 100ms': (r) => r.timings.duration < 100,
  }) || errorRate.add(1);
}

function enqueueJob() {
  const payload = JSON.stringify({
    text: randomText(),
    voiceId: randomVoice(),
  });

  const res = http.post(`${BASE_URL}/tts/enqueue`, payload, {
    headers: headers(),
    tags: { endpoint: 'enqueue' },
  });

  ttsRequests.add(1);
  enqueueDuration.add(res.timings.duration);

  const ok = check(res, {
    'enqueue status 200': (r) => r.status === 200,
    'enqueue returns jobId': (r) => {
      try { return JSON.parse(r.body).jobId !== undefined; } catch { return false; }
    },
    'enqueue response time < 500ms': (r) => r.timings.duration < 500,
  });

  if (!ok) errorRate.add(1);
}

function listJobs() {
  const res = http.get(`${BASE_URL}/tts/jobs`, {
    headers: headers(),
    tags: { endpoint: 'list_jobs' },
  });

  ttsRequests.add(1);

  check(res, {
    'list jobs status 200': (r) => r.status === 200,
    'list jobs returns array': (r) => {
      try { return Array.isArray(JSON.parse(r.body)); } catch { return false; }
    },
    'list jobs response time < 300ms': (r) => r.timings.duration < 300,
  }) || errorRate.add(1);
}

function pollJobStatus() {
  // Enqueue a job first, then poll its status
  const enqueuePayload = JSON.stringify({
    text: randomText(),
    voiceId: randomVoice(),
  });

  const enqueueRes = http.post(`${BASE_URL}/tts/enqueue`, enqueuePayload, {
    headers: headers(),
    tags: { endpoint: 'enqueue' },
  });

  ttsRequests.add(1);
  enqueueDuration.add(enqueueRes.timings.duration);

  if (enqueueRes.status !== 200) {
    errorRate.add(1);
    return;
  }

  let jobId;
  try {
    jobId = JSON.parse(enqueueRes.body).jobId;
  } catch {
    errorRate.add(1);
    return;
  }

  sleep(1);

  const statusRes = http.get(`${BASE_URL}/tts/job/${jobId}`, {
    headers: headers(),
    tags: { endpoint: 'job_status' },
  });

  ttsRequests.add(1);
  jobStatusDuration.add(statusRes.timings.duration);

  check(statusRes, {
    'job status 200 or 404': (r) => r.status === 200 || r.status === 404,
    'job status response time < 200ms': (r) => r.timings.duration < 200,
  }) || errorRate.add(1);
}

export function handleSummary(data) {
  const metrics = data.metrics;

  const p95Enqueue = metrics.tts_enqueue_duration?.values['p(95)'] ?? 0;
  const p95Generate = metrics.tts_generate_duration?.values['p(95)'] ?? 0;
  const p95Status = metrics.tts_job_status_duration?.values['p(95)'] ?? 0;
  const errorRateVal = metrics.tts_errors?.values.rate ?? 0;
  const throughput = metrics.http_reqs?.values.rate ?? 0;

  const report = {
    timestamp: new Date().toISOString(),
    summary: {
      total_requests: metrics.http_reqs?.values.count ?? 0,
      error_rate_pct: (errorRateVal * 100).toFixed(2),
      throughput_rps: throughput.toFixed(2),
    },
    latency: {
      enqueue_p95_ms: p95Enqueue.toFixed(2),
      generate_p95_ms: p95Generate.toFixed(2),
      job_status_p95_ms: p95Status.toFixed(2),
    },
    thresholds_passed: {
      error_rate: errorRateVal < 0.01,
      enqueue_p95: p95Enqueue < 500,
      job_status_p95: p95Status < 200,
    },
    quota_note: 'Google Cloud TTS: 1M chars/month free; 1000 req/min soft limit. Monitor via tts_quota alerts.',
  };

  return {
    'backend/reports/tts-load-test-summary.json': JSON.stringify(report, null, 2),
  };
}
