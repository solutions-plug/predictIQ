'use client';

import React, { useEffect, useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useI18n } from '../../lib/hooks/useI18n';
import { useActiveNavItem } from '../../../hooks/useActiveNavItem';
import '../../styles/admin.css';

function AdminAuthGate({ children }: { children: React.ReactNode }) {
  const { t } = useI18n();
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
          {t('admin.apiKey')}
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
          {isLoading ? 'Validating...' : t('admin.continue')}
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

function AdminNavLink({ href, label }: { href: string; label: string }) {
  const isActive = useActiveNavItem(href);
  return (
    <Link
      href={href}
      className={`admin-nav-link ${isActive ? 'active' : ''}`}
      aria-current={isActive ? 'page' : undefined}
    >
      {label}
    </Link>
  );
}

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  const { t } = useI18n();
  const pathname = usePathname();

export default function AdminLayout({ children }: { children: React.ReactNode }) {
  const navItems = [
    { href: '/admin/email/preview', label: t('admin.emailPreview') },
    { href: '/admin/email/analytics', label: t('admin.emailAnalytics') },
    { href: '/admin/blockchain/replay', label: t('admin.blockchainReplay') },
    { href: '/admin/content', label: t('admin.contentManagement') },
    { href: '/admin/audit', label: t('admin.auditLog') },
    { href: '/admin/api-keys', label: t('admin.apiKeys') },
  ];

  return (
    <AdminAuthGate>
      <div className="admin-layout">
        {/* Skip navigation for accessibility */}
        <a href="#admin-main-content" className="skip-link">
          {t('admin.skipToContent')}
        </a>

        {/* Admin Top Navigation */}
        <header className="admin-header" role="banner">
          <div className="admin-header-container">
            <div className="admin-brand-inner">
              <Link href="/" className="admin-brand" aria-label={t('admin.home')}>
                <span className="admin-brand-name">
                  Predict<span className="admin-brand-name-accent">IQ</span>
                </span>
              </Link>
              <span className="admin-brand-badge">{t('admin.badge')}</span>
            </div>

            <nav className="admin-nav" aria-label={t('admin.subNav')}>
              {navItems.map((item) => (
                <AdminNavLink key={item.href} href={item.href} label={item.label} />
              ))}
            </nav>

            <div>
              <Link href="/" className="admin-exit-link">
                {t('appShell.exitToSite')}
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
