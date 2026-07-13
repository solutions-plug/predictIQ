'use client';

/**
 * Stellar wallet context backed by the Freighter browser extension (v6 API).
 *
 * Real wallet: connect/disconnect, address, network, and message signing all
 * go through Freighter. Market data is mocked, so `signMessage` is used to have
 * the user genuinely authorise actions (e.g. placing a bet) without needing a
 * live Soroban RPC.
 */

import React from 'react';
import {
  isConnected,
  requestAccess,
  getAddress,
  getNetwork,
  signMessage,
} from '@stellar/freighter-api';

const ADDRESS_KEY = 'predictiq:wallet:address';

interface WalletState {
  address: string | null;
  network: string | null;
  isConnected: boolean;
  isAvailable: boolean; // Freighter extension detected
  isConnecting: boolean;
  error: string | null;
  connect: () => Promise<void>;
  disconnect: () => void;
  /** Prompt the user to sign a short message; returns the signed payload (base64) or null. */
  authorize: (message: string) => Promise<string | null>;
}

const WalletContext = React.createContext<WalletState | null>(null);

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [address, setAddress] = React.useState<string | null>(null);
  const [network, setNetwork] = React.useState<string | null>(null);
  const [isAvailable, setIsAvailable] = React.useState(false);
  const [isConnecting, setIsConnecting] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // Detect Freighter and restore a previously-connected session.
  React.useEffect(() => {
    let active = true;
    (async () => {
      try {
        const conn = await isConnected();
        if (!active) return;
        setIsAvailable(Boolean(conn?.isConnected));
        const saved = window.localStorage.getItem(ADDRESS_KEY);
        if (saved && conn?.isConnected) {
          const res = await getAddress();
          if (active && res?.address) {
            setAddress(res.address);
            const net = await getNetwork();
            if (active) setNetwork(net?.network ?? null);
          }
        }
      } catch {
        if (active) setIsAvailable(false);
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  const connect = React.useCallback(async () => {
    setError(null);
    setIsConnecting(true);
    try {
      const conn = await isConnected();
      if (!conn?.isConnected) {
        throw new Error('Freighter wallet not detected. Install the Freighter extension to connect.');
      }
      const access = await requestAccess();
      if (access?.error) throw new Error(access.error);
      const addr = access?.address || (await getAddress())?.address;
      if (!addr) throw new Error('Could not read wallet address.');
      const net = await getNetwork();
      setAddress(addr);
      setNetwork(net?.network ?? null);
      window.localStorage.setItem(ADDRESS_KEY, addr);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to connect wallet');
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const disconnect = React.useCallback(() => {
    setAddress(null);
    setNetwork(null);
    setError(null);
    window.localStorage.removeItem(ADDRESS_KEY);
  }, []);

  const authorize = React.useCallback(
    async (message: string): Promise<string | null> => {
      if (!address) throw new Error('Connect your wallet first');
      const res = await signMessage(message, { address });
      if (res?.error) throw new Error(String(res.error));
      const signed = res?.signedMessage;
      if (!signed) return null;
      if (typeof signed === 'string') return signed;
      // Bytes -> base64 without relying on Node's Buffer in the browser.
      const bytes = new Uint8Array(signed as unknown as ArrayBufferLike);
      let binary = '';
      for (const byte of bytes) {
        binary += String.fromCharCode(byte);
      }
      return btoa(binary);
    },
    [address],
  );

  const value: WalletState = {
    address,
    network,
    isConnected: Boolean(address),
    isAvailable,
    isConnecting,
    error,
    connect,
    disconnect,
    authorize,
  };

  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
}

export function useWallet(): WalletState {
  const ctx = React.useContext(WalletContext);
  if (!ctx) throw new Error('useWallet must be used within a WalletProvider');
  return ctx;
}
