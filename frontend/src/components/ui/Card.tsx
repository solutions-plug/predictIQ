import React from 'react';

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  interactive?: boolean;
  as?: 'div' | 'article' | 'section';
}

export function Card({ interactive = false, as = 'div', className = '', children, ...rest }: CardProps) {
  const Tag = as;
  const classes = ['card', interactive ? 'card--interactive' : '', className].filter(Boolean).join(' ');
  return (
    <Tag className={classes} {...rest}>
      {children}
    </Tag>
  );
}
