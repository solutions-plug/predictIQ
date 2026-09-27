'use client';

import React, { useEffect, useState, useCallback } from 'react';
import './bets.css';

// TODO: replace with the shared wallet hook from #77 once merged. This local
// stub exposes the same shape (connected address + change subscription) so
// swapping it out later is a one-line change.
function useConnectedWallet(): string | null {
  const [address, setAddress] = useState<string | null>(null);

  useEffect(() => {
    const wallet = (typeof window !== 'undefined' ? (window as any).freighterApi : null);
    let cancelled = false;

    const loadAddress = async () => {
      try {
        const result = await wallet?.getAddress?.();
        if (!cancelled) setAddress(result?.address ?? null);
      } catch {
        if (!cancelled) setAddress(null);
      }
    };

    loadAddress();
    wallet?.addEventListener?.('accountChanged', loadAddress);

    return () => {
      cancelled = true;
      wallet?.removeEventListener?.('accountChanged', loadAddress);
    };
  }, []);

  return address;
}

interface Bet {
  id: string;
  market_id: string;
  outcome: string;
  amount: string;
  placed_at: string;
}

const PAGE_SIZE = 20;

const BET_NOT_FOUND_CODE = 142;

export default function BetHistoryPage() {
  const address = useConnectedWallet();
  const [bets, setBets] = useState<Bet[]>([]);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchBets = useCallback(async (userAddress: string, pageIndex: number) => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/v1/blockchain/users/${userAddress}/bets?offset=${pageIndex * PAGE_SIZE}&limit=${PAGE_SIZE}`
      );
      if (res.status === 404 || res.status === BET_NOT_FOUND_CODE) {
        setBets([]);
        return;
      }
      if (!res.ok) throw new Error(`Failed to load bets (${res.status})`);
      const data = await res.json();
      setBets(data.bets ?? []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load bet history');
      setBets([]);
    } finally {
      setLoading(false);
    }
  }, []);

  // Re-fetch whenever the connected address changes, so switching wallet
  // accounts never leaves the previous address's stale list on screen.
  useEffect(() => {
    setPage(0);
    if (address) {
      fetchBets(address, 0);
    } else {
      setBets([]);
    }
  }, [address, fetchBets]);

  useEffect(() => {
    if (address) fetchBets(address, page);
  }, [page, address, fetchBets]);

  if (!address) {
    return (
      <div className="bet-history-page">
        <p className="bet-history-page__hint">Connect your wallet to view your bet history.</p>
      </div>
    );
  }

  return (
    <div className="bet-history-page">
      <h1 className="bet-history-page__heading">Bet History</h1>
      {loading && (
        <p className="bet-history-page__status" role="status">
          Loading bets…
        </p>
      )}
      {error && (
        <p className="bet-history-page__error" role="alert">
          {error}
        </p>
      )}
      {!loading && !error && bets.length === 0 && (
        <p className="bet-history-page__hint">No bets found for this address.</p>
      )}

      {bets.length > 0 && (
        <div className="bet-history-page__table-wrapper">
          <table className="bet-history-page__table">
            <thead>
              <tr>
                <th>Market</th>
                <th>Outcome</th>
                <th>Amount</th>
                <th>Placed</th>
              </tr>
            </thead>
            <tbody>
              {bets.map((bet) => (
                <tr key={bet.id}>
                  <td>{bet.market_id}</td>
                  <td>{bet.outcome}</td>
                  <td>{bet.amount}</td>
                  <td>{new Date(bet.placed_at).toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="bet-history-page__pagination">
        <button
          type="button"
          className="bet-history-page__pagination-btn"
          disabled={page === 0 || loading}
          onClick={() => setPage((p) => Math.max(0, p - 1))}
        >
          Previous
        </button>
        <button
          type="button"
          className="bet-history-page__pagination-btn"
          disabled={loading || bets.length < PAGE_SIZE}
          onClick={() => setPage((p) => p + 1)}
        >
          Next
        </button>
      </div>
    </div>
  );
}
