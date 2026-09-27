'use client';

/**
 * Button — shared design-system primitive (#1315).
 *
 * Every write action in this backlog (bet placement, market creation,
 * resolution, admin actions) should funnel through this so loading/
 * disabled states are handled consistently instead of ad hoc per form.
 * Styling follows the existing Button in components/admin/Form.tsx (the
 * closest prior art), generalized here to be usable outside the admin
 * section too. Styled via classNames (src/styles/ui.css) — see that
 * file's header comment for why inline `style` props don't work here.
 */

import React from 'react';

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  isLoading?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    { variant = 'primary', isLoading = false, leftIcon, rightIcon, children, disabled, className = '', ...props },
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
        {...props}
      >
        {isLoading && <span aria-hidden="true" className="ui-btn__spinner" />}
        {!isLoading && leftIcon}
        <span>{children}</span>
        {!isLoading && rightIcon}
      </button>
    );
  }
);
Button.displayName = 'Button';

export default Button;
