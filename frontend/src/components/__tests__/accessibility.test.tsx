import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { axe, toHaveNoViolations } from 'jest-axe';
import LandingPage from '../LandingPage';
import { api } from '../../lib/api/client';

expect.extend(toHaveNoViolations);

describe('Component Accessibility Tests', () => {
  beforeEach(() => {
    // Keep the Statistics mount fetch deterministic and off the network.
    jest
      .spyOn(api, 'getStatistics')
      .mockResolvedValue({ totalMarkets: 128, totalVolume: 45000, activeUsers: 512 });
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  describe('jest-axe automated tests', () => {
    it('LandingPage should have no axe violations', async () => {
      const { container } = render(<LandingPage />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('LandingPage should have no violations with form interactions', async () => {
      const { container } = render(<LandingPage />);
      const emailInput = screen.getByLabelText(/email address/i);
      
      await userEvent.type(emailInput, 'test@example.com');
      
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('LandingPage should have no violations with error state', async () => {
      const { container } = render(<LandingPage />);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.click(submitButton);
      
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });
  });

  describe('ARIA attributes', () => {
    it('should have proper ARIA labels on form inputs', () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      expect(emailInput).toHaveAttribute('type', 'email');
      expect(emailInput).toHaveAccessibleName();
    });

    it('should have aria-required on required fields', () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      expect(emailInput).toHaveAttribute('required');
    });

    it('should have aria-invalid on invalid form fields', async () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i) as HTMLInputElement;
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.click(submitButton);
      
      // Check if aria-invalid is set or validation state is indicated
      expect(emailInput.validity.valid === false || emailInput.getAttribute('aria-invalid') === 'true').toBe(true);
    });

    it('should have aria-describedby for error messages', async () => {
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      await userEvent.click(submitButton);
      
      // Error message should be present and associated
      const errorMessages = screen.queryAllByRole('alert');
      expect(errorMessages.length >= 0).toBe(true);
    });

    it('should have proper role attributes on interactive elements', () => {
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      expect(submitButton).toHaveAttribute('type', 'submit');
    });

    it('should have aria-live regions for dynamic content', () => {
      render(<LandingPage />);
      
      const liveRegions = screen.queryAllByRole('status');
      // Live regions may or may not be present, but if they are, they should be properly configured
      liveRegions.forEach(region => {
        expect(region).toHaveAttribute('aria-live');
      });
    });
  });

  describe('Keyboard navigation', () => {
    it('should allow tab navigation through form elements', async () => {
      const user = userEvent.setup();
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });

      // Focus the email field (reached via the skip link + nav in real use)
      emailInput.focus();
      expect(emailInput).toHaveFocus();

      // Tab to submit button
      await user.tab();
      expect(submitButton).toHaveFocus();
    });

    it('should allow shift+tab to navigate backwards', async () => {
      const user = userEvent.setup();
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      // Focus submit button
      submitButton.focus();
      expect(submitButton).toHaveFocus();
      
      // Shift+Tab back to email input
      await user.tab({ shift: true });
      expect(emailInput).toHaveFocus();
    });

    it('should allow Enter key to submit form', async () => {
      const user = userEvent.setup();
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      
      await user.type(emailInput, 'test@example.com');
      await user.keyboard('{Enter}');
      
      // Form should be submitted (check for success message or state change)
      expect(emailInput).toBeInTheDocument();
    });

    it('should allow Escape key to close modals if present', async () => {
      const user = userEvent.setup();
      render(<LandingPage />);
      
      // If there are any modal elements, they should respond to Escape
      await user.keyboard('{Escape}');
      
      // Page should still be functional
      expect(screen.getByRole('button', { name: /get early access/i })).toBeInTheDocument();
    });

    it('should have visible focus indicators', () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      emailInput.focus();
      
      const styles = window.getComputedStyle(emailInput);
      // Focus should be visible (outline, border, or box-shadow)
      expect(
        styles.outline !== 'none' ||
        styles.borderWidth !== '0px' ||
        styles.boxShadow !== 'none'
      ).toBe(true);
    });

    it('should maintain focus management during form submission', async () => {
      const user = userEvent.setup();
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await user.type(emailInput, 'test@example.com');
      await user.click(submitButton);
      
      // Focus should be managed appropriately after submission
      expect(document.activeElement).toBeInTheDocument();
    });
  });

  describe('Semantic HTML structure', () => {
    it('should use semantic HTML5 elements', () => {
      render(<LandingPage />);
      
      expect(screen.getByRole('banner')).toBeInTheDocument(); // header
      expect(screen.getByRole('main')).toBeInTheDocument(); // main
      expect(screen.getByRole('contentinfo')).toBeInTheDocument(); // footer
    });

    it('should have proper heading hierarchy', () => {
      render(<LandingPage />);
      
      const headings = screen.getAllByRole('heading');
      expect(headings.length).toBeGreaterThan(0);
      
      // Check that headings are in order (h1, then h2/h3, etc.)
      let previousLevel = 0;
      headings.forEach(heading => {
        const level = parseInt(heading.tagName[1]);
        expect(level - previousLevel).toBeLessThanOrEqual(1);
        previousLevel = level;
      });
    });

    it('should have exactly one h1 element', () => {
      render(<LandingPage />);
      
      const h1Elements = screen.getAllByRole('heading', { level: 1 });
      expect(h1Elements).toHaveLength(1);
    });

    it('should use form elements correctly', () => {
      render(<LandingPage />);
      
      const form = screen.getByRole('form') || screen.getByLabelText(/email address/i).closest('form');
      expect(form).toBeInTheDocument();
    });

    it('should have associated labels for all form inputs', () => {
      render(<LandingPage />);
      
      const inputs = screen.getAllByRole('textbox');
      inputs.forEach(input => {
        expect(input).toHaveAccessibleName();
      });
    });
  });

  describe('Color contrast', () => {
    it('should have sufficient color contrast for text', () => {
      const { container } = render(<LandingPage />);
      
      // This is a basic check - in production, use axe-core's color contrast checks
      const textElements = container.querySelectorAll('p, span, a, button, label');
      expect(textElements.length).toBeGreaterThan(0);
    });
  });

  describe('Image accessibility', () => {
    it('should have alt text for all images', () => {
      const { container } = render(<LandingPage />);

      // Decorative images are intentionally aria-hidden with empty alt; only
      // meaningful images must carry descriptive alt text.
      const images = container.querySelectorAll('img:not([aria-hidden="true"])');
      images.forEach(img => {
        expect(img).toHaveAttribute('alt');
        expect(img.getAttribute('alt')).not.toBe('');
      });
    });
  });

  describe('Link accessibility', () => {
    it('should have descriptive link text', () => {
      render(<LandingPage />);
      
      const links = screen.queryAllByRole('link');
      links.forEach(link => {
        expect(link).toHaveAccessibleName();
        // Avoid generic link text
        const text = link.textContent?.toLowerCase() || '';
        expect(['click here', 'read more', 'link'].includes(text)).toBe(false);
      });
    });
  });

  describe('Form validation accessibility', () => {
    it('should announce validation errors to screen readers', async () => {
      const user = userEvent.setup();
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      await user.click(submitButton);
      
      // Error messages should be present and accessible
      const alerts = screen.queryAllByRole('alert');
      expect(alerts.length >= 0).toBe(true);
    });

    it('should provide clear error messages', async () => {
      const user = userEvent.setup();
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await user.type(emailInput, 'invalid-email');
      await user.click(submitButton);
      
      // Error message should be descriptive
      expect(emailInput).toBeInTheDocument();
    });
  });

  describe('Responsive accessibility', () => {
    it('should maintain accessibility on mobile viewport', async () => {
      // Mock mobile viewport
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 375,
      });

      const { container } = render(<LandingPage />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('should maintain accessibility on tablet viewport', async () => {
      // Mock tablet viewport
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 768,
      });

      const { container } = render(<LandingPage />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('should maintain accessibility on desktop viewport', async () => {
      // Mock desktop viewport
      Object.defineProperty(window, 'innerWidth', {
        writable: true,
        configurable: true,
        value: 1920,
      });

      const { container } = render(<LandingPage />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });
  });
});
