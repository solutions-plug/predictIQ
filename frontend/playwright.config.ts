import { defineConfig, devices } from '@playwright/test';

// Flaky test detection: run each test this many times when FLAKY_DETECTION=true.
// A test is considered flaky if it passes on some runs and fails on others.
const flakyDetectionRuns = process.env.FLAKY_DETECTION ? 3 : 1;

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
    // Local / PR projects (default)
    // ------------------------------------------------------------------
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'mobile-chrome',
      use: { ...devices['Pixel 5'] },
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 12'] },
      testIgnore: isStaging ? '**' : undefined,
    },
    {
      name: 'tablet',
      use: { ...devices['iPad Pro'] },
      testIgnore: isStaging ? '**' : undefined,
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
    // Staging project: runs only the market-creation spec against the staging URL.
    // Activated when BASE_URL points to staging (or STAGING=true).
    {
      name: 'staging',
      testMatch: '**/market-creation.spec.ts',
      use: {
        ...devices['Desktop Chrome'],
        baseURL: process.env.STAGING_URL || process.env.BASE_URL || 'http://localhost:3000',
      },
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
