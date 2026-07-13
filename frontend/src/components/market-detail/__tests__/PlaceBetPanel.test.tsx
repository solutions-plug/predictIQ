import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { PlaceBetPanel } from '../PlaceBetPanel';
import { placeBet } from '../../../lib/mock/markets';
import type { Bet, Market } from '../../../lib/types';

// --- Mocks -----------------------------------------------------------------

jest.mock('../../../lib/mock/markets');

jest.mock('../../../lib/wallet/WalletProvider', () => ({
  useWallet: () => ({
    isConnected: true,
    address: 'GABC',
    connect: jest.fn(),
    authorize: jest.fn().mockResolvedValue('sig'),
    isConnecting: false,
    error: null,
  }),
}));

const mockedPlaceBet = placeBet as jest.MockedFunction<typeof placeBet>;

// --- Fixtures --------------------------------------------------------------

function seedMarket(): Market {
  return {
    id: 'mkt_test',
    title: 'Will it rain tomorrow?',
    description: 'A seed market for tests.',
    category: 'Technology',
    outcomes: [
      { id: 1, label: 'Yes' },
      { id: 0, label: 'No' },
    ],
    poolByOutcome: { 1: 6000, 0: 4000 },
    totalVolume: 10000,
    endsAt: new Date(Date.now() + 86_400_000).toISOString(),
    status: 'open',
    resolvedOutcome: null,
    createdAt: new Date().toISOString(),
  };
}

function seedBet(overrides: Partial<Bet> = {}): Bet {
  return {
    id: 'bet_1',
    marketId: 'mkt_test',
    outcomeId: 0,
    amount: 50,
    user: 'GABC',
    placedAt: new Date().toISOString(),
    claimed: false,
    payout: null,
    txHash: 'tx_seed_hash',
    ...overrides,
  };
}

beforeEach(() => {
  jest.clearAllMocks();
});

// --- Tests -----------------------------------------------------------------

test('places a bet with the selected outcome and amount, then shows confirmation', async () => {
  // Arrange
  const user = userEvent.setup();
  const market = seedMarket();
  const onBetPlaced = jest.fn();
  mockedPlaceBet.mockResolvedValue(seedBet({ outcomeId: 0, amount: 50 }));

  render(<PlaceBetPanel market={market} onBetPlaced={onBetPlaced} />);

  // Act — pick the "No" outcome (id 0), enter a stake, submit.
  await user.click(screen.getByRole('button', { name: /No/ }));
  await user.type(screen.getByLabelText(/Stake/i), '50');
  await user.click(screen.getByRole('button', { name: /Place Bet/i }));

  // Assert — placeBet called with the right marketId / outcomeId / amount.
  await waitFor(() => {
    expect(mockedPlaceBet).toHaveBeenCalledTimes(1);
  });
  expect(mockedPlaceBet).toHaveBeenCalledWith(
    expect.objectContaining({
      marketId: 'mkt_test',
      outcomeId: 0,
      amount: 50,
      user: 'GABC',
    }),
  );

  // Success confirmation surfaces and the parent is asked to refresh.
  expect(await screen.findByText(/Bet placed/i)).toBeInTheDocument();
  expect(onBetPlaced).toHaveBeenCalledTimes(1);
});
