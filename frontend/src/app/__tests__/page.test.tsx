import React, { Suspense } from 'react';
import { render, screen } from '@testing-library/react';
import { axe, toHaveNoViolations } from 'jest-axe';
import { LoadingSpinner } from '../../components/LoadingSpinner';

expect.extend(toHaveNoViolations);

describe('Home page loading fallback', () => {
  it('LoadingSpinner fallback has role=status', () => {
    render(<LoadingSpinner aria-label="Loading page" />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('LoadingSpinner fallback has aria-live=polite', () => {
    render(<LoadingSpinner aria-label="Loading page" />);
    expect(screen.getByRole('status')).toHaveAttribute('aria-live', 'polite');
  });

  it('LoadingSpinner fallback has an accessible label', () => {
    render(<LoadingSpinner aria-label="Loading page" />);
    expect(screen.getByRole('status')).toHaveAttribute('aria-label', 'Loading page');
  });

  it('LoadingSpinner fallback has no axe violations', async () => {
    const { container } = render(<LoadingSpinner aria-label="Loading page" />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });
});
