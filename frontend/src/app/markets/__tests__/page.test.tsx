import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import MarketsPage from '../page';
import { api } from '../../../lib/api/public-client';

const push = jest.fn();
let searchParams = new URLSearchParams();

jest.mock('next/navigation', () => ({
  useRouter: () => ({ push }),
  useSearchParams: () => searchParams,
}));
jest.mock('../../../lib/api/public-client', () => ({
  api: { getFeaturedMarkets: jest.fn() },
}));
jest.mock('../markets.css', () => ({}), { virtual: true });

const getMarkets = api.getFeaturedMarkets as jest.Mock;

const makeMarkets = (n: number) =>
  Array.from({ length: n }, (_, i) => ({
    id: i + 1,
    title: `Market ${i + 1}`,
    volume: 100,
    ends_at: '2999-01-01T00:00:00Z',
    resolved_outcome: null,
  }));

describe('MarketsPage', () => {
  beforeEach(() => {
    push.mockClear();
    getMarkets.mockReset();
    searchParams = new URLSearchParams();
  });

  it('requests the next page via the page query param', async () => {
    getMarkets.mockResolvedValue(makeMarkets(13));
    render(<MarketsPage />);
    await screen.findByText('Market 1');
    expect(screen.queryByText('Market 13')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(push).toHaveBeenCalledWith('/markets?page=2', { scroll: false });
  });

  it('renders the new results for page 2', async () => {
    searchParams = new URLSearchParams('page=2');
    getMarkets.mockResolvedValue(makeMarkets(13));
    render(<MarketsPage />);
    expect(await screen.findByText('Market 13')).toBeInTheDocument();
    expect(screen.queryByText('Market 1')).not.toBeInTheDocument();
    expect(screen.getByText('Page 2 of 2')).toBeInTheDocument();
  });

  it('renders the error state when the fetch fails', async () => {
    getMarkets.mockRejectedValue(new Error('network'));
    render(<MarketsPage />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to load markets');
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('renders the empty message when there are no markets', async () => {
    getMarkets.mockResolvedValue([]);
    render(<MarketsPage />);
    await waitFor(() =>
      expect(screen.getByText('No markets found for the current filters.')).toBeInTheDocument(),
    );
  });
});
