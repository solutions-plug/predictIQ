'use client';

import React from 'react';

// ---------------------------------------------------------------------------
// Form Container
// ---------------------------------------------------------------------------

export interface FormProps extends React.FormHTMLAttributes<HTMLFormElement> {
  children: React.ReactNode;
}

export const Form: React.FC<FormProps> = ({ children, className = '', ...props }) => {
  return (
    <form noValidate className={`admin-form ${className}`} {...props}>
      {children}
    </form>
  );
};

// ---------------------------------------------------------------------------
// Form Field (Wrapper with label, hint, error)
// ---------------------------------------------------------------------------

export interface FormFieldProps {
  id?: string;
  label?: string;
  required?: boolean;
  hint?: string;
  error?: string;
  className?: string;
  children: React.ReactNode;
}

export const FormField: React.FC<FormFieldProps> = ({
  id,
  label,
  required,
  hint,
  error,
  className = '',
  children,
}) => {
  const hintId = id && hint ? `${id}-hint` : undefined;
  const errorId = id && error ? `${id}-error` : undefined;

  return (
    <div className={`form-field ${error ? 'has-error' : ''} ${className}`}>
      {label && (
        <label htmlFor={id}>
          {label}
          {required && <span>*</span>}
        </label>
      )}

      {hint && (
        <p id={hintId} className="form-field-hint">
          {hint}
        </p>
      )}

      <div>{children}</div>

      {error && (
        <div id={errorId} role="alert" className="form-field-error">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <span>{error}</span>
        </div>
      )}
    </div>
  );
};

// ---------------------------------------------------------------------------
// Input Component
// ---------------------------------------------------------------------------

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  error?: string;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ error, className = '', id, ...props }, ref) => {
    const errorId = id && error ? `${id}-error` : undefined;

    return (
      <input
        ref={ref}
        id={id}
        aria-invalid={!!error}
        aria-describedby={errorId}
        className={`form-input ${error ? 'input-error' : ''} ${className}`}
        {...props}
      />
    );
  }
);
Input.displayName = 'Input';

// ---------------------------------------------------------------------------
// Textarea Component
// ---------------------------------------------------------------------------

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  error?: string;
}

export const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ error, className = '', id, rows = 4, ...props }, ref) => {
    const errorId = id && error ? `${id}-error` : undefined;

    return (
      <textarea
        ref={ref}
        id={id}
        rows={rows}
        aria-invalid={!!error}
        aria-describedby={errorId}
        className={`form-textarea ${error ? 'textarea-error' : ''} ${className}`}
        {...props}
      />
    );
  }
);
Textarea.displayName = 'Textarea';

// ---------------------------------------------------------------------------
// Select Component
// ---------------------------------------------------------------------------

export interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  error?: string;
}

export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ error, className = '', id, children, ...props }, ref) => {
    const errorId = id && error ? `${id}-error` : undefined;

    return (
      <select
        ref={ref}
        id={id}
        aria-invalid={!!error}
        aria-describedby={errorId}
        className={`form-select ${error ? 'select-error' : ''} ${className}`}
        {...props}
      >
        {children}
      </select>
    );
  }
);
Select.displayName = 'Select';

// ---------------------------------------------------------------------------
// Button Component
// ---------------------------------------------------------------------------

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  isLoading?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
  /** Stretches the button to fill its container's width. */
  fullWidth?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'primary',
      isLoading = false,
      leftIcon,
      rightIcon,
      fullWidth = false,
      children,
      disabled,
      className = '',
      ...props
    },
    ref
  ) => {
    const isDisabled = disabled || isLoading;

    return (
      <button
        ref={ref}
        disabled={isDisabled}
        aria-disabled={isDisabled}
        className={`form-btn btn-${variant} ${fullWidth ? 'form-btn--full-width' : ''} ${isLoading ? 'btn-loading' : ''} ${className}`}
        {...props}
      >
        {isLoading && <span className="btn-loading-spinner" aria-hidden="true" />}
        {!isLoading && leftIcon}
        <span>{children}</span>
        {!isLoading && rightIcon}
      </button>
    );
  }
);
Button.displayName = 'Button';

// ---------------------------------------------------------------------------
// Status Alert Component
// ---------------------------------------------------------------------------

export interface StatusAlertProps {
  type: 'success' | 'error' | 'warning' | 'info';
  title?: string;
  message?: string;
  children?: React.ReactNode;
  onDismiss?: () => void;
  className?: string;
}

export const StatusAlert: React.FC<StatusAlertProps> = ({
  type,
  title,
  message,
  children,
  onDismiss,
  className = '',
}) => {
  return (
    <div
      role={type === 'error' ? 'alert' : 'status'}
      aria-live="polite"
      className={`status-alert alert-${type} ${className}`}
    >
      <div className="status-alert__body">
        {title && <h4 className="status-alert__title">{title}</h4>}
        {message && <p className="status-alert__message">{message}</p>}
        {children}
      </div>

      {onDismiss && (
        <button type="button" onClick={onDismiss} aria-label="Dismiss alert" className="status-alert__dismiss">
          ×
        </button>
      )}
    </div>
  );
};
