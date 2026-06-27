import React from 'react';
import './Skeleton.css';

interface SkeletonProps {
  width?: string;
  height?: string;
  className?: string;
  variant?: 'text' | 'rectangular' | 'circular';
  'aria-label'?: string;
}

export const Skeleton: React.FC<SkeletonProps> = ({
  width = '100%',
  height = '1rem',
  className = '',
  variant = 'text',
  'aria-label': ariaLabel = 'Loading content'
}) => {
  const style = {
    width,
    height,
  };

  return (
    <div
      className={`skeleton ${variant} ${className}`}
      style={style}
      role="status"
      aria-live="polite"
      aria-label={ariaLabel}
    >
      <span className="visually-hidden">{ariaLabel}</span>
    </div>
  );
};