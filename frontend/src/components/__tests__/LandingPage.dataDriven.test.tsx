import React from 'react';
import { render, screen } from '@testing-library/react';
import LandingPage from '../LandingPage';
import { api } from '../../lib/api/public-client';

describe('LandingPage data-driven sections', () => {
  beforeEach(() => {
    jest
      .spyOn(api, 'getStatistics')
      .mockResolvedValue({ total_markets: 128, total_volume: 45000, active_markets: 512 });
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('renders one feature-card article per feature entry with translated copy', () => {
    render(<LandingPage />);
    const cards = document.querySelectorAll('.feature-card');
    expect(cards).toHaveLength(3);
    expect(screen.getByRole('heading', { name: 'Fully Decentralized' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Secure & Audited' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Lightning Fast' })).toBeInTheDocument();
  });

  it('renders the how-it-works steps as an ordered list with one item per step', () => {
    render(<LandingPage />);
    const steps = document.querySelectorAll('.steps-list > li');
    expect(steps).toHaveLength(4);
    expect(screen.getByRole('heading', { name: 'Create a Market' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Claim Winnings' })).toBeInTheDocument();
  });

  it('renders footer columns, including link lists, from the data array', () => {
    render(<LandingPage />);
    const columns = document.querySelectorAll('.footer-section');
    expect(columns).toHaveLength(3);

    expect(screen.getByRole('link', { name: 'Documentation' })).toHaveAttribute('href', '/docs');
    expect(screen.getByRole('link', { name: 'GitHub' })).toHaveAttribute('href', '/github');
    expect(screen.getByRole('link', { name: 'Discord' })).toHaveAttribute('href', '/discord');
    expect(screen.getByRole('link', { name: 'Privacy Policy' })).toHaveAttribute('href', '/privacy');
    expect(screen.getByRole('link', { name: 'Terms of Service' })).toHaveAttribute('href', '/terms');
  });
});
