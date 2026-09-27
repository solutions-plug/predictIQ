'use client';

import React, { useEffect, useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import '../../styles/admin.css';

function AdminAuthGate({ children }: { children: React.ReactNode }) {
  const [key, setKey] = useState('');
  const [ok, setOk] = useState(false);
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    const k = sessionStorage.getItem('predictiq-admin-key');
    if (k) {
      setKey(k);
      fetch('/api/v1/admin/session', { headers: { 'X-API-Key': k } })
        .then((r) => {
          if (r.ok) {
            setOk(true);
          } else {
            sessionStorage.removeItem('predictiq-admin-key');
            setKey('');
            setError('Session expired. Please log in again.');
          }
        })
        .catch(() => {
          sessionStorage.removeItem('predictiq-admin-key');
          setKey('');
          setError('Failed to validate session.');
        });
    }
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setIsLoading(true);

    try {
      const response = await fetch('/api/v1/admin/session', {
        headers: { 'X-API-Key': key },
      });

      if (response.ok) {
        sessionStorage.setItem('predictiq-admin-key', key);
        setOk(true);
      } else {
        setError('Invalid API key. Please try again.');
      }
    } catch {
      setError('Failed to validate API key. Please try again.');
    } finally {
      setIsLoading(false);
    }
  };

  const handleLogout = () => {
    sessionStorage.removeItem('predictiq-admin-key');
    setKey('');
    setOk(false);
    setError('');
  };

  if (!ok) {
    return (
      <form className="admin-auth-form" onSubmit={handleSubmit}>
        <label>
          Admin API key
          <input
            value={key}
            onChange={(e) => setKey(e.target.value)}
            required
            type="password"
            disabled={isLoading}
          />
        </label>
        {error && <div className="admin-auth-error">{error}</div>}
        <button type="submit" disabled={isLoading}>
          {isLoading ? 'Validating...' : 'Continue'}
        </button>
      </form>
    );
  }

  return (
    <div>
      <button onClick={handleLogout} className="admin-logout-btn">
        Logout
      </button>
      {children}
    </div>
  );
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
