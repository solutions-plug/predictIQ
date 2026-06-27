import React from 'react';
import './LoadingSpinner.css';

interface LoadingSpinnerProps {
  size?: 'small' | 'medium' | 'large';
  className?: string;
  'aria-label'?: string;
}

export const LoadingSpinner: React.FC<LoadingSpinnerProps> = ({
  size = 'medium',
  className = '',
  'aria-label': ariaLabel = 'Loading'
}) => {
  return (
    <div
      className={`loading-spinner ${size} ${className}`}
      role="status"
      aria-live="polite"
      aria-label={ariaLabel}
    >
      <div className="spinner" aria-hidden="true"></div>
      <span className="visually-hidden">{ariaLabel}</span>
    </div>
  );
};