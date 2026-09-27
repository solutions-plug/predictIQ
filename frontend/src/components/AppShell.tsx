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
import { useI18n } from '../lib/hooks/useI18n';
import { useActiveNavItem } from '../hooks/useActiveNavItem';

const NAV_ITEMS = [
  { href: '/markets', label: 'Markets' },
  { href: '/statistics', label: 'Statistics' },
  { href: '/markets/create', label: 'Create Market' },
];

/**
 * Validates the stored admin key against the admin session endpoint.
 *
 * Key presence alone is not sufficient: a stale, revoked, or garbage key
 * (including one left over from a failed login attempt) must not be
 * treated as an active admin session. This mirrors the validation used by
 * AdminAuthGate so both surfaces agree on what counts as a valid session.
 */
export async function validateAdminSession(): Promise<boolean> {
  if (typeof window === 'undefined') return false;

  const key = sessionStorage.getItem('predictiq-admin-key');
  if (!key) return false;

  try {
    const res = await fetch('/api/v1/admin/session', {
      headers: { 'x-admin-key': key },
    });
    return res.ok;
  } catch {
    return false;
  }
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const { t } = useI18n();
  const pathname = usePathname();
  const isActiveNavItem = useActiveNavItem(pathname);
  const [hasAdminSession, setHasAdminSession] = useState(false);

  useEffect(() => {
    let cancelled = false;

    validateAdminSession().then((valid) => {
      if (!cancelled) setHasAdminSession(valid);
    });

    return () => {
      cancelled = true;
    };
  }, [pathname]);

  const isLandingPage = pathname === '/';
  const isAdminSection = pathname?.startsWith('/admin');

  if (isLandingPage || isAdminSection) {
    return <>{children}</>;
  }

  const NAV_ITEMS = [
    { href: '/markets', label: t('nav.markets') },
    { href: '/statistics', label: t('nav.statistics') },
    { href: '/markets/create', label: t('nav.createMarket') },
  ];

  const navItems = hasAdminSession
    ? [...NAV_ITEMS, { href: '/admin/content', label: t('nav.admin') }]
    : NAV_ITEMS;

  return (
    <div className="app-shell">
      <a href="#app-main-content" className="skip-link">
        {t('appShell.skipToMain')}
      </a>

      <header role="banner" className="app-shell__header">
        <div className="app-shell__header-inner">
          <Link href="/" aria-label={t('appShell.home')} className="app-shell__logo">
            <span className="app-shell__logo-fg">Predict</span>
            <span className="app-shell__logo-accent">IQ</span>
          </Link>

          <nav aria-label={t('appShell.primaryNav')} className="app-shell__nav">
            <ul className="app-shell__nav-list">
              {navItems.map((item) => {
                const isActive = isActiveNavItem(item.href);
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
        © {new Date().getFullYear()} {t('appShell.home')}. Built on Stellar.
      </footer>
    </div>
  );
}

export default AppShell;
