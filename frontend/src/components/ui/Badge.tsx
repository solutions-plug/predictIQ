/**
 * Badge — shared design-system status/label primitive.
 *
 * A generic pill for short status/category labels. Domain-specific badges
 * with their own semantics (MarketStatusBadge's status→color mapping) build
 * on top of this for shape/spacing instead of redefining it; compound
 * widgets that are more than a label (AssetBadge, DepositTierBadge) are out
 * of scope for this primitive and keep their own markup.
 */

import React from 'react';

export type BadgeVariant = 'neutral' | 'success' | 'warning' | 'danger' | 'info';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: BadgeVariant;
  /** Optional leading icon/glyph, marked aria-hidden. */
  icon?: React.ReactNode;
}

export const Badge = React.forwardRef<HTMLSpanElement, BadgeProps>(
  ({ variant = 'neutral', icon, className = '', children, ...props }, ref) => {
    return (
      <span ref={ref} className={`ui-badge ui-badge--${variant} ${className}`} {...props}>
        {icon && <span aria-hidden="true">{icon}</span>}
        <span>{children}</span>
      </span>
    );
  }
);
Badge.displayName = 'Badge';

export default Badge;
