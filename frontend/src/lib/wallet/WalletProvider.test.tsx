import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { WalletProvider, useWallet, WALLET_NOT_INSTALLED } from './WalletProvider';
import '@testing-library/jest-dom';

const mockFreighterApi = {
  isConnected: jest.fn(),
  requestAccess: jest.fn(),
  setAllowed: jest.fn(),
  getAddress: jest.fn(),
  getPublicKey: jest.fn(),
  getNetwork: jest.fn(),
  getNetworkDetails: jest.fn(),
  signTransaction: jest.fn(),
};

function TestComponent() {
  const wallet = useWallet();
  return (
    <div>
      <div data-testid="address">{wallet.address || 'not connected'}</div>
      <div data-testid="network">{wallet.network || 'no network'}</div>
      <div data-testid="is-connecting">{wallet.isConnecting ? 'connecting' : 'idle'}</div>
      <div data-testid="is-installed">{wallet.isInstalled ? 'installed' : 'not installed'}</div>
      <div data-testid="error">{wallet.error || 'no error'}</div>
      <button onClick={() => wallet.connect()}>Connect</button>
      <button onClick={() => wallet.disconnect()}>Disconnect</button>
    </div>
  );
}

describe('WalletProvider', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    Object.defineProperty(window, 'freighterApi', {
      value: undefined,
      writable: true,
      configurable: true,
    });
    sessionStorage.clear();
  });

  describe('detection', () => {
    it('detects missing Freighter extension', async () => {
      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      await waitFor(() => {
        expect(screen.getByTestId('is-installed')).toHaveTextContent('not installed');
      });
    });

    it('detects installed Freighter extension', async () => {
      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      await waitFor(() => {
        expect(screen.getByTestId('is-installed')).toHaveTextContent('installed');
      });
    });
  });

  describe('connect', () => {
    it('successfully connects with requestAccess', async () => {
      const testAddress = 'GBUQWP3BOUZX34ULNQG23RQ6F4BWFIreachable';
      mockFreighterApi.requestAccess.mockResolvedValue({ address: testAddress });
      mockFreighterApi.getNetworkDetails.mockResolvedValue({ network: 'TESTNET' });

      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      const connectBtn = screen.getByText('Connect');
      fireEvent.click(connectBtn);

      await waitFor(() => {
        expect(screen.getByTestId('address')).toHaveTextContent(testAddress);
        expect(screen.getByTestId('network')).toHaveTextContent('TESTNET');
      });

      expect(sessionStorage.getItem('predictiq.wallet.wasConnected')).toBe('1');
    });

    it('handles requestAccess error', async () => {
      mockFreighterApi.requestAccess.mockResolvedValue({
        error: 'User rejected access',
      });

      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      const connectBtn = screen.getByText('Connect');
      fireEvent.click(connectBtn);

      await waitFor(() => {
        expect(screen.getByTestId('error')).toHaveTextContent('User rejected access');
        expect(screen.getByTestId('address')).toHaveTextContent('not connected');
      });
    });

    it('detects missing extension during connect', async () => {
      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      const connectBtn = screen.getByText('Connect');
      fireEvent.click(connectBtn);

      await waitFor(() => {
        expect(screen.getByTestId('error')).toHaveTextContent(WALLET_NOT_INSTALLED);
        expect(screen.getByTestId('is-installed')).toHaveTextContent('not installed');
      });
    });

    it('shows connecting state during connect', async () => {
      mockFreighterApi.requestAccess.mockImplementation(
        () => new Promise((resolve) => setTimeout(() => resolve({ address: 'test' }), 100))
      );
      mockFreighterApi.getNetworkDetails.mockResolvedValue({ network: 'TESTNET' });

      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      const connectBtn = screen.getByText('Connect');
      fireEvent.click(connectBtn);

      await waitFor(() => {
        expect(screen.getByTestId('is-connecting')).toHaveTextContent('idle');
      });
    });
  });

  describe('disconnect', () => {
    it('clears address and network', async () => {
      const testAddress = 'GBUQWP3BOUZX34ULNQG23RQ6F4BWFITEST';
      mockFreighterApi.requestAccess.mockResolvedValue({ address: testAddress });
      mockFreighterApi.getNetworkDetails.mockResolvedValue({ network: 'TESTNET' });

      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      const connectBtn = screen.getByText('Connect');
      fireEvent.click(connectBtn);

      await waitFor(() => {
        expect(screen.getByTestId('address')).toHaveTextContent(testAddress);
      });

      const disconnectBtn = screen.getByText('Disconnect');
      fireEvent.click(disconnectBtn);

      expect(screen.getByTestId('address')).toHaveTextContent('not connected');
      expect(screen.getByTestId('network')).toHaveTextContent('no network');
      expect(screen.getByTestId('error')).toHaveTextContent('no error');
      expect(sessionStorage.getItem('predictiq.wallet.wasConnected')).toBeNull();
    });
  });

  describe('account polling', () => {
    it('detects address change via polling', async () => {
      const initialAddress = 'GBUQWP3BOUZX34ULNQG23RQ6F4BWFI1';
      const newAddress = 'GBUQWP3BOUZX34ULNQG23RQ6F4BWFI2';

      mockFreighterApi.requestAccess.mockResolvedValue({ address: initialAddress });
      mockFreighterApi.getNetworkDetails.mockResolvedValue({ network: 'TESTNET' });
      mockFreighterApi.getAddress.mockResolvedValue({ address: initialAddress });

      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      const connectBtn = screen.getByText('Connect');
      fireEvent.click(connectBtn);

      await waitFor(() => {
        expect(screen.getByTestId('address')).toHaveTextContent(initialAddress);
      });

      mockFreighterApi.getAddress.mockResolvedValue({ address: newAddress });

      await waitFor(
        () => {
          expect(screen.getByTestId('address')).toHaveTextContent(newAddress);
        },
        { timeout: 5000 }
      );
    });
  });

  describe('session restore', () => {
    it('restores connected state on mount if wasConnected flag is set', async () => {
      const testAddress = 'GBUQWP3BOUZX34ULNQG23RQ6F4BWFIREST';
      sessionStorage.setItem('predictiq.wallet.wasConnected', '1');

      mockFreighterApi.isConnected.mockResolvedValue({ isConnected: true });
      mockFreighterApi.getAddress.mockResolvedValue({ address: testAddress });
      mockFreighterApi.getNetworkDetails.mockResolvedValue({ network: 'TESTNET' });

      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      await waitFor(() => {
        expect(screen.getByTestId('address')).toHaveTextContent(testAddress);
      });
    });

    it('does not restore if wasConnected flag is not set', async () => {
      mockFreighterApi.isConnected.mockResolvedValue({ isConnected: true });
      mockFreighterApi.getAddress.mockResolvedValue({ address: 'GBUQWP3BOUZX34ULNQG23RQ6F4BWFI' });

      Object.defineProperty(window, 'freighterApi', {
        value: mockFreighterApi,
        writable: true,
        configurable: true,
      });

      render(
        <WalletProvider>
          <TestComponent />
        </WalletProvider>
      );

      await waitFor(() => {
        expect(screen.getByTestId('address')).toHaveTextContent('not connected');
      });

      expect(mockFreighterApi.getAddress).not.toHaveBeenCalled();
    });
  });
});
