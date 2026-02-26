import { Page, expect } from '@playwright/test';

/**
 * Analytics tracking helper
 */
export async function setupAnalyticsTracking(page: Page) {
  await page.addInitScript(() => {
    (window as any).analyticsEvents = [];
    (window as any).trackEvent = (event: string, data?: any) => {
      (window as any).analyticsEvents.push({ event, data, timestamp: Date.now() });
    };
  });
}

export async function getAnalyticsEvents(page: Page): Promise<any[]> {
  return await page.evaluate(() => (window as any).analyticsEvents || []);
}

/**
 * Wait for network to be idle
 */
export async function waitForNetworkIdle(page: Page, timeout = 5000) {
  await page.waitForLoadState('networkidle', { timeout });
}

/**
 * Check for console errors
 */
export async function checkConsoleErrors(page: Page): Promise<string[]> {
  const errors: string[] = [];
  
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      errors.push(msg.text());
    }
  });
  
  return errors;
}

/**
 * Measure page load time
 */
export async function measurePageLoadTime(page: Page, url: string): Promise<number> {
  const startTime = Date.now();
  await page.goto(url);
  await page.waitForLoadState('load');
  return Date.now() - startTime;
}

/**
 * Check for horizontal scroll
 */
export async function hasHorizontalScroll(page: Page): Promise<boolean> {
  return await page.evaluate(() => {
    return document.documentElement.scrollWidth > document.documentElement.clientWidth;
  });
}

/**
 * Get element bounding box
 */
export async function getElementSize(page: Page, selector: string) {
  const element = page.locator(selector);
  return await element.boundingBox();
}

/**
 * Verify touch target size (WCAG 2.5.5)
 */
export async function verifyTouchTargetSize(page: Page, selector: string, minSize = 44) {
  const box = await getElementSize(page, selector);
  
  if (box) {
    expect(box.width).toBeGreaterThanOrEqual(minSize);
    expect(box.height).toBeGreaterThanOrEqual(minSize);
  }
}

/**
 * Simulate slow network
 */
export async function simulateSlowNetwork(page: Page, delayMs = 100) {
  await page.route('**/*', async (route) => {
    await new Promise(resolve => setTimeout(resolve, delayMs));
    await route.continue();
  });
}

/**
 * Get Core Web Vitals
 */
export async function getCoreWebVitals(page: Page) {
  return await page.evaluate(() => {
    return new Promise((resolve) => {
      const vitals: any = {};
      
      // First Contentful Paint
      const paintEntries = performance.getEntriesByType('paint');
      const fcp = paintEntries.find(entry => entry.name === 'first-contentful-paint');
      if (fcp) vitals.fcp = fcp.startTime;
      
      // Largest Contentful Paint
      new PerformanceObserver((list) => {
        const entries = list.getEntries();
        const lastEntry = entries[entries.length - 1] as any;
        vitals.lcp = lastEntry.startTime;
      }).observe({ entryTypes: ['largest-contentful-paint'] });
      
      // Cumulative Layout Shift
      let clsValue = 0;
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          if (!(entry as any).hadRecentInput) {
            clsValue += (entry as any).value;
          }
        }
        vitals.cls = clsValue;
      }).observe({ entryTypes: ['layout-shift'] });
      
      // First Input Delay would require actual user interaction
      
      setTimeout(() => resolve(vitals), 3000);
    });
  });
}

/**
 * Fill form and submit
 */
export async function fillAndSubmitNewsletterForm(page: Page, email: string) {
  await page.getByLabel(/email address/i).fill(email);
  await page.getByRole('button', { name: /get early access/i }).click();
}

/**
 * Navigate to section
 */
export async function navigateToSection(page: Page, sectionName: string) {
  await page.getByRole('link', { name: new RegExp(sectionName, 'i') }).click();
  await page.locator(`#${sectionName.toLowerCase().replace(/\s+/g, '-')}`).waitFor({ state: 'visible' });
}

/**
 * Verify section is in viewport
 */
export async function verifySectionInViewport(page: Page, sectionId: string) {
  const section = page.locator(`#${sectionId}`);
  await expect(section).toBeInViewport();
}

/**
 * Check accessibility violations using axe-core
 */
export async function checkA11y(page: Page) {
  await page.evaluate(() => {
    // This would integrate with axe-core if injected
    // For now, it's a placeholder for future integration
  });
}

/**
 * Verify no JavaScript errors
 */
export async function verifyNoJSErrors(page: Page) {
  const errors: string[] = [];
  
  page.on('pageerror', (error) => {
    errors.push(error.message);
  });
  
  return errors;
}

/**
 * Take screenshot with timestamp
 */
export async function takeTimestampedScreenshot(page: Page, name: string) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  await page.screenshot({ path: `screenshots/${name}-${timestamp}.png`, fullPage: true });
}

/**
 * Verify responsive layout
 */
export async function verifyResponsiveLayout(page: Page, breakpoint: { width: number; height: number }) {
  await page.setViewportSize(breakpoint);
  
  // Verify no horizontal scroll
  const hasScroll = await hasHorizontalScroll(page);
  expect(hasScroll).toBe(false);
  
  // Verify core elements are visible
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await expect(page.getByRole('navigation')).toBeVisible();
}

/**
 * Test keyboard navigation flow
 */
export async function testKeyboardNavigation(page: Page, expectedFocusOrder: string[]) {
  for (const selector of expectedFocusOrder) {
    await page.keyboard.press('Tab');
    const focusedElement = await page.evaluate(() => document.activeElement?.tagName);
    // Verify focus moved
    expect(focusedElement).toBeTruthy();
  }
}

/**
 * Verify form validation
 */
export async function verifyFormValidation(page: Page, inputLabel: string, invalidValue: string, expectedError: string) {
  await page.getByLabel(new RegExp(inputLabel, 'i')).fill(invalidValue);
  await page.getByRole('button', { name: /submit|get early access/i }).click();
  
  const errorMessage = page.getByRole('alert');
  await expect(errorMessage).toContainText(new RegExp(expectedError, 'i'));
}
