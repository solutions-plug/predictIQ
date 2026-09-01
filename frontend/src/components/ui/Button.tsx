'use client';

/**
 * Button — shared design-system primitive (#1315).
 *
 * Every write action in this backlog (bet placement, market creation,
 * resolution, admin actions) should funnel through this so loading/
 * disabled states are handled consistently instead of ad hoc per form.
 * Styling follows the existing Button in components/admin/Form.tsx (the
 * closest prior art), generalized here to be usable outside the admin
 * section too.
 */

import React from 'react';

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  isLoading?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

function variantStyles(variant: ButtonVariant): React.CSSProperties {
  switch (variant) {
    case 'primary':
      return { backgroundColor: 'var(--gold)', color: 'var(--on-primary)', border: 'none', fontWeight: 600 };
    case 'danger':
      return { backgroundColor: 'var(--destructive)', color: '#ffffff', border: 'none', fontWeight: 600 };
    case 'secondary':
      return {
        backgroundColor: 'var(--surface-2)',
        color: 'var(--fg)',
        border: '1px solid var(--border-strong)',
        fontWeight: 500,
      };
    case 'ghost':
      return { backgroundColor: 'transparent', color: 'var(--fg-muted)', border: 'none', fontWeight: 500 };
  }
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    { variant = 'primary', isLoading = false, leftIcon, rightIcon, children, disabled, className = '', style, ...props },
    ref
  ) => {
    const isDisabled = disabled || isLoading;

    return (
      <button
        ref={ref}
        type={props.type ?? 'button'}
        disabled={isDisabled}
        aria-disabled={isDisabled}
        aria-busy={isLoading || undefined}
        className={`ui-btn ui-btn--${variant} ${className}`}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '0.5rem',
          padding: '0.65rem 1.25rem',
          fontSize: 'var(--text-sm)',
          fontFamily: 'inherit',
          borderRadius: 'var(--radius-sm)',
          cursor: isDisabled ? 'not-allowed' : 'pointer',
          opacity: isDisabled ? 0.6 : 1,
          transition: 'all var(--dur-fast)',
          textDecoration: 'none',
          ...variantStyles(variant),
          ...style,
        }}
        {...props}
      >
        {isLoading && (
          <span
            aria-hidden="true"
            style={{
              display: 'inline-block',
              width: '14px',
              height: '14px',
              border: '2px solid currentColor',
              borderTopColor: 'transparent',
              borderRadius: '50%',
              animation: 'spin 0.8s linear infinite',
            }}
          />
        )}
        {!isLoading && leftIcon}
        <span>{children}</span>
        {!isLoading && rightIcon}
      </button>
    );
  }
);
Button.displayName = 'Button';

export default Button;
