'use client';

import React from 'react';
import { useOnlineStatus } from '../lib/hooks/useOnlineStatus';
import './OfflineBanner.css';

/**
 * Persistent, app-wide banner shown while the browser has no network
 * connectivity. Backed by useOnlineStatus, which debounces rapid
 * online/offline flapping so this doesn't flicker.
 */
export const OfflineBanner: React.FC = () => {
  const isOnline = useOnlineStatus();

  if (isOnline) return null;

  return (
    <div className="offline-banner" role="status" aria-live="polite">
      <span className="offline-banner__dot" aria-hidden="true" />
      You&apos;re offline. Some data may be out of date until your connection returns.
    </div>
  );
};
