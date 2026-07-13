import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CreateMarketForm } from '../CreateMarketForm';
import { createMarket } from '../../../lib/mock/markets';

const pushMock = jest.fn();

jest.mock('../../../lib/mock/markets', () => ({
  createMarket: jest.fn().mockResolvedValue({ id: 'mkt_new' }),
}));

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

jest.mock('next/navigation', () => ({
  useRouter: () => ({ push: pushMock }),
}));

/** A date guaranteed to be in the future, as YYYY-MM-DD. */
function futureDate(): string {
  return new Date(Date.now() + 30 * 86_400_000).toISOString().slice(0, 10);
}

describe('CreateMarketForm', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('submits a valid market and redirects to the new market', async () => {
    const user = userEvent.setup();
    render(<CreateMarketForm />);

    await user.type(
      screen.getByLabelText(/title/i),
      'Will ETH flip BTC by 2026?',
    );
    await user.type(
      screen.getByLabelText(/description/i),
      'Resolves YES if ETH market cap exceeds BTC before the end date.',
    );

    const dateInput = screen.getByLabelText(/end date/i);
    await user.clear(dateInput);
    await user.type(dateInput, futureDate());

    await user.click(screen.getByRole('button', { name: /create market/i }));

    await waitFor(() => {
      expect(createMarket).toHaveBeenCalledTimes(1);
    });

    expect(createMarket).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Will ETH flip BTC by 2026?',
        description:
          'Resolves YES if ETH market cap exceeds BTC before the end date.',
        category: 'Crypto',
        outcomes: ['Yes', 'No'],
        createdBy: 'GABC',
        endsAt: expect.stringMatching(/^\d{4}-\d{2}-\d{2}T/),
      }),
    );

    await waitFor(() => {
      expect(pushMock).toHaveBeenCalledWith('/markets/mkt_new');
    });
  });

  test('shows a validation error and does not submit when the title is empty', async () => {
    const user = userEvent.setup();
    render(<CreateMarketForm />);

    await user.type(
      screen.getByLabelText(/description/i),
      'Resolves YES if ETH market cap exceeds BTC before the end date.',
    );
    const dateInput = screen.getByLabelText(/end date/i);
    await user.clear(dateInput);
    await user.type(dateInput, futureDate());

    await user.click(screen.getByRole('button', { name: /create market/i }));

    expect(
      await screen.findByText(/title must be between/i),
    ).toBeInTheDocument();
    expect(createMarket).not.toHaveBeenCalled();
  });
});
