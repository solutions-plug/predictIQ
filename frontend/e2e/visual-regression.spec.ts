import { test, expect } from '@playwright/test';

test.describe('Visual Regression - Homepage', () => {
  test('should match homepage screenshot', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    
    await expect(page).toHaveScreenshot('homepage.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });

  test('should match hero section', async ({ page }) => {
    await page.goto('/');
    
    const heroSection = page.locator('.hero');
    await expect(heroSection).toHaveScreenshot('hero-section.png');
  });

  test('should match features section', async ({ page }) => {
    await page.goto('/');
    
    const featuresSection = page.locator('#features');
    await expect(featuresSection).toHaveScreenshot('features-section.png');
  });

  test('should match footer', async ({ page }) => {
    await page.goto('/');
    
    const footer = page.getByRole('contentinfo');
    await expect(footer).toHaveScreenshot('footer.png');
  });
});

test.describe('Visual Regression - Form States', () => {
  test('should match form initial state', async ({ page }) => {
    await page.goto('/');
    
    const form = page.locator('form');
    await expect(form).toHaveScreenshot('form-initial.png');
  });

  test('should match form error state', async ({ page }) => {
    await page.goto('/');
    
    await page.getByRole('button', { name: /get early access/i }).click();
    
    const form = page.locator('form');
    await expect(form).toHaveScreenshot('form-error.png');
  });

  test('should match form success state', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).fill('test@example.com');
    await page.getByRole('button', { name: /get early access/i }).click();
    
    const form = page.locator('form');
    await expect(form).toHaveScreenshot('form-success.png');
  });

  test('should match form focused state', async ({ page }) => {
    await page.goto('/');
    
    await page.getByLabel(/email address/i).focus();
    
    const form = page.locator('form');
    await expect(form).toHaveScreenshot('form-focused.png');
  });
});

test.describe('Visual Regression - Mobile', () => {
  test.use({ viewport: { width: 375, height: 667 } });
  
  test('should match mobile homepage', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    
    await expect(page).toHaveScreenshot('mobile-homepage.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });

  test('should match mobile navigation', async ({ page }) => {
    await page.goto('/');
    
    const nav = page.getByRole('navigation');
    await expect(nav).toHaveScreenshot('mobile-navigation.png');
  });

  test('should match mobile form', async ({ page }) => {
    await page.goto('/');
    
    const form = page.locator('form');
    await expect(form).toHaveScreenshot('mobile-form.png');
  });
});

test.describe('Visual Regression - Tablet', () => {
  test.use({ viewport: { width: 768, height: 1024 } });
  
  test('should match tablet homepage', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    
    await expect(page).toHaveScreenshot('tablet-homepage.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });
});

test.describe('Visual Regression - Hover States', () => {
  test('should match button hover state', async ({ page }) => {
    await page.goto('/');
    
    const button = page.getByRole('button', { name: /get early access/i });
    await button.hover();
    
    await expect(button).toHaveScreenshot('button-hover.png');
  });

  test('should match link hover state', async ({ page }) => {
    await page.goto('/');
    
    const link = page.getByRole('link', { name: /features/i });
    await link.hover();
    
    await expect(link).toHaveScreenshot('link-hover.png');
  });
});

test.describe('Visual Regression - Dark Mode', () => {
  test('should match dark mode if supported', async ({ page }) => {
    await page.emulateMedia({ colorScheme: 'dark' });
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    
    await expect(page).toHaveScreenshot('homepage-dark.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });
});

test.describe('Visual Regression - Accessibility', () => {
  test('should match high contrast mode', async ({ page }) => {
    await page.emulateMedia({ forcedColors: 'active' });
    await page.goto('/');
    
    await expect(page).toHaveScreenshot('homepage-high-contrast.png', {
      fullPage: true,
    });
  });

  test('should match reduced motion', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    
    await expect(page).toHaveScreenshot('homepage-reduced-motion.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });
});

test.describe('Visual Regression - Breakpoints', () => {
  const breakpoints = [
    { name: 'mobile-320', width: 320, height: 568 },
    { name: 'mobile-375', width: 375, height: 667 },
    { name: 'tablet-768', width: 768, height: 1024 },
    { name: 'desktop-1024', width: 1024, height: 768 },
    { name: 'desktop-1440', width: 1440, height: 900 },
  ];

  for (const breakpoint of breakpoints) {
    test(`should match ${breakpoint.name} layout`, async ({ page }) => {
      await page.setViewportSize({ width: breakpoint.width, height: breakpoint.height });
      await page.goto('/');
      await page.waitForLoadState('networkidle');
      
      await expect(page).toHaveScreenshot(`${breakpoint.name}.png`, {
        fullPage: true,
        animations: 'disabled',
      });
    });
  }
});
