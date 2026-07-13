import React from 'react';
import { render, screen } from '@testing-library/react';
import type { Market } from '../../../lib/types';
import { MarketsExplorer } from '../MarketsExplorer';
import { listMarkets, getFeaturedMarkets } from '../../../lib/mock/markets';

jest.mock('../../../lib/mock/markets');

jest.mock('next/link', () => ({
  __esModule: true,
  default: ({ href, children }: { href: string; children: React.ReactNode }) => (
    <a href={href}>{children}</a>
  ),
}));

const mockListMarkets = listMarkets as jest.MockedFunction<typeof listMarkets>;
const mockGetFeatured = getFeaturedMarkets as jest.MockedFunction<typeof getFeaturedMarkets>;

function makeMarket(overrides: Partial<Market> = {}): Market {
  return {
    id: 'mkt_test',
    title: 'Will the seed market appear?',
    description: 'A seeded market for the explorer test.',
    category: 'Crypto',
    outcomes: [
      { id: 1, label: 'Yes' },
      { id: 0, label: 'No' },
    ],
    poolByOutcome: { 1: 7000, 0: 3000 },
    totalVolume: 10000,
    endsAt: new Date(Date.now() + 5 * 86_400_000).toISOString(),
    status: 'open',
    resolvedOutcome: null,
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

describe('MarketsExplorer', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders a seeded market from the data layer', async () => {
    const seed = [makeMarket()];
    mockListMarkets.mockResolvedValue(seed);
    mockGetFeatured.mockResolvedValue(seed);

    render(<MarketsExplorer />);

    // Appears in both the featured strip and the main grid.
    const titles = await screen.findAllByText('Will the seed market appear?');
    expect(titles.length).toBeGreaterThan(0);
    expect(mockListMarkets).toHaveBeenCalled();
  });

  it('shows an empty state when no markets match', async () => {
    mockListMarkets.mockResolvedValue([]);
    mockGetFeatured.mockResolvedValue([]);

    render(<MarketsExplorer />);

    expect(await screen.findByText('No markets match your filters')).toBeInTheDocument();
  });
});
