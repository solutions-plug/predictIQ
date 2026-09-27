'use client';

/**
 * Stellar wallet integration (Freighter).
 *
 * Every write flow in the app (bet, create, cancel, resolve) needs a
 * connected wallet's address and a way to get a transaction signed. This
 * provider owns that state so those flows never talk to `window.freighterApi`
 * directly.
 *
 * Freighter injects `window.freighterApi` into the page. It does not emit a
 * DOM event when the user switches accounts or networks in the extension, so
 * we poll its read-only methods at a low frequency while connected — the
 * documented workaround for this gap.
 */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';

/** Minimal shape of the API Freighter injects as `window.freighterApi`. */
interface FreighterApi {
  isConnected?: () => Promise<{ isConnected: boolean } | boolean>;
  requestAccess?: () => Promise<{ address: string } | { publicKey: string } | { error: string }>;
  setAllowed?: () => Promise<unknown>;
  getAddress?: () => Promise<{ address: string } | { error: string }>;
  getPublicKey?: () => Promise<string>;
  getNetwork?: () => Promise<{ network: string } | string>;
  getNetworkDetails?: () => Promise<{ network: string; networkPassphrase?: string }>;
  signTransaction: (
    xdr: string,
    opts?: { address?: string; network?: string; networkPassphrase?: string }
  ) => Promise<{ signedTxXdr: string } | string>;
}

declare global {
  interface Window {
    freighterApi?: FreighterApi;
  }
}

/** Distinct from "not connected" — surfaced by the UI as an install prompt. */
export const WALLET_NOT_INSTALLED = 'WALLET_NOT_INSTALLED';

export interface WalletContextValue {
  /** Connected public key/address, or null when no wallet is connected. */
  address: string | null;
  /** Active Stellar network reported by the wallet (e.g. "TESTNET"), or null. */
  network: string | null;
  /** True while a connect() call is in flight. */
  isConnecting: boolean;
  /** False when the Freighter browser extension is not detected at all. */
  isInstalled: boolean;
  /** Last connect/sign error message, or WALLET_NOT_INSTALLED. Cleared on next attempt. */
  error: string | null;
  /** Prompts the extension's connect UI and stores the resulting address. */
  connect: () => Promise<void>;
  /** Clears local connection state. Does not revoke extension-level access; the site remains authorized in Freighter. */
  disconnect: () => void;
  /** Signs an unsigned transaction XDR with the connected wallet. */
  signAndSubmit: (transactionXdr: string) => Promise<{ signedTxXdr: string }>;
}

const WalletContext = createContext<WalletContextValue | null>(null);

const ACCOUNT_POLL_INTERVAL_MS = 3000;
const SESSION_STORAGE_KEY = 'predictiq.wallet.wasConnected';

function getFreighter(): FreighterApi | undefined {
  return typeof window === 'undefined' ? undefined : window.freighterApi;
}

async function readAddress(api: FreighterApi): Promise<string | null> {
  try {
    if (api.getAddress) {
      const res = await api.getAddress();
      return 'address' in res ? res.address : null;
    }
    if (api.getPublicKey) {
      return (await api.getPublicKey()) || null;
    }
  } catch {
    // Extension not unlocked / access not yet granted — treat as disconnected.
  }
  return null;
}

async function readNetwork(api: FreighterApi): Promise<string | null> {
  try {
    if (api.getNetworkDetails) {
      const res = await api.getNetworkDetails();
      return res?.network ?? null;
    }
    if (api.getNetwork) {
      const res = await api.getNetwork();
      return typeof res === 'string' ? res : res?.network ?? null;
    }
  } catch {
    // Ignore — network becomes known again on the next successful read.
  }
  return null;
}

export function WalletProvider({ children }: { children: ReactNode }) {
  const [address, setAddress] = useState<string | null>(null);
  const [network, setNetwork] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isInstalled, setIsInstalled] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const disconnect = useCallback(() => {
    setAddress(null);
    setNetwork(null);
    setError(null);
    if (typeof window !== 'undefined') {
      window.sessionStorage.removeItem(SESSION_STORAGE_KEY);
    }
  }, []);

  const connect = useCallback(async () => {
    const api = getFreighter();
    if (!api) {
      setIsInstalled(false);
      setError(WALLET_NOT_INSTALLED);
      return;
    }

    setIsConnecting(true);
    setError(null);
    try {
      let nextAddress: string | null = null;

      if (api.requestAccess) {
        const res = await api.requestAccess();
        if ('error' in res) throw new Error(res.error);
        nextAddress = 'address' in res ? res.address : res.publicKey;
      } else {
        if (api.setAllowed) await api.setAllowed();
        nextAddress = await readAddress(api);
      }

      if (!nextAddress) {
        throw new Error('The wallet did not return an address. Please try again.');
      }

      setAddress(nextAddress);
      setNetwork(await readNetwork(api));
      if (typeof window !== 'undefined') {
        window.sessionStorage.setItem(SESSION_STORAGE_KEY, '1');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect wallet.');
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const signAndSubmit = useCallback(
    async (transactionXdr: string): Promise<{ signedTxXdr: string }> => {
      const api = getFreighter();
      if (!api) {
        throw new Error(WALLET_NOT_INSTALLED);
      }
      if (!address) {
        throw new Error('Connect a wallet before signing a transaction.');
      }

      const result = await api.signTransaction(transactionXdr, {
        address,
        network: network ?? undefined,
      });
      const signedTxXdr = typeof result === 'string' ? result : result.signedTxXdr;
      if (!signedTxXdr) {
        throw new Error('The wallet did not return a signed transaction.');
      }
      return { signedTxXdr };
    },
    [address, network]
  );

  // Detect the extension on mount and silently restore a prior session —
  // Freighter keeps its own authorization, we only remember whether to ask.
  useEffect(() => {
    let cancelled = false;

    (async () => {
      const api = getFreighter();
      if (!api) {
        setIsInstalled(false);
        return;
      }
      setIsInstalled(true);

      const wasConnected =
        typeof window !== 'undefined' && window.sessionStorage.getItem(SESSION_STORAGE_KEY) === '1';
      if (!wasConnected) return;

      try {
        const connected = api.isConnected ? await api.isConnected() : true;
        const isConnected = typeof connected === 'boolean' ? connected : connected.isConnected;
        if (!isConnected || cancelled) return;

        const nextAddress = await readAddress(api);
        if (cancelled || !nextAddress) return;
        setAddress(nextAddress);
        setNetwork(await readNetwork(api));
      } catch {
        // Leave disconnected — the user can reconnect explicitly.
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // Poll for account/network changes made in the extension while connected.
  // Pause polling while the tab is hidden to conserve battery/CPU.
  useEffect(() => {
    if (!address) return undefined;

    const startPolling = () => {
      pollRef.current = setInterval(async () => {
        const api = getFreighter();
        if (!api) {
          setIsInstalled(false);
          return;
        }
        const [nextAddress, nextNetwork] = await Promise.all([readAddress(api), readNetwork(api)]);
        setAddress((prev) => (nextAddress !== prev ? nextAddress : prev));
        setNetwork((prev) => (nextNetwork !== prev ? nextNetwork : prev));
      }, ACCOUNT_POLL_INTERVAL_MS);
    };

    const stopPolling = () => {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
    };

    const handleVisibilityChange = () => {
      if (document.hidden) {
        stopPolling();
      } else {
        startPolling();
      }
    };

    startPolling();
    document.addEventListener('visibilitychange', handleVisibilityChange);

    return () => {
      stopPolling();
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [address]);

  const value: WalletContextValue = {
    address,
    network,
    isConnecting,
    isInstalled,
    error,
    connect,
    disconnect,
    signAndSubmit,
  };

  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
}

export function useWallet(): WalletContextValue {
  const ctx = useContext(WalletContext);
  if (!ctx) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return ctx;
}
