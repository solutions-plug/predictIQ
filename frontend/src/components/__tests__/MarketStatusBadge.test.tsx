import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { MarketStatusBadge } from '../markets/MarketStatusBadge';

describe('MarketStatusBadge', () => {
  it('renders Active status correctly', () => {
    render(<MarketStatusBadge status="Active" />);

    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--success');
  });

  it('renders PendingResolution status correctly', () => {
    render(<MarketStatusBadge status="PendingResolution" />);

    expect(screen.getByText('Pending Resolution')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--warning');
  });

  it('renders Disputed status correctly', () => {
    render(<MarketStatusBadge status="Disputed" />);

    expect(screen.getByText('Disputed')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--info');
  });

  it('renders Resolved status correctly', () => {
    render(<MarketStatusBadge status="Resolved" />);

    expect(screen.getByText('Resolved')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--neutral');
  });

  it('renders Cancelled status correctly', () => {
    render(<MarketStatusBadge status="Cancelled" />);

    expect(screen.getByText('Cancelled')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--danger');
  });

  it('renders Unknown status for unrecognized state', () => {
    render(<MarketStatusBadge status="UnknownState" />);

    expect(screen.getByText('Unknown')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--neutral');
  });

  it('renders Unknown status when status is null', () => {
    render(<MarketStatusBadge status={null} />);

    expect(screen.getByText('Unknown')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--neutral');
  });

  it('renders Unknown status when status is undefined', () => {
    render(<MarketStatusBadge />);

    expect(screen.getByText('Unknown')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveClass('ui-badge--neutral');
  });

  it('applies custom className', () => {
    render(<MarketStatusBadge status="Active" className="custom-class" />);

    const badge = screen.getByRole('status');
    expect(badge).toHaveClass('custom-class');
    expect(badge).toHaveClass('ui-badge');
  });

  it('includes the status icon as a decorative glyph', () => {
    render(<MarketStatusBadge status="Active" />);

    const icon = screen.getByText('●');
    expect(icon).toBeInTheDocument();
    expect(icon).toHaveAttribute('aria-hidden', 'true');
  });

  it('has correct aria-label for accessibility', () => {
    render(<MarketStatusBadge status="Active" />);

    const badge = screen.getByRole('status');
    expect(badge).toHaveAttribute('aria-label', 'Market is active');
  });

  it('has correct aria-label for unknown status', () => {
    render(<MarketStatusBadge status="UnknownState" />);

    const badge = screen.getByRole('status');
    expect(badge).toHaveAttribute('aria-label', 'Market status is unknown');
  });

  it('handles case-sensitive status values', () => {
    render(<MarketStatusBadge status="active" />);

    expect(screen.getByText('Unknown')).toBeInTheDocument();
  });
});
