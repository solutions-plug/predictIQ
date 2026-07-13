import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import type { Bet, Market } from '../../../lib/types';
import { PortfolioView } from '../PortfolioView';
import {
  getUserBets,
  getMarket,
  claimWinnings,
} from '../../../lib/mock/markets';

jest.mock('../../../lib/mock/markets');

// Mutable wallet state so individual tests can flip isConnected.
const mockWallet = {
  isConnected: true,
  address: 'GABC',
  connect: jest.fn(),
  authorize: jest.fn().mockResolvedValue('sig'),
  isConnecting: false,
  error: null as string | null,
};

jest.mock('../../../lib/wallet/WalletProvider', () => ({
  useWallet: () => mockWallet,
}));

jest.mock('next/link', () => ({
  __esModule: true,
  default: ({ children, href }: { children: React.ReactNode; href: string }) => (
    <a href={href}>{children}</a>
  ),
}));

const mockedGetUserBets = getUserBets as jest.MockedFunction<typeof getUserBets>;
const mockedGetMarket = getMarket as jest.MockedFunction<typeof getMarket>;
const mockedClaimWinnings = claimWinnings as jest.MockedFunction<typeof claimWinnings>;

const resolvedMarket: Market = {
  id: 'mkt_1',
  title: 'Will BTC close above $100k?',
  description: 'desc',
  category: 'Crypto',
  outcomes: [
    { id: 1, label: 'Yes' },
    { id: 0, label: 'No' },
  ],
  poolByOutcome: { 1: 9000, 0: 3000 },
  totalVolume: 12000,
  endsAt: new Date().toISOString(),
  status: 'resolved',
  resolvedOutcome: 1,
  createdAt: new Date().toISOString(),
};

const winningBet: Bet = {
  id: 'bet_1',
  marketId: 'mkt_1',
  outcomeId: 1,
  amount: 500,
  user: 'GABC',
  placedAt: new Date().toISOString(),
  claimed: false,
  payout: 666,
};

beforeEach(() => {
  jest.clearAllMocks();
  mockWallet.isConnected = true;
  mockWallet.address = 'GABC';
  mockWallet.authorize = jest.fn().mockResolvedValue('sig');
});

describe('PortfolioView (connected)', () => {
  beforeEach(() => {
    mockedGetUserBets.mockResolvedValue([winningBet]);
    mockedGetMarket.mockResolvedValue(resolvedMarket);
    mockedClaimWinnings.mockResolvedValue({ ...winningBet, claimed: true });
  });

  it('renders the user positions with market title and backed outcome', async () => {
    render(<PortfolioView />);

    expect(await screen.findByText('Will BTC close above $100k?')).toBeInTheDocument();
    expect(screen.getByText(/Backed: Yes/)).toBeInTheDocument();
  });

  it('claims a winning, unclaimed bet', async () => {
    render(<PortfolioView />);

    const claimButton = await screen.findByRole('button', { name: /claim/i });
    fireEvent.click(claimButton);

    await waitFor(() => {
      expect(mockedClaimWinnings).toHaveBeenCalledWith('bet_1');
    });
    expect(mockWallet.authorize).toHaveBeenCalled();
    expect(await screen.findByText(/Claimed/)).toBeInTheDocument();
  });
});

describe('PortfolioView (not connected)', () => {
  beforeEach(() => {
    mockWallet.isConnected = false;
    mockWallet.address = '';
  });

  it('shows the connect prompt', async () => {
    render(<PortfolioView />);

    expect(
      await screen.findByText(/Connect your wallet to view your positions/i),
    ).toBeInTheDocument();
    expect(mockedGetUserBets).not.toHaveBeenCalled();
  });
});
