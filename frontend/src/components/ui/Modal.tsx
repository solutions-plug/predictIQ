'use client';

/**
 * Modal — shared design-system dialog primitive (#1318, consolidated in #1596).
 *
 * Two ad-hoc Modal implementations already existed in this codebase
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
 *
 * Styled via classNames (src/styles/ui.css) — see that file's header
 * comment for why inline `style` props don't work here. `maxWidth` maps
 * to a small fixed set of width modifier classes rather than an inline
 * style for the same reason. Callers that previously relied on the
 * legacy modals' own chrome (e.g. admin/market-specific styling) pass
 * `className` to layer their styles on top of the shared primitive
 * instead of reimplementing the dialog.
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
  /**
   * Optional footer slot. The legacy components/Modal.tsx and
   * components/admin/Modal.tsx rendered their action buttons in a
   * dedicated footer region; callers migrating from those can pass the
   * same buttons here so the shared primitive covers their surface area
   * without a reimplementation.
   */
  footer?: React.ReactNode;
}

export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  disableBackdropDismiss = false,
  disableEscapeKey = false,
  maxWidth,
  className = '',
  footer,
}: ModalProps) {
  const widthClass = maxWidth === '540px' ? 'ui-modal--w-540' : '';
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
    <div role="presentation" onClick={handleBackdropClick} className="ui-modal-backdrop">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={description ? descId : undefined}
        tabIndex={-1}
        className={`ui-modal ${widthClass} ${className}`}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="ui-modal__header">
          <div>
            <h2 id={titleId} className="ui-modal__title">
              {title}
            </h2>
            {description && (
              <p id={descId} className="ui-modal__desc">
                {description}
              </p>
            )}
          </div>
          {!disableBackdropDismiss && (
            <button type="button" onClick={onClose} aria-label="Close dialog" className="ui-modal__close">
              ×
            </button>
          )}
        </div>

        <div className="ui-modal__body">{children}</div>

        {footer && <div className="ui-modal__footer">{footer}</div>}
      </div>
    </div>
  );
}

export default Modal;
