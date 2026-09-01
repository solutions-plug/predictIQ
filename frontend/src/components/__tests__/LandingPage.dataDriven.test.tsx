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
    expect(screen.getByRole('heading', { name: 'Multi-outcome markets' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Hybrid oracle + community resolution' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Stellar speed, with referrals' })).toBeInTheDocument();
  });

  it('renders the how-it-works steps as an ordered list with one item per step', () => {
    render(<LandingPage />);
    const list = document.querySelector('ol.steps-list');
    expect(list).toBeInTheDocument();
    const steps = document.querySelectorAll('.steps-list > li');
    expect(steps).toHaveLength(4);
    // Real product flow.
    expect(screen.getByRole('heading', { name: 'Connect your wallet' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Claim your payout' })).toBeInTheDocument();
  });

  it('each step links to a real feature route', () => {
    render(<LandingPage />);
    const stepLinks = document.querySelectorAll('.steps-list > li a');
    expect(stepLinks).toHaveLength(4);
    stepLinks.forEach((a) => {
      expect(a.getAttribute('href')).toMatch(/^\/(markets|account)/);
    });
  });

  it('footer columns route to pages that exist - no dead links', () => {
    render(<LandingPage />);
    const columns = document.querySelectorAll('.footer-section');
    expect(columns).toHaveLength(4); // brand + Product + Resources + newsletter

    expect(screen.getByRole('link', { name: 'Markets' })).toHaveAttribute('href', '/markets');
    expect(screen.getByRole('link', { name: 'Statistics' })).toHaveAttribute('href', '/statistics');
    expect(screen.getByRole('link', { name: 'Create a market' })).toHaveAttribute(
      'href',
      '/markets/create',
    );

    const footer = screen.getByRole('contentinfo');
    footer.querySelectorAll('a[href^="/"]').forEach((a) => {
      // every internal footer link points at a known app route
      expect(a.getAttribute('href')).toMatch(
        /^\/(markets|statistics|markets\/create|account\/bets|main-content|#)/,
      );
    });
    // No links to the removed pages.
    expect(screen.queryByRole('link', { name: 'Privacy Policy' })).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'Terms of Service' })).not.toBeInTheDocument();

    // External resources open safely in a new tab.
    const docs = screen.getByRole('link', { name: 'Documentation' });
    expect(docs).toHaveAttribute('href', expect.stringContaining('github.com'));
    expect(docs).toHaveAttribute('rel', 'noopener noreferrer');
  });
});
