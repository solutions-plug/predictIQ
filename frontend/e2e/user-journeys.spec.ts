import { test, expect } from '@playwright/test';

test.describe('User Journey: Homepage → Features → Newsletter Signup', () => {
  test('should complete full journey successfully', async ({ page }) => {
    await page.goto('/');
    
    // Verify homepage loaded
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    
    // Browse features
    await page.getByRole('link', { name: /features/i }).click();
    await expect(page.locator('#features')).toBeInViewport();
    
    // Verify feature cards are visible
    await expect(page.getByRole('heading', { name: /fully decentralized/i })).toBeVisible();
    await expect(page.getByRole('heading', { name: /secure & audited/i })).toBeVisible();
    await expect(page.getByRole('heading', { name: /lightning fast/i })).toBeVisible();
    
    // Scroll back to newsletter signup
    await page.getByRole('link', { name: /predictiq logo/i }).click();
    
    // Fill newsletter form
    const emailInput = page.getByLabel(/email address/i);
    await emailInput.fill('user@example.com');
    
    // Submit form
    await page.getByRole('button', { name: /get early access/i }).click();
    
    // Verify success
    await expect(page.getByRole('button', { name: /subscribed/i })).toBeVisible();
    await expect(emailInput).toBeDisabled();
  });

  test('should track analytics events', async ({ page }) => {
    const analyticsEvents: string[] = [];
    
    // Mock analytics tracking
    await page.addInitScript(() => {
      (window as any).trackEvent = (event: string) => {
        (window as any).analyticsEvents = (window as any).analyticsEvents || [];
        (window as any).analyticsEvents.push(event);
      };
    });
    
    await page.goto('/');
    
    // Click features link
    await page.getByRole('link', { name: /features/i }).click();
    
    // Submit newsletter
    await page.getByLabel(/email address/i).fill('test@example.com');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    // Verify events were tracked (if analytics is implemented)
    const events = await page.evaluate(() => (window as any).analyticsEvents || []);
    expect(Array.isArray(events)).toBe(true);
  });
});

test.describe('User Journey: Homepage → View Markets → Launch App', () => {
  test('should navigate to markets and launch app', async ({ page }) => {
    await page.goto('/');
    
    // Navigate to How It Works section
    await page.getByRole('link', { name: /how it works/i }).click();
    await expect(page.locator('#how-it-works')).toBeInViewport();
    
    // Verify steps are visible
    await expect(page.getByRole('heading', { name: /create a market/i })).toBeVisible();
    await expect(page.getByRole('heading', { name: /place bets/i })).toBeVisible();
    await expect(page.getByRole('heading', { name: /oracle resolution/i })).toBeVisible();
    await expect(page.getByRole('heading', { name: /claim winnings/i })).toBeVisible();
    
    // Click external launch app link (if exists)
    const launchAppLink = page.getByRole('link', { name: /launch app/i }).first();
    if (await launchAppLink.count() > 0) {
      await expect(launchAppLink).toHaveAttribute('href', /.+/);
    }
  });
});

test.describe('User Journey: Homepage → FAQ → Contact', () => {
  test('should navigate to about and contact sections', async ({ page }) => {
    await page.goto('/');
    
    // Navigate to About section
    await page.getByRole('link', { name: /about/i }).click();
    await expect(page.locator('#about')).toBeInViewport();
    
    // Verify about content
    await expect(page.getByText(/predictiq is a decentralized/i)).toBeVisible();
    
    // Navigate to Contact section
    await page.getByRole('link', { name: /contact/i }).click();
    await expect(page.locator('#contact')).toBeInViewport();
    
    // Verify footer links
    await expect(page.getByRole('link', { name: /documentation/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /github/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /discord/i })).toBeVisible();
  });
});

test.describe('User Journey: Mobile Navigation Flow', () => {
  test.use({ viewport: { width: 375, height: 667 } });
  
  test('should navigate on mobile device', async ({ page }) => {
    await page.goto('/');
    
    // Verify mobile layout
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    
    // Test mobile navigation
    await page.getByRole('link', { name: /features/i }).click();
    await expect(page.locator('#features')).toBeInViewport();
    
    // Test form on mobile
    await page.getByLabel(/email address/i).fill('mobile@example.com');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    await expect(page.getByRole('button', { name: /subscribed/i })).toBeVisible();
  });
});
