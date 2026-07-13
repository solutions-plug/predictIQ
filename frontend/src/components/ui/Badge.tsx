import React from 'react';
import type { MarketStatus } from '../../lib/types';

type Tone = 'default' | 'open' | 'resolved' | 'closed' | 'gold';

export function Badge({
  tone = 'default',
  className = '',
  children,
}: {
  tone?: Tone;
  className?: string;
  children: React.ReactNode;
}) {
  const classes = ['badge', tone !== 'default' ? `badge--${tone}` : '', className]
    .filter(Boolean)
    .join(' ');
  return <span className={classes}>{children}</span>;
}

/** Convenience badge for a market status. */
export function StatusBadge({ status }: { status: MarketStatus }) {
  const label = status.charAt(0).toUpperCase() + status.slice(1);
  return <Badge tone={status}>{label}</Badge>;
}
