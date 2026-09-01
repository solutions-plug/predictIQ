'use client';

/**
 * Modal — shared design-system dialog primitive (#1318).
 *
 * Two ad-hoc Modal implementations already exist in this codebase
 * (components/Modal.tsx, components/admin/Modal.tsx), each built because
 * this shared primitive didn't exist yet — components/Modal.tsx's own
 * header comment says as much. This consolidates both: focus trap +
 * Tab-cycling, Escape-to-close, backdrop click-to-close (each
 * individually disable-able for confirmation-gated destructive actions),
 * body-scroll lock while open, and restores focus to the previously
 * focused element on close. Existing callers — bet placement (#78),
 * market cancellation (#74), the blockchain replay admin tool (#96), and
 * GDPR deletion (#102) — can migrate to this without rewriting how they
 * open/close the dialog; only the two legacy Modal.tsx files are
 * superseded.
 */

import React, { useCallback, useEffect, useId, useRef } from 'react';

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  disableBackdropDismiss?: boolean;
  disableEscapeKey?: boolean;
  maxWidth?: string;
  className?: string;
}

export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  disableBackdropDismiss = false,
  disableEscapeKey = false,
  maxWidth = '560px',
  className = '',
}: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const generatedId = useId();
  const titleId = `${generatedId}-title`;
  const descId = `${generatedId}-desc`;

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !disableEscapeKey) {
        onClose();
        return;
      }

      if (event.key === 'Tab' && dialogRef.current) {
        const focusable = dialogRef.current.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (focusable.length === 0) return;

        const first = focusable[0];
        const last = focusable[focusable.length - 1];

        if (event.shiftKey) {
          if (document.activeElement === first) {
            last.focus();
            event.preventDefault();
          }
        } else if (document.activeElement === last) {
          first.focus();
          event.preventDefault();
        }
      }
    },
    [disableEscapeKey, onClose]
  );

  useEffect(() => {
    if (!open) return;

    previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
    document.body.style.overflow = 'hidden';
    document.addEventListener('keydown', handleKeyDown);

    const timer = setTimeout(() => {
      const firstFocusable = dialogRef.current?.querySelector<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      (firstFocusable ?? dialogRef.current)?.focus();
    }, 0);

    return () => {
      clearTimeout(timer);
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
      previouslyFocusedRef.current?.focus();
    };
  }, [open, handleKeyDown]);

  if (!open) return null;

  const handleBackdropClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.target === event.currentTarget && !disableBackdropDismiss) {
      onClose();
    }
  };

  return (
    <div
      role="presentation"
      onClick={handleBackdropClick}
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(5, 10, 20, 0.85)',
        backdropFilter: 'blur(8px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 9999,
        padding: '1rem',
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={description ? descId : undefined}
        tabIndex={-1}
        className={`ui-modal ${className}`}
        style={{
          width: '100%',
          maxWidth,
          backgroundColor: 'var(--surface)',
          border: '1px solid var(--border-strong)',
          borderRadius: 'var(--radius)',
          boxShadow: 'var(--shadow), 0 0 30px rgba(0, 0, 0, 0.6)',
          color: 'var(--fg)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          outline: 'none',
        }}
        onClick={(event) => event.stopPropagation()}
      >
        <div
          style={{
            padding: '1.25rem 1.5rem',
            borderBottom: '1px solid var(--border)',
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'space-between',
            gap: '1rem',
          }}
        >
          <div>
            <h2
              id={titleId}
              style={{
                margin: 0,
                fontSize: '1.25rem',
                fontFamily: 'var(--font-display)',
                fontWeight: 600,
                color: 'var(--fg)',
              }}
            >
              {title}
            </h2>
            {description && (
              <p
                id={descId}
                style={{
                  margin: '0.35rem 0 0',
                  fontSize: 'var(--text-sm)',
                  color: 'var(--fg-muted)',
                  lineHeight: 1.4,
                }}
              >
                {description}
              </p>
            )}
          </div>
          {!disableBackdropDismiss && (
            <button
              type="button"
              onClick={onClose}
              aria-label="Close dialog"
              style={{
                background: 'transparent',
                border: 'none',
                color: 'var(--fg-muted)',
                cursor: 'pointer',
                padding: '0.25rem',
                fontSize: '1.25rem',
                lineHeight: 1,
                borderRadius: 'var(--radius-sm)',
              }}
            >
              ×
            </button>
          )}
        </div>

        <div
          style={{
            padding: '1.5rem',
            overflowY: 'auto',
            maxHeight: 'calc(80vh - 120px)',
          }}
        >
          {children}
        </div>
      </div>
    </div>
  );
}

export default Modal;
