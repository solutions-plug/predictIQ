# E2E Tests

End-to-end tests for the PredictIQ landing page using Playwright.

## Quick Start

```bash
# Install dependencies
npm install

# Install Playwright browsers
npm run playwright:install

# Run tests
npm run test:e2e

# Run with UI (recommended)
npm run test:e2e:ui
```

## Test Suites

- **user-journeys.spec.ts** - Critical user paths and conversions
- **interactions.spec.ts** - Forms, CTAs, navigation, and links
- **mobile.spec.ts** - Mobile devices and responsive breakpoints
- **performance.spec.ts** - Load times and Core Web Vitals
- **visual-regression.spec.ts** - Screenshot comparison tests
- **accessibility.spec.ts** - WCAG 2.1 AA compliance

## Commands

```bash
# Development
npm run test:e2e:ui          # Interactive UI mode
npm run test:e2e:headed      # See browser
npm run test:e2e:debug       # Debug mode

# Specific browsers
npm run test:e2e:chrome      # Chrome only
npm run test:e2e:firefox     # Firefox only
npm run test:e2e:safari      # Safari only
npm run test:e2e:mobile      # Mobile devices

# CI/CD
npm run test:e2e:ci          # Run in CI mode
npm run test:e2e:report      # View HTML report
```

## Coverage

✅ Homepage → Features → Newsletter signup  
✅ Homepage → Markets → Launch app  
✅ Homepage → FAQ → Contact  
✅ Mobile navigation flow  
✅ Form submissions (valid/invalid)  
✅ CTA button interactions  
✅ Navigation between sections  
✅ Scroll behavior  
✅ External link clicks  
✅ Responsive breakpoints (320px - 1920px)  
✅ Cross-browser (Chrome, Firefox, Safari)  
✅ Mobile devices (iOS, Android)  
✅ Performance metrics  
✅ Visual regression  
✅ Accessibility compliance  

## Documentation

See [E2E_TESTING_GUIDE.md](../E2E_TESTING_GUIDE.md) for comprehensive documentation.

## CI/CD

Tests run automatically on:
- Push to `main` or `develop`
- Pull requests

Results available in GitHub Actions artifacts.
