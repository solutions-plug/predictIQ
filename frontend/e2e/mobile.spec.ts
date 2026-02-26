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
