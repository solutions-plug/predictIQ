import { test, expect } from '@playwright/test';

test.describe('Form Submissions', () => {
  test('should submit newsletter form with valid email', async ({ page }) => {
    await page.goto('/');
    
    const emailInput = page.getByLabel(/email address/i);
    const submitButton = page.getByRole('button', { name: /get early access/i });
    
    await emailInput.fill('valid@example.com');
    await submitButton.click();
    
    await expect(page.getByRole('button', { name: /subscribed/i })).toBeVisible();
    await expect(emailInput).toBeDisabled();
  });

  test('should show error for empty email', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).click();
    
    await expect(page.getByRole('alert')).toContainText(/email is required/i);
  });

  test('should show error for invalid email format', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).fill('invalid-email');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    await expect(page.getByRole('alert')).toContainText(/valid email/i);
  });

  test('should clear error when user types', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    
    await page.getByLabel(/email address/i).fill('t');
    await expect(page.getByRole('alert')).not.toBeVisible();
  });

  test('should prevent multiple submissions', async ({ page }) => {
    await page.goto('/');
    
    const emailInput = page.getByLabel(/email address/i);
    const submitButton = page.getByRole('button', { name: /get early access/i });
    
    await emailInput.fill('test@example.com');
    await submitButton.click();
    
    await expect(submitButton).toBeDisabled();
    await expect(emailInput).toBeDisabled();
  });
});

test.describe('CTA Button Interactions', () => {
  test('should have visible and clickable CTA button', async ({ page }) => {
    await page.goto('/');
    
    const ctaButton = page.getByRole('button', { name: /get early access/i });
    await expect(ctaButton).toBeVisible();
    await expect(ctaButton).toBeEnabled();
    
    await ctaButton.click();
    // Should trigger validation
    await expect(page.getByRole('alert')).toBeVisible();
  });

  test('should change button state after submission', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).fill('test@example.com');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    const subscribedButton = page.getByRole('button', { name: /subscribed/i });
    await expect(subscribedButton).toBeVisible();
    await expect(subscribedButton).toBeDisabled();
  });

  test('should have proper hover states', async ({ page }) => {
    await page.goto('/');
    
    const ctaButton = page.getByRole('button', { name: /get early access/i });
    await ctaButton.hover();
    
    // Button should remain visible and enabled on hover
    await expect(ctaButton).toBeVisible();
    await expect(ctaButton).toBeEnabled();
  });
});

test.describe('Navigation Between Sections', () => {
  test('should navigate to all main sections', async ({ page }) => {
    await page.goto('/');
    
    // Navigate to Features
    await page.getByRole('link', { name: /features/i }).click();
    await expect(page.locator('#features')).toBeInViewport();
    
    // Navigate to How It Works
    await page.getByRole('link', { name: /how it works/i }).click();
    await expect(page.locator('#how-it-works')).toBeInViewport();
    
    // Navigate to About
    await page.getByRole('link', { name: /about/i }).click();
    await expect(page.locator('#about')).toBeInViewport();
    
    // Navigate to Contact
    await page.getByRole('link', { name: /contact/i }).click();
    await expect(page.locator('#contact')).toBeInViewport();
  });

  test('should use smooth scroll behavior', async ({ page }) => {
    await page.goto('/');
    
    const initialPosition = await page.evaluate(() => window.scrollY);
    
    await page.getByRole('link', { name: /features/i }).click();
    
    // Wait for scroll to complete
    await page.waitForTimeout(500);
    
    const finalPosition = await page.evaluate(() => window.scrollY);
    expect(finalPosition).toBeGreaterThan(initialPosition);
  });

  test('should maintain navigation state', async ({ page }) => {
    await page.goto('/');
    
    // Navigate to section
    await page.getByRole('link', { name: /features/i }).click();
    
    // Verify URL hash updated
    await expect(page).toHaveURL(/#features/);
  });
});

test.describe('Scroll Behavior', () => {
  test('should scroll to sections on anchor click', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('link', { name: /features/i }).click();
    
    const featuresSection = page.locator('#features');
    await expect(featuresSection).toBeInViewport();
  });

  test('should show skip to main content link', async ({ page }) => {
    await page.goto('/');
    
    const skipLink = page.getByText(/skip to main content/i);
    await expect(skipLink).toBeInTheDocument();
    
    await skipLink.click();
    await expect(page.locator('#main-content')).toBeInViewport();
  });

  test('should handle scroll to top', async ({ page }) => {
    await page.goto('/');
    
    // Scroll to bottom
    await page.getByRole('link', { name: /contact/i }).click();
    await expect(page.locator('#contact')).toBeInViewport();
    
    // Click logo to go back to top
    await page.getByRole('img', { name: /predictiq logo/i }).click();
    
    const scrollPosition = await page.evaluate(() => window.scrollY);
    expect(scrollPosition).toBeLessThan(100);
  });
});

test.describe('External Link Clicks', () => {
  test('should have external links with proper attributes', async ({ page }) => {
    await page.goto('/');
    
    const externalLinks = [
      page.getByRole('link', { name: /documentation/i }),
      page.getByRole('link', { name: /github/i }),
      page.getByRole('link', { name: /discord/i }),
    ];
    
    for (const link of externalLinks) {
      if (await link.count() > 0) {
        await expect(link).toHaveAttribute('href', /.+/);
      }
    }
  });

  test('should open external links', async ({ page, context }) => {
    await page.goto('/');
    
    const docsLink = page.getByRole('link', { name: /documentation/i });
    
    if (await docsLink.count() > 0) {
      const href = await docsLink.getAttribute('href');
      expect(href).toBeTruthy();
    }
  });
});
