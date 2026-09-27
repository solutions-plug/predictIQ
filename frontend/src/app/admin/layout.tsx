'use client';

import React, { useEffect, useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import '../../styles/admin.css';

function AdminAuthGate({ children }: { children: React.ReactNode }) {
  const [key, setKey] = useState('');
  const [ok, setOk] = useState(false);

  useEffect(() => {
    const k = sessionStorage.getItem('predictiq-admin-key');
    if (k) {
      setKey(k);
      fetch('/api/v1/admin/session', { headers: { 'X-API-Key': k } })
        .then((r) => setOk(r.ok))
        .catch(() => setOk(false));
    }
  }, []);

  if (!ok) {
    return (
      <form
        className="admin-auth-form"
        onSubmit={(e) => {
          e.preventDefault();
          sessionStorage.setItem('predictiq-admin-key', key);
          setOk(true);
        }}
      >
        <label>
          Admin API key
          <input
            value={key}
            onChange={(e) => setKey(e.target.value)}
            required
            type="password"
          />
        </label>
        <button type="submit">Continue</button>
      </form>
    );
  }

  return <>{children}</>;
}

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();

  const navItems = [
    { href: '/admin/email/preview', label: 'Email Preview' },
    { href: '/admin/email/analytics', label: 'Email Analytics' },
    { href: '/admin/blockchain/replay', label: 'Blockchain Replay' },
    { href: '/admin/content', label: 'Content Management' },
    { href: '/admin/audit', label: 'Audit Log' },
    { href: '/admin/api-keys', label: 'API Keys' },
  ];

  return (
    <AdminAuthGate>
      <div className="admin-layout">
        {/* Skip navigation for accessibility */}
        <a href="#admin-main-content" className="skip-link">
          Skip to admin content
        </a>

        {/* Admin Top Navigation */}
        <header className="admin-header" role="banner">
          <div className="admin-header-container">
            <div className="admin-brand-inner">
              <Link href="/" className="admin-brand" aria-label="PredictIQ Home">
                <span className="admin-brand-name">
                  Predict<span className="admin-brand-name-accent">IQ</span>
                </span>
              </Link>
              <span className="admin-brand-badge">Admin</span>
            </div>

            <nav className="admin-nav" aria-label="Admin sub-navigation">
              {navItems.map((item) => {
                const isActive = pathname === item.href || pathname?.startsWith(`${item.href}/`);
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={`admin-nav-link ${isActive ? 'active' : ''}`}
                    aria-current={isActive ? 'page' : undefined}
                  >
                    {item.label}
                  </Link>
                );
              })}
            </nav>

            <div>
              <Link href="/" className="admin-exit-link">
                Exit to Site →
              </Link>
            </div>
          </div>
        </header>

        {/* Main Admin Content */}
        <main id="admin-main-content" className="admin-main" role="main">
          {children}
        </main>
      </div>
    </AdminAuthGate>
  );
}
