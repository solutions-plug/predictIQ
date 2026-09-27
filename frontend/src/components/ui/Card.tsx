/**
 * Card — shared design-system container primitive (#1316).
 *
 * Market list items (#57), statistics tiles (#49), and admin panels
 * (#89-97) each currently reinvent their own padding/border/shadow rules;
 * this is the one shared container to converge on instead. Styled via
 * classNames (src/styles/ui.css) — see that file's header comment for
 * why inline `style` props don't work here.
 */

import React from 'react';

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Renders with a hover elevation/border-highlight, for clickable cards. */
  interactive?: boolean;
  /** Removes the default padding, for cards that manage their own inner layout. */
  noPadding?: boolean;
}

export const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ interactive = false, noPadding = false, className = '', children, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={`ui-card ${interactive ? 'ui-card--interactive' : ''} ${noPadding ? 'ui-card--no-padding' : ''} ${className}`}
        {...props}
      >
        {children}
      </div>
    );
  }
);
Card.displayName = 'Card';

export interface CardHeaderProps extends React.HTMLAttributes<HTMLDivElement> {}

export function CardHeader({ className = '', children, ...props }: CardHeaderProps) {
  return (
    <div className={`ui-card__header ${className}`} {...props}>
      {children}
    </div>
  );
}

export interface CardTitleProps extends React.HTMLAttributes<HTMLHeadingElement> {
  as?: 'h2' | 'h3' | 'h4';
}

export function CardTitle({ as = 'h3', className = '', children, ...props }: CardTitleProps) {
  const Heading = as;
  return (
    <Heading className={`ui-card__title ${className}`} {...props}>
      {children}
    </Heading>
  );
}

export interface CardBodyProps extends React.HTMLAttributes<HTMLDivElement> {}

export function CardBody({ className = '', children, ...props }: CardBodyProps) {
  return (
    <div className={`ui-card__body ${className}`} {...props}>
      {children}
    </div>
  );
}

export interface CardFooterProps extends React.HTMLAttributes<HTMLDivElement> {}

export function CardFooter({ className = '', children, ...props }: CardFooterProps) {
  return (
    <div className={`ui-card__footer ${className}`} {...props}>
      {children}
    </div>
  );
}

export default Card;
