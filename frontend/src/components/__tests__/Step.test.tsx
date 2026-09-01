import React from 'react';
import { render, screen } from '@testing-library/react';
import { Step } from '../landing/Step';

describe('Step', () => {
  it('renders title and description (populated state)', () => {
    render(<Step title="Connect your wallet" description="Link a Stellar wallet to get started." />);

    expect(screen.getByRole('heading', { level: 3, name: 'Connect your wallet' })).toBeInTheDocument();
    expect(screen.getByText('Link a Stellar wallet to get started.')).toBeInTheDocument();
  });

  it('renders as a list item so a sequence of Steps forms a valid <ol>/<ul>', () => {
    render(
      <ul>
        <Step title="Step one" description="Do this first." />
      </ul>
    );
    expect(screen.getByRole('listitem')).toBeInTheDocument();
  });

  it('links the step title to its feature when href is given', () => {
    render(<Step title="Browse markets" description="See open markets." href="/markets" />);
    const link = screen.getByRole('link', { name: 'Browse markets' });
    expect(link).toHaveAttribute('href', '/markets');
    // heading still present and named by the link
    expect(screen.getByRole('heading', { level: 3, name: 'Browse markets' })).toBeInTheDocument();
  });

  it('renders a plain title (no link) when href is omitted', () => {
    render(<Step title="Static step" description="No link." />);
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('does not carry a hardcoded step number (numbering is CSS counter, DOM-order derived)', () => {
    const { container } = render(
      <ol>
        <Step title="First" description="a" />
        <Step title="Second" description="b" />
      </ol>,
    );
    expect(container.textContent).not.toMatch(/\b(1\.|01|Step 1)\b/);
  });
});
