const nextJest = require('next/jest')

// next/jest loads next.config.js (which validates required env vars) before any
// setup file runs, so provide a default here for the test environment.
process.env.NEXT_PUBLIC_API_URL =
  process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001'

const createJestConfig = nextJest({
  // Provide the path to your Next.js app to load next.config.js and .env files in your test environment
  dir: './',
})

// Add any custom config to be passed to Jest
const customJestConfig = {
  setupFilesAfterEnv: ['<rootDir>/jest.setup.js'],
  testEnvironment: 'jest-environment-jsdom',
  moduleNameMapper: {
    '^@/(.*)$': '<rootDir>/src/$1',
  },
  collectCoverageFrom: [
    'src/**/*.{js,jsx,ts,tsx}',
    '!src/**/*.d.ts',
    '!src/**/*.stories.{js,jsx,ts,tsx}',
    '!src/**/__tests__/**',
    // Framework entry/infra covered by the build + Playwright e2e, not unit tests:
    '!src/app/**', // RootLayout / page entry (async server components)
    '!src/proxy.ts', // edge middleware (runs in the edge runtime)
    // App UI surface — validated by component tests + Playwright e2e (e2e/*.spec.ts)
    // + visual regression, rather than jsdom unit line-coverage:
    '!src/components/app/**',
    '!src/components/ui/**',
    '!src/components/markets-browse/**',
    '!src/components/market-detail/**',
    '!src/components/market-create/**',
    '!src/components/portfolio/**',
    '!src/components/market-resolve/**',
    // Wallet integration depends on the Freighter browser extension (e2e-tested):
    '!src/lib/wallet/**',
  ],
  coverageThreshold: {
    global: {
      branches: 80,
      functions: 80,
      lines: 80,
      statements: 80,
    },
  },
  testMatch: [
    '**/__tests__/**/*.[jt]s?(x)',
    '**/?(*.)+(spec|test).[jt]s?(x)',
  ],
  // e2e/*.spec.ts are Playwright tests, run via `npm run test:e2e` (not Jest).
  testPathIgnorePatterns: ['/node_modules/', '/.next/', '<rootDir>/e2e/'],
  transformIgnorePatterns: [
    '/node_modules/',
    '^.+\\.module\\.(css|sass|scss)$',
  ],
}

// createJestConfig is exported this way to ensure that next/jest can load the Next.js config which is async
module.exports = createJestConfig(customJestConfig)
