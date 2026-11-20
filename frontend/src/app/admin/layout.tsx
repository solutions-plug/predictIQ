'use client';

import { ReactNode, useCallback, useEffect, useState } from 'react';
import Link from 'next/link';
import { usePathname, useRouter } from 'next/navigation';
import { Modal } from '@/components/Modal';

interface AdminLayoutProps {
  children: ReactNode;
}

interface RotateKeyResponse {
  apiKey?: string;
  key?: string;
}

const ADMIN_NAV = [
  { href: '/admin', label: 'Overview' },
  { href: '/admin/users', label: 'Users' },
  { href: '/admin/settings', label: 'Settings' },
];

export default function AdminLayout({ children }: AdminLayoutProps) {
  const pathname = usePathname();
  const router = useRouter();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [rotating, setRotating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newKey, setNewKey] = useState<string | null>(null);

  useEffect(() => {
    if (!newKey) return;
    try {
      sessionStorage.setItem('admin:apiKey', newKey);
    } catch {
      // sessionStorage may be unavailable; the key is still shown below.
    }
  }, [newKey]);

  const rotateKey = useCallback(async () => {
    setConfirmOpen(false);
    setRotating(true);
    setError(null);
    try {
      const res = await fetch('/api/admin/rotate-key', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ confirm: true }),
      });
      if (!res.ok) {
        throw new Error(`Rotation failed (${res.status})`);
      }
      const data = (await res.json()) as RotateKeyResponse;
      const issued = data.apiKey ?? data.key ?? null;
      if (!issued) {
        throw new Error('Rotation response did not include a key');
      }
      setNewKey(issued);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Rotation failed');
    } finally {
      setRotating(false);
    }
  }, []);

  return (
    <div className="admin-layout">
      <aside className="admin-sidebar">
        <nav>
          {ADMIN_NAV.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className={pathname === item.href ? 'active' : undefined}
            >
              {item.label}
            </Link>
          ))}
        </nav>
        <button
          type="button"
          className="admin-rotate-key"
          onClick={() => setConfirmOpen(true)}
          disabled={rotating}
        >
          {rotating ? 'Rotating…' : 'Rotate API key'}
        </button>
      </aside>

      <main className="admin-content">
        {error && (
          <p role="alert" className="admin-error">
            {error}
          </p>
        )}
        {newKey && (
          <div role="status" className="admin-new-key">
            <p>Your new API key (shown once):</p>
            <code>{newKey}</code>
            <button type="button" onClick={() => setNewKey(null)}>
              Dismiss
            </button>
          </div>
        )}
        {children}
      </main>

      <Modal
        open={confirmOpen}
        onClose={() => setConfirmOpen(false)}
        title="Rotate API key?"
      >
        <p>
          Rotating the API key immediately invalidates the current key and any
          active sessions using it. This cannot be undone.
        </p>
        <div className="modal__actions">
          <button type="button" onClick={() => setConfirmOpen(false)}>
            Cancel
          </button>
          <button type="button" onClick={rotateKey}>
            Rotate key
          </button>
        </div>
      </Modal>
    </div>
  );
}
