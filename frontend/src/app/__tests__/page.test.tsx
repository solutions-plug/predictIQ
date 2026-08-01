import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import Home from '../page';

jest.mock('../../components/LandingPage', () => ({
  LandingPage: () => <div data-testid="landing-page-stub" />,
}));

describe('Home page dynamic loading fallback', () => {
  it('renders exactly one loading indicator with a consistent aria-label while the dynamic import resolves', async () => {
    render(<Home />);

    const indicators = screen.getAllByRole('status');
    expect(indicators).toHaveLength(1);
    expect(indicators[0]).toHaveAttribute('aria-label', 'Loading page');
    expect(indicators[0]).toHaveAttribute('aria-live', 'polite');

    await waitFor(() => {
      expect(screen.getByTestId('landing-page-stub')).toBeInTheDocument();
    });

    expect(screen.queryAllByRole('status')).toHaveLength(0);
  });
});
