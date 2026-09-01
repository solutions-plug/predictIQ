'use client';

import React, { useEffect, useState } from 'react';
import {
  onDeprecation,
  currentDeprecation,
  type DeprecationInfo,
} from '../lib/api/deprecation';

const DISMISS_KEY = 'predictiq.deprecation.dismissed-sunset';

function readDismissed(): string | null {
  try {
    return localStorage.getItem(DISMISS_KEY);
  } catch {
    return null;
  }
}

function formatSunset(sunset: string | null): string {
  if (!sunset) return 'soon';
  const d = new Date(sunset);
  return Number.isNaN(d.getTime())
    ? sunset
    : d.toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' });
}

/**
 * Dismissible, non-blocking banner shown when the API reports it is deprecated.
 * The dismissal is remembered per sunset-date: it stays hidden across visits, but
 * a new/changed sunset date brings it back.
 */
export const DeprecationBanner: React.FC = () => {
  const [info, setInfo] = useState<DeprecationInfo | null>(() => currentDeprecation());
  const [dismissedSunset, setDismissedSunset] = useState<string | null>(() => readDismissed());

  useEffect(() => onDeprecation(setInfo), []);

  if (!info) return null;
  // Only counts as "dismissed" when it was dismissed for *this* sunset date.
  if (dismissedSunset !== null && dismissedSunset === (info.sunset ?? '')) return null;

  const dismiss = () => {
    const key = info.sunset ?? '';
    try {
      localStorage.setItem(DISMISS_KEY, key);
    } catch {
      /* storage unavailable - banner will reappear next load, which is acceptable */
    }
    setDismissedSunset(key);
  };

  return (
    <div className="deprecation-banner" role="status" aria-live="polite">
      <p className="deprecation-banner__text">
        This version of the PredictIQ API is deprecated and support ends{' '}
        <strong>{formatSunset(info.sunset)}</strong>.
        {info.migrationUrl && (
          <>
            {' '}
            <a href={info.migrationUrl} className="deprecation-banner__link">
              Read the migration guide
            </a>
            .
          </>
        )}
      </p>
      <button
        type="button"
        className="deprecation-banner__dismiss"
        onClick={dismiss}
        aria-label="Dismiss deprecation notice"
      >
        &times;
      </button>
    </div>
  );
};

export default DeprecationBanner;
