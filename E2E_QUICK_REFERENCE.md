# E2E Testing Quick Reference

## Installation

```bash
cd frontend
npm install
npm run playwright:install
```

## Common Commands

```bash
# Development
npm run test:e2e:ui          # Interactive UI (best for dev)
npm run test:e2e:headed      # See browser
npm run test:e2e:debug       # Debug mode

# Run tests
npm run test:e2e             # All tests
npm run test:e2e:chrome      # Chrome only
npm run test:e2e:firefox     # Firefox only
npm run test:e2e:safari      # Safari only
npm run test:e2e:mobile      # Mobile devices

# CI/CD
npm run test:e2e:ci          # CI mode
npm run test:e2e:report      # View report
```

## Test Files

| File | Purpose |
|------|---------|
| `user-journeys.spec.ts` | Critical user paths |
| `interactions.spec.ts` | Forms, CTAs, navigation |
| `mobile.spec.ts` | Mobile & responsive |
| `performance.spec.ts` | Load times & metrics |
| `visual-regression.spec.ts` | Screenshots |
| `accessibility.spec.ts` | WCAG compliance |

## Run Specific Tests

```bash
# Specific file
npx playwright test e2e/user-journeys.spec.ts

# Specific test
npx playwright test -g "should complete full journey"

# Specific browser
npx playwright test --project=chromium
```

## Debugging

```bash
# Debug mode
npm run test:e2e:debug

# Headed mode
npm run test:e2e:headed

# View trace
npx playwright show-trace trace.zip
```

## Update Screenshots

```bash
# Update all baselines
npx playwright test --update-snapshots

# Update specific test
npx playwright test visual-regression.spec.ts --update-snapshots
```

## CI/CD

Tests run automatically on:
- Push to `main` or `develop`
- Pull requests

View results in GitHub Actions → Artifacts

## Coverage

✅ User journeys (4 paths)  
✅ Form submissions  
✅ CTA interactions  
✅ Navigation  
✅ Mobile (3 devices)  
✅ Responsive (7 breakpoints)  
✅ Cross-browser (3 browsers)  
✅ Performance metrics  
✅ Visual regression  
✅ Accessibility (WCAG 2.1 AA)  

## Performance Targets

- Page Load: < 3s
- Time to Interactive: < 5s
- FCP: < 1.5s
- LCP: < 2.5s
- CLS: < 0.1

## Documentation

- **Full Guide:** `E2E_TESTING_GUIDE.md`
- **E2E README:** `e2e/README.md`
- **Implementation:** `IMPLEMENTATION_SUMMARY_ISSUE_92.md`

## Troubleshooting

### Tests failing?

```bash
# Update browsers
npm run playwright:install

# Clear cache
rm -rf node_modules/.cache

# Check Node version
node --version  # Should be >= 18
```

### Flaky tests?

- Add explicit waits: `await expect(element).toBeVisible()`
- Disable animations: `animations: 'disabled'`
- Increase timeout: `test.setTimeout(60000)`

### Visual regression diff?

```bash
# Update baseline
npx playwright test --update-snapshots

# Review diff in test-results/
```

## Helper Functions

```typescript
import {
  fillAndSubmitNewsletterForm,
  verifyResponsiveLayout,
  getCoreWebVitals,
  navigateToSection,
} from './helpers';

// Use in tests
await fillAndSubmitNewsletterForm(page, 'test@example.com');
await verifyResponsiveLayout(page, { width: 375, height: 667 });
```

## Best Practices

✅ Use semantic selectors (`getByRole`, `getByLabel`)  
✅ Explicit waits with assertions  
✅ Test user behavior, not implementation  
✅ Keep tests independent  
✅ Disable animations for stability  
✅ Capture screenshots on failure  

## Support

1. Check `E2E_TESTING_GUIDE.md`
2. Review [Playwright docs](https://playwright.dev)
3. Open GitHub issue
4. Contact dev team
