'use client';

/**
 * TextInput — shared design-system form primitive (#1317).
 *
 * Self-contained: renders its own <label>, hint, and validation-state
 * error message so callers (market creation #69-76, admin content editing
 * #97, the wallet/bet form #78) don't each need to wire up the
 * label/hint/error scaffolding themselves.
 */

import React, { useId } from 'react';

export interface TextInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export const TextInput = React.forwardRef<HTMLInputElement, TextInputProps>(
  ({ label, hint, error, required, className = '', id, style, ...props }, ref) => {
    const generatedId = useId();
    const inputId = id ?? generatedId;
    const hintId = hint ? `${inputId}-hint` : undefined;
    const errorId = error ? `${inputId}-error` : undefined;
    const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined;

    return (
      <div className={`ui-field ${className}`} style={{ marginBottom: '1.25rem' }}>
        {label && (
          <label
            htmlFor={inputId}
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
        <input
          ref={ref}
          id={inputId}
          required={required}
          aria-invalid={!!error}
          aria-describedby={describedBy}
          className={`ui-input ${error ? 'ui-input--error' : ''}`}
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
            transition: 'border-color var(--dur-fast), box-shadow var(--dur-fast)',
            ...style,
          }}
          {...props}
        />
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
TextInput.displayName = 'TextInput';

export default TextInput;
