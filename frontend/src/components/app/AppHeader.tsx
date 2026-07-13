'use client';

import React from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useWallet } from '../../lib/wallet/WalletProvider';
import { useDarkMode } from '../../lib/hooks/useDarkMode';
import { shortAddress } from '../../lib/format';
import { Button } from '../ui/Button';

const NAV = [
  { href: '/markets', label: 'Markets' },
  { href: '/markets/create', label: 'Create' },
  { href: '/portfolio', label: 'Portfolio' },
];

export function AppHeader() {
  const pathname = usePathname();
  const { isDarkMode, toggleDarkMode } = useDarkMode();
  const { isConnected, address, isConnecting, connect, disconnect, error } = useWallet();

  return (
    <header className="app-header" role="banner">
      <div className="app-header__inner app-container">
        <Link href="/" className="logo" aria-label="PredictIQ home">
          <img src="/mark.svg" alt="PredictIQ Logo" width={34} height={34} />
          <span className="logo-text" aria-hidden="true">
            PredictIQ
          </span>
        </Link>

        <nav className="app-nav" aria-label="Primary">
          {NAV.map((item) => {
            // Active = the most specific nav href that prefixes the current path,
            // so /markets/create highlights "Create", not "Markets".
            const matches = NAV.filter(
              (n) => pathname === n.href || pathname.startsWith(`${n.href}/`),
            );
            const best = matches.sort((a, b) => b.href.length - a.href.length)[0];
            const active = best?.href === item.href;
            return (
              <Link
                key={item.href}
                href={item.href}
                className={active ? 'app-nav__link is-active' : 'app-nav__link'}
                aria-current={active ? 'page' : undefined}
              >
                {item.label}
              </Link>
            );
          })}
        </nav>

        <div className="app-header__controls">
          <button
            type="button"
            onClick={toggleDarkMode}
            className="dark-mode-toggle"
            aria-label={isDarkMode ? 'Switch to light mode' : 'Switch to dark mode'}
            title={isDarkMode ? 'Light mode' : 'Dark mode'}
          >
            {isDarkMode ? '☀️' : '🌙'}
          </button>

          {isConnected && address ? (
            <div className="wallet-chip">
              <span className="wallet-chip__dot" aria-hidden="true" />
              <span className="wallet-chip__addr" title={address}>
                {shortAddress(address)}
              </span>
              <button
                type="button"
                className="wallet-chip__disconnect"
                onClick={disconnect}
                aria-label="Disconnect wallet"
              >
                ×
              </button>
            </div>
          ) : (
            <Button
              size="sm"
              onClick={connect}
              loading={isConnecting}
              title={error ?? undefined}
            >
              Connect Wallet
            </Button>
          )}
        </div>
      </div>
      {error && (
        <p className="app-header__error app-container" role="alert">
          {error}
        </p>
      )}
    </header>
  );
}
