'use client';

/**
 * Select — shared design-system form primitive (#1317).
 * See TextInput.tsx for the shared label/hint/error convention.
 */

import React, { useId } from 'react';

export interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ label, hint, error, required, className = '', id, children, style, ...props }, ref) => {
    const generatedId = useId();
    const selectId = id ?? generatedId;
    const hintId = hint ? `${selectId}-hint` : undefined;
    const errorId = error ? `${selectId}-error` : undefined;
    const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined;

    return (
      <div className={`ui-field ${className}`} style={{ marginBottom: '1.25rem' }}>
        {label && (
          <label
            htmlFor={selectId}
            style={{
              display: 'block',
              fontSize: 'var(--text-sm)',
              fontWeight: 500,
              color: 'var(--fg)',
              marginBottom: '0.4rem',
            }}
          >
            {label}
            {required && (
              <span aria-hidden="true" style={{ color: 'var(--destructive)', marginLeft: '0.25rem' }}>
                *
              </span>
            )}
          </label>
        )}
        {hint && (
          <p
            id={hintId}
            style={{
              fontSize: 'var(--text-xs)',
              color: 'var(--fg-muted)',
              marginTop: '-0.2rem',
              marginBottom: '0.4rem',
              lineHeight: 1.4,
            }}
          >
            {hint}
          </p>
        )}
        <select
          ref={ref}
          id={selectId}
          required={required}
          aria-invalid={!!error}
          aria-describedby={describedBy}
          className={`ui-select ${error ? 'ui-select--error' : ''}`}
          style={{
            width: '100%',
            padding: '0.65rem 0.85rem',
            fontSize: 'var(--text-sm)',
            fontFamily: 'inherit',
            backgroundColor: 'var(--surface-2)',
            color: 'var(--fg)',
            border: `1px solid ${error ? 'var(--destructive)' : 'var(--border)'}`,
            borderRadius: 'var(--radius-sm)',
            outline: 'none',
            boxSizing: 'border-box',
            cursor: 'pointer',
            transition: 'border-color var(--dur-fast), box-shadow var(--dur-fast)',
            ...style,
          }}
          {...props}
        >
          {children}
        </select>
        {error && (
          <p
            id={errorId}
            role="alert"
            style={{
              color: 'var(--destructive)',
              fontSize: 'var(--text-xs)',
              marginTop: '0.4rem',
              fontWeight: 500,
            }}
          >
            {error}
          </p>
        )}
      </div>
    );
  }
);
Select.displayName = 'Select';

export default Select;
