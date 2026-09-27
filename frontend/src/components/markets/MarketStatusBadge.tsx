import React from 'react';
import { Badge, type BadgeVariant } from '../ui/Badge';

interface MarketStatusBadgeProps {
  status?: string | null;
  className?: string;
}

type StatusConfig = {
  label: string;
  icon: string;
  variant: BadgeVariant;
  ariaLabel: string;
};

const STATUS_MAP: Record<string, StatusConfig> = {
  Active: {
    label: 'Active',
    icon: '●',
    variant: 'success',
    ariaLabel: 'Market is active',
  },
  PendingResolution: {
    label: 'Pending Resolution',
    icon: '⧗',
    variant: 'warning',
    ariaLabel: 'Market is pending resolution',
  },
  Disputed: {
    label: 'Disputed',
    icon: '⚡',
    variant: 'info',
    ariaLabel: 'Market is disputed',
  },
  Resolved: {
    label: 'Resolved',
    icon: '✓',
    variant: 'neutral',
    ariaLabel: 'Market is resolved',
  },
  Cancelled: {
    label: 'Cancelled',
    icon: '✕',
    variant: 'danger',
    ariaLabel: 'Market is cancelled',
  },
};

const UNKNOWN_STATUS: StatusConfig = {
  label: 'Unknown',
  icon: '?',
  variant: 'neutral',
  ariaLabel: 'Market status is unknown',
};

export const MarketStatusBadge: React.FC<MarketStatusBadgeProps> = ({
  status,
  className = '',
}) => {
  const config = status && STATUS_MAP[status] ? STATUS_MAP[status] : UNKNOWN_STATUS;

  return (
    <Badge
      variant={config.variant}
      icon={config.icon}
      className={className}
      role="status"
      aria-label={config.ariaLabel}
    >
      {config.label}
    </Badge>
  );
};
