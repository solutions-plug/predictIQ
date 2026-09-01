import React from 'react';
import { render, screen } from '@testing-library/react';
import LandingPage from '../LandingPage';
import { api } from '../../lib/api/public-client';

describe('LandingPage hero CTAs (#1343)', () => {
  beforeEach(() => {
    jest
      .spyOn(api, 'getStatistics')
      .mockResolvedValue({ total_markets: 1, total_volume: 0, active_markets: 0 });
  });
  afterEach(() => jest.restoreAllMocks());

  it('the primary CTA routes into the product at /markets', () => {
    render(<LandingPage />);
    const primary = screen.getByRole('link', { name: /explore markets/i });
    expect(primary).toHaveAttribute('href', '/markets');
  });

  it('the secondary CTA anchors to the "how it works" section', () => {
    render(<LandingPage />);
    const secondary = screen.getByRole('link', { name: /see how it works/i });
    expect(secondary).toHaveAttribute('href', '#how-it-works');
    expect(document.querySelector('#how-it-works')).toBeInTheDocument();
  });

  it('every feature card is a keyboard-reachable link into the product', () => {
    render(<LandingPage />);
    const cards = document.querySelectorAll('.feature-card');
    expect(cards).toHaveLength(3);
    cards.forEach((card) => {
      const link = card.querySelector('a');
      expect(link).toHaveAttribute('href', '/markets');
      expect(link).not.toHaveAttribute('tabindex', '-1');
    });
  });
});
