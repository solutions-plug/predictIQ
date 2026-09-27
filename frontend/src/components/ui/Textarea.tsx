'use client';

/**
 * Textarea — shared design-system form primitive (#1317).
 * See TextInput.tsx for the shared label/hint/error convention. Styled
 * via classNames (src/styles/ui.css) — see that file's header comment
 * for why inline `style` props don't work here.
 */

import React, { useId } from 'react';

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ label, hint, error, required, className = '', id, rows = 4, ...props }, ref) => {
    const generatedId = useId();
    const textareaId = id ?? generatedId;
    const hintId = hint ? `${textareaId}-hint` : undefined;
    const errorId = error ? `${textareaId}-error` : undefined;
    const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined;

    return (
      <div className={`ui-field ${className}`}>
        {label && (
          <label htmlFor={textareaId} className="ui-field__label">
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
        <textarea
          ref={ref}
          id={textareaId}
          rows={rows}
          required={required}
          aria-invalid={!!error}
          aria-describedby={describedBy}
          className={`ui-textarea ${error ? 'ui-textarea--error' : ''}`}
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
Textarea.displayName = 'Textarea';

export default Textarea;
