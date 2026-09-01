import { test, expect } from '@playwright/test';

/**
 * #1346 - every internal footer link resolves (no dead links to removed pages).
 * #1348 - no horizontal scroll at 320 / 768 / 1440.
 */

test.describe('landing footer links resolve (#1346)', () => {
  test('every internal footer link returns a non-error status', async ({ page, request }) => {
    await page.goto('/');
    const footer = page.getByRole('contentinfo');
    const hrefs = await footer.locator('a[href^="/"]:not([href^="//"])').evaluateAll((els) =>
      Array.from(new Set(els.map((el) => (el as HTMLAnchorElement).getAttribute('href') ?? ''))),
    );

    expect(hrefs.length).toBeGreaterThan(0);
    for (const href of hrefs) {
      if (href.startsWith('/#') || href === '/#') continue; // in-page anchor
      const res = await request.get(href);
      expect(res.status(), `${href} should resolve`).toBeLessThan(400);
    }
  });
});

test.describe('landing has no horizontal scroll (#1348)', () => {
  for (const width of [320, 768, 1440]) {
    test(`at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto('/');
      await page.waitForLoadState('networkidle');
      await page.evaluate(() => document.fonts.ready);

      const overflow = await page.evaluate(() => {
        const doc = document.documentElement;
        return doc.scrollWidth - doc.clientWidth;
      });
      // allow a 1px rounding tolerance
      expect(overflow, `viewport ${width}px must not scroll horizontally`).toBeLessThanOrEqual(1);
    });
  }
});
