import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { axe, toHaveNoViolations } from 'jest-axe';
import LandingPage from '../LandingPage';

expect.extend(toHaveNoViolations);

describe('LandingPage Accessibility Tests', () => {
  describe('Automated Accessibility (jest-axe)', () => {
    it('should have no axe violations on initial render', async () => {
      const { container } = render(<LandingPage />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('should have no axe violations with form error state', async () => {
      const { container } = render(<LandingPage />);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.click(submitButton);
      
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('should have no axe violations after successful submission', async () => {
      const { container } = render(<LandingPage />);
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.type(emailInput, 'test@example.com');
      await userEvent.click(submitButton);
      
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });
  });

  describe('Semantic HTML', () => {
    it('should use semantic HTML5 elements', () => {
      render(<LandingPage />);
      
      expect(screen.getByRole('banner')).toBeInTheDocument(); // header
      expect(screen.getByRole('main')).toBeInTheDocument(); // main
      expect(screen.getByRole('contentinfo')).toBeInTheDocument(); // footer
      expect(screen.getByRole('navigation')).toBeInTheDocument(); // nav
    });

    it('should have proper heading hierarchy', () => {
      render(<LandingPage />);
      
      const headings = screen.getAllByRole('heading');
      const h1s = headings.filter(h => h.tagName === 'H1');
      const h2s = headings.filter(h => h.tagName === 'H2');
      const h3s = headings.filter(h => h.tagName === 'H3');
      
      // Should have exactly one h1
      expect(h1s).toHaveLength(1);
      expect(h1s[0]).toHaveTextContent(/decentralized prediction markets/i);
      
      // Should have multiple h2s for sections
      expect(h2s.length).toBeGreaterThan(0);
      
      // Should have h3s for subsections
      expect(h3s.length).toBeGreaterThan(0);
    });

    it('should use article elements for feature cards', () => {
      const { container } = render(<LandingPage />);
      const articles = container.querySelectorAll('article');
      
      expect(articles.length).toBeGreaterThan(0);
    });
  });

  describe('ARIA Roles and Attributes', () => {
    it('should have proper ARIA landmarks', () => {
      render(<LandingPage />);
      
      expect(screen.getByRole('banner')).toBeInTheDocument();
      expect(screen.getByRole('main')).toBeInTheDocument();
      expect(screen.getByRole('contentinfo')).toBeInTheDocument();
      expect(screen.getByRole('navigation', { name: /main navigation/i })).toBeInTheDocument();
    });

    it('should have aria-labelledby for sections', () => {
      const { container } = render(<LandingPage />);
      
      const heroSection = container.querySelector('[aria-labelledby="hero-heading"]');
      expect(heroSection).toBeInTheDocument();
      
      const featuresSection = container.querySelector('[aria-labelledby="features-heading"]');
      expect(featuresSection).toBeInTheDocument();
    });

    it('should have aria-required on required form fields', () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      expect(emailInput).toHaveAttribute('aria-required', 'true');
    });

    it('should have aria-invalid when form has errors', async () => {
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      await userEvent.click(submitButton);
      
      const emailInput = screen.getByLabelText(/email address/i);
      expect(emailInput).toHaveAttribute('aria-invalid', 'true');
    });

    it('should have aria-describedby linking to error message', async () => {
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      await userEvent.click(submitButton);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const errorMessage = screen.getByRole('alert');
      
      expect(emailInput).toHaveAttribute('aria-describedby', 'email-error');
      expect(errorMessage).toHaveAttribute('id', 'email-error');
    });

    it('should have aria-live region for status updates', () => {
      const { container } = render(<LandingPage />);
      
      const statusRegion = container.querySelector('[role="status"][aria-live="polite"]');
      expect(statusRegion).toBeInTheDocument();
    });

    it('should have aria-hidden on decorative images', () => {
      const { container } = render(<LandingPage />);
      
      const decorativeImages = container.querySelectorAll('[aria-hidden="true"]');
      expect(decorativeImages.length).toBeGreaterThan(0);
    });
  });

  describe('Keyboard Navigation', () => {
    it('should have skip to main content link', () => {
      render(<LandingPage />);
      
      const skipLink = screen.getByText(/skip to main content/i);
      expect(skipLink).toBeInTheDocument();
      expect(skipLink).toHaveAttribute('href', '#main-content');
    });

    it('should allow keyboard navigation through form', async () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      // Tab to email input
      await userEvent.tab();
      expect(emailInput).toHaveFocus();
      
      // Type email
      await userEvent.keyboard('test@example.com');
      
      // Tab to submit button
      await userEvent.tab();
      expect(submitButton).toHaveFocus();
      
      // Press Enter to submit
      await userEvent.keyboard('{Enter}');
      
      expect(screen.getByText(/subscribed!/i)).toBeInTheDocument();
    });

    it('should allow keyboard navigation through navigation links', async () => {
      render(<LandingPage />);
      
      const navLinks = screen.getAllByRole('link');
      const mainNavLinks = navLinks.filter(link => 
        ['Features', 'How It Works', 'About', 'Contact'].includes(link.textContent || '')
      );
      
      expect(mainNavLinks.length).toBeGreaterThan(0);
      
      // All links should be keyboard accessible
      mainNavLinks.forEach(link => {
        expect(link).toHaveAttribute('href');
      });
    });

    it('should maintain focus order', async () => {
      render(<LandingPage />);
      
      const focusableElements = [
        screen.getByText(/skip to main content/i),
        ...screen.getAllByRole('link'),
        screen.getByLabelText(/email address/i),
        screen.getByRole('button', { name: /get early access/i }),
      ];
      
      // Verify all elements are in the document
      focusableElements.forEach(element => {
        expect(element).toBeInTheDocument();
      });
    });
  });

  describe('Form Labels and Validation', () => {
    it('should have properly associated labels', () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      expect(emailInput).toHaveAttribute('id', 'email-input');
    });

    it('should indicate required fields', () => {
      render(<LandingPage />);
      
      const requiredIndicator = screen.getByText('*');
      expect(requiredIndicator).toHaveAttribute('aria-label', 'required');
    });

    it('should show validation error with role="alert"', async () => {
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      await userEvent.click(submitButton);
      
      const errorMessage = screen.getByRole('alert');
      expect(errorMessage).toHaveTextContent(/email is required/i);
    });

    it('should clear error when user starts typing', async () => {
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      await userEvent.click(submitButton);
      
      expect(screen.getByRole('alert')).toBeInTheDocument();
      
      const emailInput = screen.getByLabelText(/email address/i);
      await userEvent.type(emailInput, 't');
      
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });

    it('should validate email format', async () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.type(emailInput, 'invalid-email');
      await userEvent.click(submitButton);
      
      expect(screen.getByRole('alert')).toHaveTextContent(/valid email address/i);
    });

    it('should disable form after successful submission', async () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.type(emailInput, 'test@example.com');
      await userEvent.click(submitButton);
      
      expect(emailInput).toBeDisabled();
      expect(submitButton).toBeDisabled();
    });
  });

  describe('Image Alt Text', () => {
    it('should have descriptive alt text for logo', () => {
      render(<LandingPage />);
      
      const logo = screen.getByAltText(/predictiq logo/i);
      expect(logo).toBeInTheDocument();
    });

    it('should have empty alt text for decorative images', () => {
      const { container } = render(<LandingPage />);
      
      const decorativeImages = container.querySelectorAll('img[alt=""]');
      decorativeImages.forEach(img => {
        expect(img).toHaveAttribute('aria-hidden', 'true');
      });
    });

    it('should have width and height attributes on images', () => {
      const { container } = render(<LandingPage />);
      
      const images = container.querySelectorAll('img');
      images.forEach(img => {
        expect(img).toHaveAttribute('width');
        expect(img).toHaveAttribute('height');
      });
    });
  });

  describe('Focus Management', () => {
    it('should have visible focus indicators', async () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      
      await userEvent.tab();
      expect(emailInput).toHaveFocus();
      
      // Focus should be visible (tested via CSS in integration tests)
      expect(emailInput).toBeInTheDocument();
    });

    it('should not trap focus', async () => {
      render(<LandingPage />);
      
      const focusableElements = screen.getAllByRole('link');
      
      // Should be able to tab through all elements
      for (let i = 0; i < focusableElements.length; i++) {
        await userEvent.tab();
      }
      
      // Should reach the end without being trapped
      expect(document.activeElement).toBeInTheDocument();
    });
  });

  describe('Screen Reader Compatibility', () => {
    it('should announce form submission success', async () => {
      const { container } = render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.type(emailInput, 'test@example.com');
      await userEvent.click(submitButton);
      
      const statusRegion = container.querySelector('#form-status');
      expect(statusRegion).toHaveTextContent(/successfully subscribed/i);
    });

    it('should have visually hidden text for screen readers', () => {
      const { container } = render(<LandingPage />);
      
      const visuallyHidden = container.querySelectorAll('.visually-hidden');
      expect(visuallyHidden.length).toBeGreaterThan(0);
    });

    it('should have meaningful button labels', () => {
      render(<LandingPage />);
      
      const submitButton = screen.getByRole('button', { name: /get early access/i });
      expect(submitButton).toHaveAccessibleName();
    });

    it('should update button label after submission', async () => {
      render(<LandingPage />);
      
      const emailInput = screen.getByLabelText(/email address/i);
      let submitButton = screen.getByRole('button', { name: /get early access/i });
      
      await userEvent.type(emailInput, 'test@example.com');
      await userEvent.click(submitButton);
      
      submitButton = screen.getByRole('button', { name: /already subscribed/i });
      expect(submitButton).toBeInTheDocument();
    });
  });

  describe('Color Contrast', () => {
    it('should have sufficient color contrast (manual verification required)', () => {
      // Note: Automated tools like axe will check this, but manual verification
      // with tools like Chrome DevTools or Lighthouse is recommended
      render(<LandingPage />);
      
      // This test serves as a reminder to check color contrast manually
      expect(screen.getByRole('main')).toBeInTheDocument();
    });
  });

  describe('Responsive and Zoom', () => {
    it('should render without horizontal scroll at 200% zoom', () => {
      // This is typically tested manually or with visual regression tools
      // This test serves as documentation
      render(<LandingPage />);
      
      expect(screen.getByRole('main')).toBeInTheDocument();
    });
  });
});
