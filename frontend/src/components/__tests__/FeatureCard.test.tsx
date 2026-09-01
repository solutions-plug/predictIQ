import React from 'react';
import { render, screen } from '@testing-library/react';
import { FeatureCard } from '../landing/FeatureCard';

describe('FeatureCard', () => {
  it('renders title, description, and icon (populated state)', () => {
    const { container } = render(
      <FeatureCard
        icon="/icons/rocket.svg"
        title="Fast markets"
        description="Trade on outcomes in seconds."
      />
    );

    expect(screen.getByRole('heading', { level: 3, name: 'Fast markets' })).toBeInTheDocument();
    expect(screen.getByText('Trade on outcomes in seconds.')).toBeInTheDocument();

    const icon = container.querySelector('img');
    expect(icon).toHaveAttribute('src', '/icons/rocket.svg');
  });

  it('marks the icon decorative (empty alt text) so screen readers skip it', () => {
    // An <img alt=""> is exposed with role "presentation", not "img" — querying
    // by role would silently miss a regression that dropped alt="", so assert
    // directly on the element instead.
    const { container } = render(<FeatureCard icon="/icons/lock.svg" title="Secure" description="Audited contracts." />);

    const icon = container.querySelector('img');
    expect(icon).toHaveAttribute('alt', '');
    expect(icon).toHaveAttribute('aria-hidden', 'true');
  });

  it('renders as an article landmark so multiple cards are distinguishable', () => {
    render(<FeatureCard icon="/icons/chart.svg" title="Live data" description="Real-time odds." />);
    expect(screen.getByRole('article')).toBeInTheDocument();
  });

  it('without href, the card is not a link', () => {
    render(<FeatureCard icon="/i.svg" title="Plain" description="No link." />);
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('with href, the whole card is a keyboard-reachable link with an accessible name', () => {
    render(
      <FeatureCard icon="/i.svg" title="Oracle resolution" description="Pyth + Reflector." href="/markets" />,
    );
    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/markets');
    // Anchors are tab-focusable by default (no tabindex needed); assert it isn't
    // removed from the tab order.
    expect(link).not.toHaveAttribute('tabindex', '-1');
    expect(link).toHaveAccessibleName(/oracle resolution/i);
  });
});
