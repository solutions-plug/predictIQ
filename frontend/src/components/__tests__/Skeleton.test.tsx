import React from 'react';
import { render, screen } from '@testing-library/react';
import { Skeleton } from './Skeleton';

describe('Skeleton', () => {
  it('renders with default props', () => {
    render(<Skeleton />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
    expect(skeleton).toHaveAttribute('aria-live', 'polite');
    expect(skeleton).toHaveAttribute('aria-label', 'Loading content');
  });

  it('renders with custom dimensions', () => {
    render(<Skeleton width="200px" height="50px" />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toHaveStyle({ width: '200px', height: '50px' });
  });

  it('renders with different variants', () => {
    const { rerender } = render(<Skeleton variant="text" />);
    expect(screen.getByRole('status')).toHaveClass('text');

    rerender(<Skeleton variant="rectangular" />);
    expect(screen.getByRole('status')).toHaveClass('rectangular');

    rerender(<Skeleton variant="circular" />);
    expect(screen.getByRole('status')).toHaveClass('circular');
  });

  it('includes visually hidden text for screen readers', () => {
    render(<Skeleton />);
    expect(screen.getByText('Loading content')).toHaveClass('visually-hidden');
  });

  it('applies custom className', () => {
    render(<Skeleton className="custom-skeleton" />);
    expect(screen.getByRole('status')).toHaveClass('custom-skeleton');
  });
});