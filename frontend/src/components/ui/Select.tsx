'use client';

/**
 * Select — shared design-system form primitive (#1317).
 * See TextInput.tsx for the shared label/hint/error convention. Styled
 * via classNames (src/styles/ui.css) — see that file's header comment
 * for why inline `style` props don't work here.
 */

import React, { useId } from 'react';

export interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ label, hint, error, required, className = '', id, children, ...props }, ref) => {
    const generatedId = useId();
    const selectId = id ?? generatedId;
    const hintId = hint ? `${selectId}-hint` : undefined;
    const errorId = error ? `${selectId}-error` : undefined;
    const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined;

    return (
      <div className={`ui-field ${className}`}>
        {label && (
          <label htmlFor={selectId} className="ui-field__label">
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
        <select
          ref={ref}
          id={selectId}
          required={required}
          aria-invalid={!!error}
          aria-describedby={describedBy}
          className={`ui-select ${error ? 'ui-select--error' : ''}`}
          {...props}
        >
          {children}
        </select>
        {error && (
          <p id={errorId} role="alert" className="ui-field__error">
            {error}
          </p>
        )}
      </div>
    );
  }
);
Select.displayName = 'Select';

export default Select;
