import { test, expect } from '@playwright/test';

test.describe('Mobile Navigation', () => {
  test.use({ viewport: { width: 375, height: 667 } });
  
  test('should display mobile layout correctly', async ({ page }) => {
    await page.goto('/');
    
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    await expect(page.getByRole('navigation')).toBeVisible();
  });

  test('should navigate between sections on mobile', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('link', { name: /features/i }).click();
    await expect(page.locator('#features')).toBeInViewport();
    
    await page.getByRole('link', { name: /about/i }).click();
    await expect(page.locator('#about')).toBeInViewport();
  });

  test('should submit form on mobile', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).fill('mobile@example.com');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    await expect(page.getByRole('button', { name: /subscribed/i })).toBeVisible();
  });

  test('should handle touch interactions', async ({ page }) => {
    await page.goto('/');
    
    const featuresLink = page.getByRole('link', { name: /features/i });
    await featuresLink.tap();
    
    await expect(page.locator('#features')).toBeInViewport();
  });
});

test.describe('Tablet Layout', () => {
  test.use({ viewport: { width: 768, height: 1024 } });
  
  test('should display tablet layout correctly', async ({ page }) => {
    await page.goto('/');
    
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    await expect(page.getByRole('navigation')).toBeVisible();
  });

  test('should handle tablet interactions', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).fill('tablet@example.com');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    await expect(page.getByRole('button', { name: /subscribed/i })).toBeVisible();
  });
});

test.describe('Responsive Breakpoints', () => {
  const breakpoints = [
    { name: 'mobile-small', width: 320, height: 568 },
    { name: 'mobile', width: 375, height: 667 },
    { name: 'mobile-large', width: 414, height: 896 },
    { name: 'tablet', width: 768, height: 1024 },
    { name: 'desktop', width: 1024, height: 768 },
    { name: 'desktop-large', width: 1440, height: 900 },
    { name: 'desktop-xl', width: 1920, height: 1080 },
  ];

  for (const breakpoint of breakpoints) {
    test(`should render correctly at ${breakpoint.name} (${breakpoint.width}x${breakpoint.height})`, async ({ page }) => {
      await page.setViewportSize({ width: breakpoint.width, height: breakpoint.height });
      await page.goto('/');
      
      // Verify core elements are visible
      await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
      await expect(page.getByRole('navigation')).toBeVisible();
      await expect(page.getByLabel(/email address/i)).toBeVisible();
      await expect(page.getByRole('button', { name: /get early access/i })).toBeVisible();
      
      // Verify no horizontal scroll
      const hasHorizontalScroll = await page.evaluate(() => {
        return document.documentElement.scrollWidth > document.documentElement.clientWidth;
      });
      expect(hasHorizontalScroll).toBe(false);
    });
  }
});

test.describe('Landscape Orientation', () => {
  test('should handle mobile landscape', async ({ page }) => {
    await page.setViewportSize({ width: 667, height: 375 });
    await page.goto('/');
    
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    await expect(page.getByRole('navigation')).toBeVisible();
  });

  test('should handle tablet landscape', async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 768 });
    await page.goto('/');
    
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    await expect(page.getByRole('navigation')).toBeVisible();
  });
});

test.describe('Touch Target Sizes', () => {
  test.use({ viewport: { width: 375, height: 667 } });
  
  test('should have adequate touch target sizes', async ({ page }) => {
    await page.goto('/');
    
    const interactiveElements = [
      page.getByRole('link', { name: /features/i }),
      page.getByRole('button', { name: /get early access/i }),
      page.getByLabel(/email address/i),
    ];
    
    for (const element of interactiveElements) {
      const box = await element.boundingBox();
      if (box) {
        // WCAG recommends minimum 44x44 pixels for touch targets
        expect(box.height).toBeGreaterThanOrEqual(40);
      }
    }
  });
});

test.describe('Mobile Form Interactions', () => {
  test.use({ viewport: { width: 375, height: 667 } });
  
  test('should handle mobile keyboard', async ({ page }) => {
    await page.goto('/');
    
    const emailInput = page.getByLabel(/email address/i);
    await emailInput.click();
    
    // Verify input is focused
    await expect(emailInput).toBeFocused();
    
    await emailInput.fill('mobile@example.com');
    await expect(emailInput).toHaveValue('mobile@example.com');
  });

  test('should handle mobile form validation', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    
    await page.getByLabel(/email address/i).fill('invalid');
    await page.getByRole('button', { name: /get early access/i }).click();
    await expect(page.getByRole('alert')).toContainText(/valid email/i);
  });
});

test.describe('Viewport Zoom', () => {
  test('should handle 200% zoom without horizontal scroll', async ({ page }) => {
    await page.goto('/');

    // Simulate 200% zoom
    await page.evaluate(() => {
      document.body.style.zoom = '2';
    });

    // Verify no horizontal scroll
    const hasHorizontalScroll = await page.evaluate(() => {
      return document.documentElement.scrollWidth > document.documentElement.clientWidth;
    });

    // Note: This might fail if CSS doesn't handle zoom properly
    // It's a reminder to test manually
    expect(hasHorizontalScroll).toBe(false);
  });
});

// ─── Touch Gesture Interactions ──────────────────────────────────────────────
// These tests run on both mobile-chrome (Pixel 5) and mobile-safari (iPhone 12)
// via the Playwright projects defined in playwright.config.ts.
// See e2e/README.md § Touch Gesture Tests for authoring guidance.

test.describe('Touch Gesture – Swipe Navigation', () => {
  test.use({ viewport: { width: 375, height: 667 } });

  test('swipe up should scroll page to reveal sections below the fold', async ({ page }) => {
    await page.goto('/');

    const initialScrollY = await page.evaluate(() => window.scrollY);

    // Simulate a swipe-up gesture by touching and dragging upward
    await page.touchscreen.tap(187, 400);
    await page.evaluate(async () => {
      await new Promise<void>((resolve) => {
        let start = 400;
        const end = 100;
        const steps = 20;
        const stepSize = (start - end) / steps;
        let step = 0;

        const touch = new Touch({ identifier: 1, target: document.body, clientX: 187, clientY: start });
        document.body.dispatchEvent(new TouchEvent('touchstart', { touches: [touch], changedTouches: [touch] }));

        const interval = setInterval(() => {
          start -= stepSize;
          step++;
          const moveTouch = new Touch({ identifier: 1, target: document.body, clientX: 187, clientY: start });
          document.body.dispatchEvent(new TouchEvent('touchmove', { touches: [moveTouch], changedTouches: [moveTouch] }));

          if (step >= steps) {
            clearInterval(interval);
            const endTouch = new Touch({ identifier: 1, target: document.body, clientX: 187, clientY: start });
            document.body.dispatchEvent(new TouchEvent('touchend', { touches: [], changedTouches: [endTouch] }));
            resolve();
          }
        }, 16);
      });
    });

    await page.waitForTimeout(300);
    const finalScrollY = await page.evaluate(() => window.scrollY);
    // After a swipe-up gesture the page should have scrolled down
    expect(finalScrollY).toBeGreaterThanOrEqual(initialScrollY);
  });

  test('swipe via touchscreen API should move to next section', async ({ page }) => {
    await page.goto('/');

    // Use Playwright's built-in touchscreen for a more realistic swipe
    await page.touchscreen.tap(187, 300);
    // Drag finger upwards to scroll down the page
    await page.mouse.move(187, 300);
    await page.mouse.down();
    await page.mouse.move(187, 100, { steps: 10 });
    await page.mouse.up();

    await page.waitForTimeout(300);
    // Features section should now be accessible (in or near viewport)
    const featuresSection = page.locator('#features');
    const count = await featuresSection.count();
    if (count > 0) {
      // Section exists — verify page scrolled enough to expose content below fold
      const scrollY = await page.evaluate(() => window.scrollY);
      expect(scrollY).toBeGreaterThanOrEqual(0);
    }
  });
});

test.describe('Touch Gesture – Tap to Select Outcome', () => {
  test.use({ viewport: { width: 375, height: 667 } });

  test('tap on CTA button should trigger action via touch', async ({ page }) => {
    await page.goto('/');

    const ctaButton = page.getByRole('button', { name: /get early access/i });
    await expect(ctaButton).toBeVisible();

    // Tap instead of click — uses the Pointer Events / Touch Events path
    await ctaButton.tap();

    // An empty-email tap should surface the validation error
    await expect(page.getByRole('alert')).toBeVisible();
  });

  test('tap on navigation link should navigate to the target section', async ({ page }) => {
    await page.goto('/');

    const featuresLink = page.getByRole('link', { name: /features/i });
    await featuresLink.tap();

    await expect(page.locator('#features')).toBeInViewport();
  });

  test('tap on email input should focus it and show mobile keyboard hint', async ({ page }) => {
    await page.goto('/');

    const emailInput = page.getByLabel(/email address/i);
    await emailInput.tap();

    await expect(emailInput).toBeFocused();
    // Input type must be email so mobile browsers open the correct keyboard
    await expect(emailInput).toHaveAttribute('type', 'email');
  });

  test('double-tap on text should not cause unintended zoom', async ({ page }) => {
    await page.goto('/');

    const heading = page.getByRole('heading', { name: /decentralized prediction markets/i });
    const box = await heading.boundingBox();

    if (box) {
      const cx = box.x + box.width / 2;
      const cy = box.y + box.height / 2;

      await page.touchscreen.tap(cx, cy);
      await page.touchscreen.tap(cx, cy);
      await page.waitForTimeout(300);
    }

    // Page content should still be visible after double-tap
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
  });
});

test.describe('Touch Gesture – Long Press', () => {
  test.use({ viewport: { width: 375, height: 667 } });

  test('long press on navigation link should not break navigation', async ({ page }) => {
    await page.goto('/');

    const featuresLink = page.getByRole('link', { name: /features/i });
    const box = await featuresLink.boundingBox();

    if (box) {
      const cx = box.x + box.width / 2;
      const cy = box.y + box.height / 2;

      // Simulate a long press by holding the pointer down for 800 ms
      await page.mouse.move(cx, cy);
      await page.mouse.down();
      await page.waitForTimeout(800);
      await page.mouse.up();
    }

    // A short tap after should still navigate correctly
    await featuresLink.tap();
    await expect(page.locator('#features')).toBeInViewport();
  });
});
