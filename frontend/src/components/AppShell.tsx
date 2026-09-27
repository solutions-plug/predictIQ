'use client';

/**
 * AppShell — primary app-wide navigation (#1314).
 *
 * The landing page (`/`) already owns its own full marketing header/nav/
 * footer (components/LandingPage.tsx — separate anchor-link nav for
 * #features/#how-it-works/#about/#contact), so AppShell skips rendering
 * on `/` to avoid a duplicate header there. Everywhere else (Markets,
 * Statistics, Create Market, account, tx, and — conditionally, once an
 * admin session exists — Admin) gets a persistent header with primary
 * navigation and a minimal footer, matching the sub-nav pattern already
 * established by app/admin/layout.tsx for its own section.
 *
 * Styled entirely via classNames (src/styles/ui.css), not inline `style`
 * props: this app's CSP sends `style-src 'self'` with no `unsafe-inline`
 * and no style nonce, so an inline style attribute is silently dropped by
 * the browser rather than applied.
 */

import React, { useEffect, useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';

const NAV_ITEMS = [
  { href: '/markets', label: 'Markets' },
  { href: '/statistics', label: 'Statistics' },
  { href: '/markets/create', label: 'Create Market' },
];

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [hasAdminSession, setHasAdminSession] = useState(false);

  useEffect(() => {
    setHasAdminSession(Boolean(sessionStorage.getItem('predictiq-admin-key')));
  }, [pathname]);

  const isLandingPage = pathname === '/';
  const isAdminSection = pathname?.startsWith('/admin');

  if (isLandingPage || isAdminSection) {
    return <>{children}</>;
  }

  const navItems = hasAdminSession
    ? [...NAV_ITEMS, { href: '/admin/content', label: 'Admin' }]
    : NAV_ITEMS;

  return (
    <div className="app-shell">
      <a href="#app-main-content" className="skip-link">
        Skip to main content
      </a>

      <header role="banner" className="app-shell__header">
        <div className="app-shell__header-inner">
          <Link href="/" aria-label="PredictIQ Home" className="app-shell__logo">
            <span className="app-shell__logo-fg">Predict</span>
            <span className="app-shell__logo-accent">IQ</span>
          </Link>

          <nav aria-label="Primary navigation" className="app-shell__nav">
            <ul className="app-shell__nav-list">
              {navItems.map((item) => {
                const isActive = pathname === item.href || pathname?.startsWith(`${item.href}/`);
                return (
                  <li key={item.href}>
                    <Link
                      href={item.href}
                      aria-current={isActive ? 'page' : undefined}
                      className={`app-shell__nav-link ${isActive ? 'app-shell__nav-link--active' : ''}`}
                    >
                      {item.label}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </nav>
        </div>
      </header>

      <main id="app-main-content" role="main" className="app-shell__main">
        {children}
      </main>

      <footer role="contentinfo" className="app-shell__footer">
        © {new Date().getFullYear()} PredictIQ. Built on Stellar.
      </footer>
    </div>
  );
}

export default AppShell;
