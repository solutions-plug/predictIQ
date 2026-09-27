'use client';

/**
 * TextInput — shared design-system form primitive (#1317).
 *
 * Self-contained: renders its own <label>, hint, and validation-state
 * error message so callers (market creation #69-76, admin content editing
 * #97, the wallet/bet form #78) don't each need to wire up the
 * label/hint/error scaffolding themselves. Styled via classNames
 * (src/styles/ui.css) — see that file's header comment for why inline
 * `style` props don't work here.
 */

import React, { useId } from 'react';

export interface TextInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export const TextInput = React.forwardRef<HTMLInputElement, TextInputProps>(
  ({ label, hint, error, required, className = '', id, ...props }, ref) => {
    const generatedId = useId();
    const inputId = id ?? generatedId;
    const hintId = hint ? `${inputId}-hint` : undefined;
    const errorId = error ? `${inputId}-error` : undefined;
    const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined;

    return (
      <div className={`ui-field ${className}`}>
        {label && (
          <label htmlFor={inputId} className="ui-field__label">
            {label}
            {required && (
              <span aria-hidden="true" className="ui-field__required">
                *
              </span>
            )}
          </label>
        )}
        {hint && (
          <p id={hintId} className="ui-field__hint">
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
          {...props}
        />
        {error && (
          <p id={errorId} role="alert" className="ui-field__error">
            {error}
          </p>
        )}
      </div>
    );
  }
);
TextInput.displayName = 'TextInput';

export default TextInput;
