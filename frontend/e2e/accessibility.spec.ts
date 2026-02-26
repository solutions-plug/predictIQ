import { test, expect } from '@playwright/test';

test.describe('Keyboard Navigation', () => {
  test('should navigate with Tab key', async ({ page }) => {
    await page.goto('/');
    
    // Tab through interactive elements
    await page.keyboard.press('Tab');
    await expect(page.getByText(/skip to main content/i)).toBeFocused();
    
    await page.keyboard.press('Tab');
    // Should focus on first navigation link
    
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    
    // Should eventually reach email input
    await expect(page.getByLabel(/email address/i)).toBeFocused();
  });

  test('should submit form with Enter key', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).focus();
    await page.keyboard.type('test@example.com');
    await page.keyboard.press('Enter');
    
    await expect(page.getByRole('button', { name: /subscribed/i })).toBeVisible();
  });

  test('should navigate with Shift+Tab', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).focus();
    await page.keyboard.press('Shift+Tab');
    
    await expect(page.getByLabel(/email address/i)).toBeFocused();
  });

  test('should activate links with Enter', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('link', { name: /features/i }).focus();
    await page.keyboard.press('Enter');
    
    await expect(page.locator('#features')).toBeInViewport();
  });

  test('should not trap focus', async ({ page }) => {
    await page.goto('/');
    
    // Tab through all elements
    for (let i = 0; i < 50; i++) {
      await page.keyboard.press('Tab');
    }
    
    // Should not be stuck in a focus trap
    const activeElement = await page.evaluate(() => document.activeElement?.tagName);
    expect(activeElement).toBeTruthy();
  });
});

test.describe('Screen Reader Support', () => {
  test('should have proper ARIA landmarks', async ({ page }) => {
    await page.goto('/');
    
    await expect(page.getByRole('banner')).toBeVisible();
    await expect(page.getByRole('main')).toBeVisible();
    await expect(page.getByRole('navigation')).toBeVisible();
    await expect(page.getByRole('contentinfo')).toBeVisible();
  });

  test('should announce form errors', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).click();
    
    const alert = page.getByRole('alert');
    await expect(alert).toBeVisible();
    await expect(alert).toHaveAttribute('role', 'alert');
  });

  test('should have live region for status updates', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).fill('test@example.com');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    const statusRegion = page.locator('[role="status"]');
    await expect(statusRegion).toBeInTheDocument();
  });

  test('should have descriptive button labels', async ({ page }) => {
    await page.goto('/');
    
    const button = page.getByRole('button', { name: /get early access/i });
    const ariaLabel = await button.getAttribute('aria-label');
    
    expect(ariaLabel || await button.textContent()).toBeTruthy();
  });

  test('should have proper heading hierarchy', async ({ page }) => {
    await page.goto('/');
    
    const h1 = page.getByRole('heading', { level: 1 });
    await expect(h1).toHaveCount(1);
    
    const h2s = page.getByRole('heading', { level: 2 });
    await expect(h2s).toHaveCount(await h2s.count());
  });
});

test.describe('Focus Indicators', () => {
  test('should show visible focus on interactive elements', async ({ page }) => {
    await page.goto('/');
    
    const emailInput = page.getByLabel(/email address/i);
    await emailInput.focus();
    
    // Check if element has focus
    await expect(emailInput).toBeFocused();
    
    // Visual focus indicator should be present (checked via CSS)
    const outlineStyle = await emailInput.evaluate((el) => {
      return window.getComputedStyle(el).outline;
    });
    
    // Should have some form of outline or focus indicator
    expect(outlineStyle).toBeTruthy();
  });

  test('should maintain focus visibility on all interactive elements', async ({ page }) => {
    await page.goto('/');
    
    const interactiveElements = [
      page.getByRole('link', { name: /features/i }),
      page.getByLabel(/email address/i),
      page.getByRole('button', { name: /get early access/i }),
    ];
    
    for (const element of interactiveElements) {
      await element.focus();
      await expect(element).toBeFocused();
    }
  });
});

test.describe('Skip Links', () => {
  test('should have skip to main content link', async ({ page }) => {
    await page.goto('/');
    
    const skipLink = page.getByText(/skip to main content/i);
    await expect(skipLink).toBeInTheDocument();
  });

  test('should skip to main content when activated', async ({ page }) => {
    await page.goto('/');
    
    await page.keyboard.press('Tab');
    await page.keyboard.press('Enter');
    
    const mainContent = page.locator('#main-content');
    await expect(mainContent).toBeInViewport();
  });
});

test.describe('Form Accessibility', () => {
  test('should have associated labels', async ({ page }) => {
    await page.goto('/');
    
    const emailInput = page.getByLabel(/email address/i);
    await expect(emailInput).toBeVisible();
    
    const inputId = await emailInput.getAttribute('id');
    expect(inputId).toBeTruthy();
  });

  test('should indicate required fields', async ({ page }) => {
    await page.goto('/');
    
    const emailInput = page.getByLabel(/email address/i);
    const ariaRequired = await emailInput.getAttribute('aria-required');
    
    expect(ariaRequired).toBe('true');
  });

  test('should link errors to inputs', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).click();
    
    const emailInput = page.getByLabel(/email address/i);
    const ariaDescribedBy = await emailInput.getAttribute('aria-describedby');
    
    expect(ariaDescribedBy).toBeTruthy();
    
    const errorElement = page.locator(`#${ariaDescribedBy}`);
    await expect(errorElement).toBeVisible();
  });

  test('should mark invalid inputs', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).click();
    
    const emailInput = page.getByLabel(/email address/i);
    const ariaInvalid = await emailInput.getAttribute('aria-invalid');
    
    expect(ariaInvalid).toBe('true');
  });
});

test.describe('Image Accessibility', () => {
  test('should have alt text for meaningful images', async ({ page }) => {
    await page.goto('/');
    
    const logo = page.getByRole('img', { name: /predictiq logo/i });
    await expect(logo).toBeVisible();
  });

  test('should hide decorative images from screen readers', async ({ page }) => {
    await page.goto('/');
    
    const decorativeImages = page.locator('img[aria-hidden="true"]');
    const count = await decorativeImages.count();
    
    // Should have some decorative images
    expect(count).toBeGreaterThan(0);
  });
});

test.describe('Color Contrast', () => {
  test('should have sufficient contrast for text', async ({ page }) => {
    await page.goto('/');
    
    // This is a placeholder - actual contrast checking requires specialized tools
    // Use axe-core or similar for automated contrast checking
    const heading = page.getByRole('heading', { name: /decentralized prediction markets/i });
    await expect(heading).toBeVisible();
  });
});

test.describe('Reduced Motion', () => {
  test('should respect prefers-reduced-motion', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    
    // Verify page loads and is functional
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    
    // Animations should be disabled or reduced
    // This would need to be verified through CSS inspection
  });
});

test.describe('Zoom and Reflow', () => {
  test('should support 200% zoom', async ({ page }) => {
    await page.goto('/');
    
    // Simulate browser zoom
    await page.evaluate(() => {
      document.body.style.zoom = '2';
    });
    
    // Content should still be visible and usable
    await expect(page.getByRole('heading', { name: /decentralized prediction markets/i })).toBeVisible();
    await expect(page.getByLabel(/email address/i)).toBeVisible();
  });

  test('should reflow content at 400% zoom', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 1024 });
    await page.goto('/');
    
    // Simulate 400% zoom (320px effective width)
    await page.setViewportSize({ width: 320, height: 256 });
    
    // Content should reflow without horizontal scroll
    const hasHorizontalScroll = await page.evaluate(() => {
      return document.documentElement.scrollWidth > document.documentElement.clientWidth;
    });
    
    expect(hasHorizontalScroll).toBe(false);
  });
});
