import React from 'react';
import './Skeleton.css';

interface SkeletonProps {
  className?: string;
  variant?: 'text' | 'rectangular' | 'circular';
  'aria-label'?: string;
}

export const Skeleton: React.FC<SkeletonProps> = ({
  className = '',
  variant = 'text',
  'aria-label': ariaLabel = 'Loading content'
}) => {
  return (
    <div
      className={`skeleton ${variant} ${className}`}
      role="status"
      aria-live="polite"
      aria-label={ariaLabel}
    >
      <span className="visually-hidden">{ariaLabel}</span>
    </div>
  );
};