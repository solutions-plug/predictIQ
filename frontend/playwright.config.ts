import { defineConfig, devices } from '@playwright/test';

// Flaky test detection: run each test this many times when FLAKY_DETECTION=true.
// A test is considered flaky if it passes on some runs and fails on others.
const flakyDetectionRuns = process.env.FLAKY_DETECTION ? 3 : 1;

// Specs that use page.route() mocks and need no live backend.
// These run on every PR in CI.
const MOCKED_SPECS = [
  'e2e/user-journeys.spec.ts',
  'e2e/market-creation.spec.ts',
  'e2e/accessibility.spec.ts',
  'e2e/interactions.spec.ts',
  'e2e/mobile.spec.ts',
  'e2e/performance.spec.ts',
  'e2e/visual-regression.spec.ts',
];

// Specs that require a real backend. Only run on merge to main via the
// e2e-staging workflow.
const INTEGRATION_SPECS = 'e2e/integration/**/*.spec.ts';

const isStaging = !!process.env.STAGING_URL;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  // Repeat each test to surface flaky behaviour
  repeatEach: flakyDetectionRuns,
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['json', { outputFile: 'playwright-report/results.json' }],
    ['junit', { outputFile: 'playwright-report/results.xml' }],
    ['list']
  ],
  use: {
    baseURL: process.env.BASE_URL || 'http://localhost:3000',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    // ------------------------------------------------------------------
    // Mocked E2E — no live backend required. Runs on every PR.
    // ------------------------------------------------------------------
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
      testMatch: MOCKED_SPECS,
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
      testMatch: MOCKED_SPECS,
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
      testMatch: MOCKED_SPECS,
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'mobile-chrome',
      use: { ...devices['Pixel 5'] },
      testMatch: MOCKED_SPECS,
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 12'] },
      testMatch: MOCKED_SPECS,
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'tablet',
      use: { ...devices['iPad Pro'] },
      testMatch: MOCKED_SPECS,
      testIgnore: isStaging ? '**' : undefined,
    },

    // ------------------------------------------------------------------
    // Integration E2E — requires a real backend. Runs on merge to main.
    // ------------------------------------------------------------------
    {
      name: 'integration-e2e',
      use: { ...devices['Desktop Chrome'] },
      testMatch: INTEGRATION_SPECS,
      testIgnore: isStaging ? undefined : '**',
    },

    // ------------------------------------------------------------------
    // Staging project — activated when STAGING_URL is set.
    // Runs against a real API; no local web server is started.
    // ------------------------------------------------------------------
    {
      name: 'staging',
      use: {
        ...devices['Desktop Chrome'],
        baseURL: process.env.STAGING_URL,
      },
      testIgnore: isStaging ? undefined : '**',
    },
  ],
  webServer: process.env.BASE_URL
    ? undefined  // skip local dev server when targeting a remote URL (e.g. staging)
    : {
        command: 'npm run dev',
        url: 'http://localhost:3000',
        reuseExistingServer: !process.env.CI,
        timeout: 120000,
      },
});
