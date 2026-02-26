#!/usr/bin/env node

/**
 * CI/CD Test Runner for E2E Tests
 * Runs Playwright tests with proper configuration for CI environments
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const CI = process.env.CI === 'true';
const REPORT_DIR = path.join(__dirname, '..', 'playwright-report');

console.log('🚀 Starting E2E Test Suite...\n');

// Ensure report directory exists
if (!fs.existsSync(REPORT_DIR)) {
  fs.mkdirSync(REPORT_DIR, { recursive: true });
}

// Configuration
const config = {
  workers: CI ? 1 : undefined,
  retries: CI ? 2 : 0,
  reporter: CI ? 'github' : 'list',
};

console.log('Configuration:', config);
console.log('CI Mode:', CI ? 'Yes' : 'No');
console.log('');

try {
  // Run Playwright tests
  const command = `npx playwright test ${CI ? '--reporter=github,html,json,junit' : ''}`;
  
  console.log(`Running: ${command}\n`);
  
  execSync(command, {
    stdio: 'inherit',
    env: {
      ...process.env,
      CI: CI ? 'true' : 'false',
    },
  });
  
  console.log('\n✅ All E2E tests passed!');
  
  // Generate summary
  if (fs.existsSync(path.join(REPORT_DIR, 'results.json'))) {
    const results = JSON.parse(
      fs.readFileSync(path.join(REPORT_DIR, 'results.json'), 'utf-8')
    );
    
    console.log('\n📊 Test Summary:');
    console.log(`   Total Suites: ${results.suites?.length || 0}`);
    console.log(`   Report: ${REPORT_DIR}/index.html`);
  }
  
  process.exit(0);
} catch (error) {
  console.error('\n❌ E2E tests failed!');
  console.error('Check the report at:', path.join(REPORT_DIR, 'index.html'));
  process.exit(1);
}
