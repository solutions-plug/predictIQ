# End-to-End Testing Guide

## Overview

This document provides comprehensive guidance for running and maintaining E2E tests for the PredictIQ landing page using Playwright.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Test Structure](#test-structure)
3. [Running Tests](#running-tests)
4. [Writing Tests](#writing-tests)
5. [CI/CD Integration](#cicd-integration)
6. [Debugging](#debugging)
7. [Best Practices](#best-practices)
8. [Troubleshooting](#troubleshooting)

## Quick Start

### Installation

```bash
cd frontend

# Install dependencies
npm install

# Install Playwright browsers
npm run playwright:install
```

### Run Tests

```bash
# Run all E2E tests
npm run test:e2e

# Run with UI mode (recommended for development)
npm run test:e2e:ui

# Run in headed mode (see browser)
npm run test:e2e:headed

# Run specific browser
npm run test:e2e:chrome
npm run test:e2e:firefox
npm run test:e2e:safari

# Run mobile tests
npm run test:e2e:mobile

# Debug mode
npm run test:e2e:debug
```

## Test Structure

```
frontend/
├── e2e/
│   ├── user-journeys.spec.ts      # User journey tests
│   ├── interactions.spec.ts       # Form and interaction tests
│   ├── mobile.spec.ts             # Mobile and responsive tests
│   ├── performance.spec.ts        # Performance metrics tests
│   ├── visual-regression.spec.ts  # Screenshot comparison tests
│   ├── accessibility.spec.ts      # A11y compliance tests
│   └── helpers.ts                 # Test utilities
├── playwright.config.ts           # Playwright configuration
└── playwright-report/             # Test reports (generated)
```

## Test Coverage

### User Journeys

1. **Homepage → Features → Newsletter Signup**
   - Browse features
   - Submit newsletter form
   - Verify success state

2. **Homepage → View Markets → Launch App**
   - Navigate to How It Works
   - View market creation steps
   - Click external links

3. **Homepage → FAQ → Contact**
   - Navigate to About section
   - Access footer links
   - Verify contact information

4. **Mobile Navigation Flow**
   - Test mobile layout
   - Touch interactions
   - Mobile form submission

### Interactions

- Form submissions (valid/invalid)
- CTA button clicks
- Navigation between sections
- Scroll behavior
- External link clicks

### Mobile & Responsive

- Multiple viewport sizes (320px - 1920px)
- Touch target sizes (WCAG 2.5.5)
- Mobile keyboard handling
- Landscape orientation
- Zoom support (200% - 400%)

### Performance

- Page load time (< 3s)
- Core Web Vitals (FCP, LCP, CLS)
- Time to Interactive (< 5s)
- Resource loading
- Network conditions (slow 3G, offline)

### Visual Regression

- Homepage screenshots
- Form states (initial, error, success, focused)
- Mobile layouts
- Tablet layouts
- Hover states
- Dark mode
- High contrast mode
- Reduced motion

### Accessibility

- Keyboard navigation
- Screen reader support
- Focus indicators
- Skip links
- Form accessibility
- Image alt text
- Color contrast
- ARIA attributes

## Running Tests

### Development

```bash
# Interactive UI mode (best for development)
npm run test:e2e:ui

# Watch mode with headed browser
npm run test:e2e:headed

# Debug specific test
npm run test:e2e:debug -- user-journeys.spec.ts
```

### CI/CD

```bash
# Run all tests in CI mode
npm run test:e2e:ci

# View HTML report
npm run test:e2e:report
```

### Specific Test Suites

```bash
# Run specific file
npx playwright test e2e/user-journeys.spec.ts

# Run specific test
npx playwright test -g "should complete full journey"

# Run specific browser
npx playwright test --project=chromium

# Run mobile only
npx playwright test --project=mobile-chrome --project=mobile-safari
```

## Writing Tests

### Basic Test Structure

```typescript
import { test, expect } from '@playwright/test';

test.describe('Feature Name', () => {
  test('should do something', async ({ page }) => {
    await page.goto('/');
    
    // Interact with page
    await page.getByRole('button', { name: /click me/i }).click();
    
    // Assert result
    await expect(page.getByText(/success/i)).toBeVisible();
  });
});
```

### Using Helpers

```typescript
import { fillAndSubmitNewsletterForm, verifyResponsiveLayout } from './helpers';

test('should submit form', async ({ page }) => {
  await page.goto('/');
  await fillAndSubmitNewsletterForm(page, 'test@example.com');
  await expect(page.getByRole('button', { name: /subscribed/i })).toBeVisible();
});
```

### Mobile Tests

```typescript
test.describe('Mobile Tests', () => {
  test.use({ viewport: { width: 375, height: 667 } });
  
  test('should work on mobile', async ({ page }) => {
    await page.goto('/');
    // Test mobile-specific behavior
  });
});
```

### Visual Regression

```typescript
test('should match screenshot', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveScreenshot('homepage.png', {
    fullPage: true,
    animations: 'disabled',
  });
});
```

## CI/CD Integration

### GitHub Actions

Tests run automatically on:
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop`

### Test Matrix

- **Browsers**: Chrome, Firefox, Safari
- **Devices**: Desktop, Mobile (iOS/Android), Tablet
- **Test Types**: Functional, Visual, Performance, Accessibility

### Artifacts

- HTML reports (30 days retention)
- Screenshots (7 days retention)
- Videos of failed tests (7 days retention)

### Viewing Results

1. Go to GitHub Actions tab
2. Select workflow run
3. Download artifacts
4. Open `playwright-report/index.html`

## Debugging

### Debug Mode

```bash
# Open Playwright Inspector
npm run test:e2e:debug

# Debug specific test
npx playwright test --debug -g "test name"
```

### Headed Mode

```bash
# See browser while tests run
npm run test:e2e:headed
```

### Screenshots and Videos

```bash
# Screenshots are taken on failure automatically
# Videos are recorded for failed tests

# View in test-results/ directory
ls test-results/
```

### Console Logs

```typescript
test('should log console', async ({ page }) => {
  page.on('console', msg => console.log(msg.text()));
  await page.goto('/');
});
```

## Best Practices

### 1. Use Semantic Selectors

```typescript
// ✅ Good - semantic and resilient
await page.getByRole('button', { name: /submit/i });
await page.getByLabel(/email address/i);

// ❌ Bad - brittle
await page.locator('.btn-primary');
await page.locator('#email-input');
```

### 2. Wait for Elements

```typescript
// ✅ Good - explicit wait
await expect(page.getByText(/success/i)).toBeVisible();

// ❌ Bad - implicit wait
await page.waitForTimeout(1000);
```

### 3. Test User Behavior

```typescript
// ✅ Good - tests actual user flow
await page.getByLabel(/email/i).fill('test@example.com');
await page.getByRole('button', { name: /submit/i }).click();
await expect(page.getByText(/success/i)).toBeVisible();

// ❌ Bad - tests implementation
await page.evaluate(() => submitForm());
```

### 4. Keep Tests Independent

```typescript
// ✅ Good - each test is independent
test('test 1', async ({ page }) => {
  await page.goto('/');
  // Test logic
});

test('test 2', async ({ page }) => {
  await page.goto('/');
  // Test logic
});
```

### 5. Use Test Fixtures

```typescript
// ✅ Good - reusable setup
test.beforeEach(async ({ page }) => {
  await page.goto('/');
});

test('test 1', async ({ page }) => {
  // Test logic
});
```

## Troubleshooting

### Tests Failing Locally

1. **Update browsers**
   ```bash
   npm run playwright:install
   ```

2. **Clear cache**
   ```bash
   rm -rf node_modules/.cache
   ```

3. **Check Node version**
   ```bash
   node --version  # Should be >= 18
   ```

### Flaky Tests

1. **Add explicit waits**
   ```typescript
   await expect(element).toBeVisible();
   ```

2. **Disable animations**
   ```typescript
   await page.emulateMedia({ reducedMotion: 'reduce' });
   ```

3. **Increase timeout**
   ```typescript
   test('slow test', async ({ page }) => {
     test.setTimeout(60000);
     // Test logic
   });
   ```

### Visual Regression Failures

1. **Update baseline**
   ```bash
   npx playwright test --update-snapshots
   ```

2. **Review diff**
   - Check `test-results/` for diff images
   - Verify changes are intentional

3. **Platform differences**
   - Screenshots may differ between OS
   - Use Docker for consistent results

### CI/CD Failures

1. **Check logs**
   - View GitHub Actions logs
   - Download artifacts

2. **Run locally with CI flag**
   ```bash
   CI=true npm run test:e2e
   ```

3. **Check resource limits**
   - Increase timeout in workflow
   - Reduce parallel workers

## Performance Targets

- **Page Load**: < 3 seconds
- **Time to Interactive**: < 5 seconds
- **First Contentful Paint**: < 1.5 seconds
- **Largest Contentful Paint**: < 2.5 seconds
- **Cumulative Layout Shift**: < 0.1
- **Flaky Test Rate**: < 5%

## Accessibility Standards

- **WCAG 2.1 Level AA** compliance
- **Keyboard navigation** fully supported
- **Screen reader** compatible
- **Touch targets** ≥ 44x44 pixels
- **Color contrast** ≥ 4.5:1 for text
- **Zoom support** up to 400%

## Reporting

### HTML Report

```bash
npm run test:e2e:report
```

### JSON Report

```javascript
// Available at: playwright-report/results.json
const results = require('./playwright-report/results.json');
console.log(`Total tests: ${results.suites.length}`);
```

### JUnit Report

```xml
<!-- Available at: playwright-report/results.xml -->
<!-- Compatible with CI/CD systems -->
```

## Resources

- [Playwright Documentation](https://playwright.dev)
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [Web Vitals](https://web.dev/vitals/)
- [Testing Best Practices](https://playwright.dev/docs/best-practices)

## Support

For issues or questions:
1. Check this documentation
2. Review [Playwright docs](https://playwright.dev)
3. Open an issue on GitHub
4. Contact the development team
