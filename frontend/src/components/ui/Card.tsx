/**
 * Card — shared design-system container primitive (#1316).
 *
 * Market list items (#57), statistics tiles (#49), and admin panels
 * (#89-97) each currently reinvent their own padding/border/shadow rules;
 * this is the one shared container to converge on instead.
 */

import React from 'react';

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Renders with a hover elevation/border-highlight, for clickable cards. */
  interactive?: boolean;
  /** Removes the default padding, for cards that manage their own inner layout. */
  noPadding?: boolean;
}

export const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ interactive = false, noPadding = false, className = '', style, children, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={`ui-card ${interactive ? 'ui-card--interactive' : ''} ${className}`}
        style={{
          backgroundColor: 'var(--surface)',
          border: '1px solid var(--border)',
          borderRadius: 'var(--radius)',
          boxShadow: 'var(--shadow-sm)',
          padding: noPadding ? 0 : '1.25rem',
          transition: interactive ? 'border-color var(--dur-fast), box-shadow var(--dur-fast)' : undefined,
          cursor: interactive ? 'pointer' : undefined,
          ...style,
        }}
        {...props}
      >
        {children}
      </div>
    );
  }
);
Card.displayName = 'Card';

export interface CardHeaderProps extends React.HTMLAttributes<HTMLDivElement> {}

export function CardHeader({ className = '', style, children, ...props }: CardHeaderProps) {
  return (
    <div
      className={`ui-card__header ${className}`}
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'space-between',
        gap: '1rem',
        marginBottom: '0.85rem',
        ...style,
      }}
      {...props}
    >
      {children}
    </div>
  );
}

export interface CardTitleProps extends React.HTMLAttributes<HTMLHeadingElement> {
  as?: 'h2' | 'h3' | 'h4';
}

export function CardTitle({ as = 'h3', className = '', style, children, ...props }: CardTitleProps) {
  const Heading = as;
  return (
    <Heading
      className={`ui-card__title ${className}`}
      style={{
        margin: 0,
        fontSize: 'var(--text-lg)',
        fontFamily: 'var(--font-display)',
        fontWeight: 600,
        color: 'var(--fg)',
        ...style,
      }}
      {...props}
    >
      {children}
    </Heading>
  );
}

export interface CardBodyProps extends React.HTMLAttributes<HTMLDivElement> {}

export function CardBody({ className = '', style, children, ...props }: CardBodyProps) {
  return (
    <div
      className={`ui-card__body ${className}`}
      style={{ fontSize: 'var(--text-sm)', color: 'var(--fg-muted)', lineHeight: 1.5, ...style }}
      {...props}
    >
      {children}
    </div>
  );
}

export interface CardFooterProps extends React.HTMLAttributes<HTMLDivElement> {}

export function CardFooter({ className = '', style, children, ...props }: CardFooterProps) {
  return (
    <div
      className={`ui-card__footer ${className}`}
      style={{
        marginTop: '1rem',
        paddingTop: '0.85rem',
        borderTop: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        gap: '0.75rem',
        ...style,
      }}
      {...props}
    >
      {children}
    </div>
  );
}

export default Card;
