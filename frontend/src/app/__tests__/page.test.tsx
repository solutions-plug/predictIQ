import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import Home from '../page';
import DisputePage from '../markets/page';

jest.mock('../../components/LandingPage', () => ({
  LandingPage: () => <div data-testid="landing-page-stub" />,
}));

jest.mock('next/navigation', () => ({
  useRouter: () => ({ back: jest.fn(), push: jest.fn() }),
}));

jest.mock('../../hooks/useDisputeMarket', () => ({
  useDisputeMarket: (id: string) => ({
    market: id ? { id, question: 'Test market' } : null,
    isLoading: false,
    error: null,
    submitDispute: jest.fn(),
  }),
}));

jest.mock('../../components/DisputeForm', () => ({
  DisputeForm: ({ market }: { market: { id: string } }) => (
    <div data-testid="dispute-form-stub">{market.id}</div>
  ),
}));

jest.mock('../../components/LoadingSpinner', () => ({
  LoadingSpinner: () => <div role="status" aria-label="Loading market" />,
}));

jest.mock('../../components/ErrorMessage', () => ({
  ErrorMessage: ({ message }: { message: string }) => (
    <div role="alert">{message}</div>
  ),
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

describe('Dispute page params handling', () => {
  it('awaits a resolved params promise and renders the market id', async () => {
    render(<DisputePage params={Promise.resolve({ id: 'market-123' })} />);

    await waitFor(() => {
      expect(screen.getByTestId('dispute-form-stub')).toHaveTextContent('market-123');
    });
  });

  it('renders a clear message when the id is missing', async () => {
    render(<DisputePage params={Promise.resolve({ id: '' })} />);

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(
        'No market id was provided for this dispute page.'
      );
    });
  });
});
