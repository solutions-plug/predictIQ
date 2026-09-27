import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { PlaceBetForm, type BetMarket } from '../PlaceBetForm';
import { ApiError } from '../../../lib/api/public-client';

// ---------------------------------------------------------------------------
// Module mocks — no network traffic
// ---------------------------------------------------------------------------

const mockConnect = jest.fn();
const mockWallet = {
  address: null as string | null,
  network: 'TESTNET' as string | null,
  isConnecting: false,
  isInstalled: true,
  error: null as string | null,
  connect: mockConnect,
  disconnect: jest.fn(),
  signAndSubmit: jest.fn(),
};

jest.mock('../../../lib/wallet/WalletProvider', () => ({
  ...jest.requireActual('../../../lib/wallet/WalletProvider'),
  useWallet: () => mockWallet,
}));

const mockPlaceBet = jest.fn();
const mockGetTransactionStatus = jest.fn();
jest.mock('../../../lib/api/public-client', () => {
  const actual = jest.requireActual('../../../lib/api/public-client');
  return {
    ...actual,
    api: {
      ...actual.api,
      placeBet: (...args: unknown[]) => mockPlaceBet(...args),
      getTransactionStatus: (...args: unknown[]) => mockGetTransactionStatus(...args),
    },
  };
});

const mockFetchNativeBalance = jest.fn();
jest.mock('../../../lib/wallet/balance', () => ({
  fetchNativeBalance: (...args: unknown[]) => mockFetchNativeBalance(...args),
}));

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

const MARKET: BetMarket = {
  id: 42,
  title: 'Will it rain tomorrow?',
  outcomes: [
    { index: 0, label: 'Yes' },
    { index: 1, label: 'No' },
  ],
};

const WALLET_ADDRESS = 'GABC1234TESTADDRESS';

function renderForm(onSubmitted?: jest.Mock) {
  return render(<PlaceBetForm market={MARKET} onSubmitted={onSubmitted} />);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function setWalletConnected(address = WALLET_ADDRESS) {
  mockWallet.address = address;
}

function setWalletDisconnected() {
  mockWallet.address = null;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('PlaceBetForm', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    setWalletDisconnected();
    mockWallet.isConnecting = false;
    mockWallet.error = null;
  });

  // -------------------------------------------------------------------------
  // AC 1: Submitting with no wallet triggers connect()
  // -------------------------------------------------------------------------
  describe('wallet not connected', () => {
    it('calls wallet.connect() when the form is submitted without a connected wallet', async () => {
      mockConnect.mockResolvedValue(undefined);
      renderForm();

      const submitButton = screen.getByRole('button', { name: /connect wallet/i });
      fireEvent.click(submitButton);

      await waitFor(() => {
        expect(mockConnect).toHaveBeenCalledTimes(1);
      });
    });

    it('does not attempt to place a bet when wallet is not connected', async () => {
      mockConnect.mockResolvedValue(undefined);
      renderForm();

      fireEvent.click(screen.getByRole('button', { name: /connect wallet/i }));

      await waitFor(() => {
        expect(mockConnect).toHaveBeenCalled();
      });
      expect(mockPlaceBet).not.toHaveBeenCalled();
    });

    it('shows a connect-wallet hint when no wallet is present', () => {
      renderForm();
      expect(screen.getByText(/connect your wallet to place a bet/i)).toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // AC 2: Client-side validation errors
  // -------------------------------------------------------------------------
  describe('client-side validation', () => {
    beforeEach(() => {
      setWalletConnected();
    });

    it('shows a validation error when amount is missing', async () => {
      renderForm();

      // Deselect outcome so only the amount path is tested
      // (The component pre-selects the first outcome, so we submit with empty amount)
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent(/enter an amount greater than 0/i);
      });
      expect(mockPlaceBet).not.toHaveBeenCalled();
    });

    it('shows a validation error when amount is zero', async () => {
      renderForm();

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '0' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent(/enter an amount greater than 0/i);
      });
      expect(mockPlaceBet).not.toHaveBeenCalled();
    });

    it('shows a validation error when amount is negative', async () => {
      renderForm();

      const amountInput = screen.getByRole('spinbutton');
      // Set value directly on the DOM node to bypass HTML5 number-input
      // sanitisation so the component's own validate() path is exercised.
      Object.defineProperty(amountInput, 'value', { get: () => '-5', configurable: true });
      fireEvent.change(amountInput, { target: { value: '-5' } });
      // Submit the form element directly to bypass native constraint validation in jsdom.
      fireEvent.submit(amountInput.closest('form')!);

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent(/enter an amount greater than 0/i);
      });
      expect(mockPlaceBet).not.toHaveBeenCalled();
    });

    it('shows a validation error when amount is not a number', async () => {
      renderForm();

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: 'abc' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent(/enter an amount greater than 0/i);
      });
      expect(mockPlaceBet).not.toHaveBeenCalled();
    });

    it('clears a previous validation error on a new submit attempt', async () => {
      mockPlaceBet.mockResolvedValue({ tx_hash: 'abc123', status: 'pending' });
      renderForm();

      const amountInput = screen.getByRole('spinbutton');

      // First: trigger error
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));
      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeInTheDocument();
      });

      // Then: fix and resubmit
      fireEvent.change(amountInput, { target: { value: '10' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(screen.queryByRole('alert')).not.toBeInTheDocument();
      });
    });
  });

  // -------------------------------------------------------------------------
  // AC 3: Successful submit shows tx status and calls onSubmitted
  // -------------------------------------------------------------------------
  describe('successful bet submission', () => {
    beforeEach(() => {
      setWalletConnected();
    });

    it('calls api.placeBet with correct arguments and shows the tx status', async () => {
      const TX_HASH = 'abcdef1234567890';
      mockPlaceBet.mockResolvedValue({ tx_hash: TX_HASH, status: 'pending' });
      const onSubmitted = jest.fn();

      renderForm(onSubmitted);

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '25' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(mockPlaceBet).toHaveBeenCalledWith(
          MARKET.id,
          { wallet: WALLET_ADDRESS, outcome: 0, amount: '25' }
        );
      });

      await waitFor(() => {
        expect(screen.getByRole('status')).toHaveTextContent(/bet submitted/i);
        expect(screen.getByRole('status')).toHaveTextContent(TX_HASH.slice(0, 10));
      });
    });

    it('calls onSubmitted callback with txHash, outcome, and amount', async () => {
      const TX_HASH = 'deadbeef12345678';
      mockPlaceBet.mockResolvedValue({ tx_hash: TX_HASH, status: 'pending' });
      const onSubmitted = jest.fn();

      renderForm(onSubmitted);

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '50' } });

      // Select the second outcome ("No")
      fireEvent.click(screen.getByLabelText('No'));
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(onSubmitted).toHaveBeenCalledWith({
          txHash: TX_HASH,
          outcome: 1,
          amount: '50',
        });
      });
    });

    it('works without an onSubmitted prop (optional callback)', async () => {
      mockPlaceBet.mockResolvedValue({ tx_hash: 'txhash001', status: 'pending' });

      renderForm(); // no onSubmitted

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '10' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(screen.getByRole('status')).toHaveTextContent(/bet submitted/i);
      });
    });

    it('shows a generic API error message when placeBet throws a non-balance ApiError', async () => {
      mockPlaceBet.mockRejectedValue(
        new ApiError('Market is already closed.', 422, 'MARKET_CLOSED')
      );

      renderForm();

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '10' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent(/market is already closed/i);
      });
    });
  });

  // -------------------------------------------------------------------------
  // AC 4: Insufficient-balance error renders fresh balance message
  // -------------------------------------------------------------------------
  describe('insufficient balance error path', () => {
    beforeEach(() => {
      setWalletConnected();
    });

    it('calls fetchNativeBalance and shows fresh balance in the error message', async () => {
      // ApiError whose details carry the Soroban InsufficientBalance contract code (107)
      const insufficientBalanceError = new ApiError(
        'Insufficient balance to complete this transaction.',
        400,
        'CONTRACT_ERROR',
        { contract_code: 107 }
      );
      mockPlaceBet.mockRejectedValue(insufficientBalanceError);
      mockFetchNativeBalance.mockResolvedValue('12.5000000');

      renderForm();

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '100' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(mockFetchNativeBalance).toHaveBeenCalledWith(WALLET_ADDRESS, 'TESTNET');
      });

      await waitFor(() => {
        const alert = screen.getByRole('alert');
        expect(alert).toHaveTextContent(/insufficient balance to complete this transaction/i);
        expect(alert).toHaveTextContent(/12\.5000000/);
        expect(alert).toHaveTextContent(/100 XLM/);
      });
    });

    it('shows "unavailable" when fetchNativeBalance returns null', async () => {
      const insufficientBalanceError = new ApiError(
        'Insufficient balance to complete this transaction.',
        400,
        'CONTRACT_ERROR',
        { contract_code: 107 }
      );
      mockPlaceBet.mockRejectedValue(insufficientBalanceError);
      mockFetchNativeBalance.mockResolvedValue(null);

      renderForm();

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '200' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        const alert = screen.getByRole('alert');
        expect(alert).toHaveTextContent(/insufficient balance to complete this transaction/i);
        expect(alert).toHaveTextContent(/unavailable/i);
      });
    });

    it('does not call fetchNativeBalance for unrelated API errors', async () => {
      mockPlaceBet.mockRejectedValue(
        new ApiError('Some other error', 500, 'SERVER_ERROR')
      );

      renderForm();

      const amountInput = screen.getByRole('spinbutton');
      fireEvent.change(amountInput, { target: { value: '10' } });
      fireEvent.click(screen.getByRole('button', { name: /place bet/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent(/some other error/i);
      });
      expect(mockFetchNativeBalance).not.toHaveBeenCalled();
    });
  });

  // -------------------------------------------------------------------------
  // General rendering
  // -------------------------------------------------------------------------
  describe('rendering', () => {
    it('renders all market outcomes as radio buttons', () => {
      setWalletConnected();
      renderForm();

      expect(screen.getByLabelText('Yes')).toBeInTheDocument();
      expect(screen.getByLabelText('No')).toBeInTheDocument();
    });

    it('pre-selects the first outcome on mount', () => {
      setWalletConnected();
      renderForm();

      expect(screen.getByLabelText<HTMLInputElement>('Yes').checked).toBe(true);
      expect(screen.getByLabelText<HTMLInputElement>('No').checked).toBe(false);
    });
  });
});
