# End-to-End Tests

Playwright tests for the PredictIQ frontend. All specs live in this directory and
are executed against a running Next.js dev server (or a remote URL via `BASE_URL`).

## Running tests

```bash
# All tests (headless)
npm run test:e2e

# Mobile projects only (Pixel 5 + iPhone 12)
npm run test:e2e:mobile

# Interactive UI
npm run test:e2e:ui
```

## Playwright projects

| Project | Device |
|---|---|
| `chromium` | Desktop Chrome |
| `firefox` | Desktop Firefox |
| `webkit` | Desktop Safari |
| `mobile-chrome` | Pixel 5 |
| `mobile-safari` | iPhone 12 |
| `tablet` | iPad Pro |

## Spec files

| File | Coverage |
|---|---|
| `accessibility.spec.ts` | ARIA landmarks, keyboard navigation, focus indicators, skip links |
| `interactions.spec.ts` | Form submissions, validation error messages, CTA buttons, navigation, scroll |
| `mobile.spec.ts` | Mobile/tablet viewports, responsive breakpoints, touch gestures |
| `market-creation.spec.ts` | Market creation flow (runs against staging via `STAGING_URL`) |
| `performance.spec.ts` | Core Web Vitals, page load timing |
| `user-journeys.spec.ts` | Full user journeys end-to-end |
| `visual-regression.spec.ts` | Screenshot comparison |

---

## Touch Gesture Tests

Touch gesture tests are in `mobile.spec.ts` under the **Touch Gesture** describe
blocks. They run automatically on both the `mobile-chrome` (Pixel 5) and
`mobile-safari` (iPhone 12) Playwright projects.

### What is tested

| Describe block | Gestures |
|---|---|
| `Touch Gesture – Swipe Navigation` | Swipe-up to scroll page; drag via mouse emulation to reach next section |
| `Touch Gesture – Tap to Select Outcome` | Tap CTA button, tap nav link, tap email input, double-tap heading |
| `Touch Gesture – Long Press` | 800 ms hold followed by a tap to confirm navigation still works |

### How gestures are simulated

Playwright does not expose a native touch-swipe API. We use two complementary
approaches depending on what the test needs:

1. **`element.tap()`** — Playwright's first-class tap API. Uses pointer events and
   correctly targets the element. Prefer this for simple tap interactions.

2. **`page.touchscreen.tap(x, y)`** — Raw touchscreen coordinates. Use when the
   element bounding box is needed for precise placement (e.g. double-tap).

3. **`page.mouse.move / down / up`** — Mouse drag emulation. Used for swipe
   gestures because `touchscreen.tap` does not support dragging. The result
   exercises scroll handlers that listen to pointer/mouse events.

4. **Dispatching `TouchEvent` via `page.evaluate`** — Used to exercise listeners
   that explicitly require `touchstart` / `touchmove` / `touchend` events. Note
   that the Web Platform requires a secure context (HTTPS or localhost) for
   `TouchEvent` constructor; tests run against `http://localhost:3000` which
   satisfies this.

### Adding new gesture tests

1. Pick the appropriate describe block or create a new one inside the
   **Touch Gesture** section at the bottom of `mobile.spec.ts`.
2. Use `test.use({ viewport: { width: 375, height: 667 } })` inside the describe
   block so the test runs at a realistic mobile size on all projects.
3. Prefer `element.tap()` over raw coordinates. Fall back to `touchscreen.tap` or
   mouse drag only when tap is insufficient.
4. Document the new gesture in the table above.
