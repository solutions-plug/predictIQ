'use client';

import React, { useEffect, useRef, useCallback } from 'react';

export interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  disableBackdropDismiss?: boolean;
  disableEscapeKey?: boolean;
  ariaLabelledBy?: string;
  ariaDescribedBy?: string;
  maxWidth?: string;
  className?: string;
}

export const Modal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  title,
  description,
  children,
  disableBackdropDismiss = false,
  disableEscapeKey = false,
  ariaLabelledBy = 'admin-modal-title',
  ariaDescribedBy = 'admin-modal-desc',
  maxWidth = '560px',
  className = '',
}) => {
  const modalRef = useRef<HTMLDivElement>(null);
  const previousActiveElement = useRef<HTMLElement | null>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (!disableEscapeKey && !disableBackdropDismiss) {
          onClose();
        }
      }

      // Trap focus inside modal
      if (e.key === 'Tab' && modalRef.current) {
        const focusableElements = modalRef.current.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (focusableElements.length === 0) return;

        const firstElement = focusableElements[0];
        const lastElement = focusableElements[focusableElements.length - 1];

        if (e.shiftKey) {
          if (document.activeElement === firstElement) {
            lastElement.focus();
            e.preventDefault();
          }
        } else {
          if (document.activeElement === lastElement) {
            firstElement.focus();
            e.preventDefault();
          }
        }
      }
    },
    [disableEscapeKey, disableBackdropDismiss, onClose]
  );

  useEffect(() => {
    if (isOpen) {
      previousActiveElement.current = document.activeElement as HTMLElement;
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';

      // Focus the first focusable item in modal
      setTimeout(() => {
        if (modalRef.current) {
          const firstFocusable = modalRef.current.querySelector<HTMLElement>(
            'button, input, select, textarea, [tabindex]:not([tabindex="-1"])'
          );
          if (firstFocusable) {
            firstFocusable.focus();
          } else {
            modalRef.current.focus();
          }
        }
      }, 50);
    } else {
      document.body.style.overflow = '';
      if (previousActiveElement.current) {
        previousActiveElement.current.focus();
      }
    }

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [isOpen, handleKeyDown]);

  if (!isOpen) return null;

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) {
      if (!disableBackdropDismiss) {
        onClose();
      }
    }
  };

  const widthClass = maxWidth === '540px' ? 'modal-container--w-540' : '';

  return (
    <div className="modal-overlay" onClick={handleBackdropClick} role="presentation">
      <div
        ref={modalRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={ariaLabelledBy}
        aria-describedby={description ? ariaDescribedBy : undefined}
        tabIndex={-1}
        className={`modal-container ${widthClass} ${className}`}
      >
        <div className="modal-header">
          <div>
            <h2 id={ariaLabelledBy}>{title}</h2>
            {description && <p id={ariaDescribedBy}>{description}</p>}
          </div>
          {!disableBackdropDismiss && (
            <button type="button" onClick={onClose} aria-label="Close dialog" className="modal-close">
              ×
            </button>
          )}
        </div>

        <div className="modal-body">{children}</div>
      </div>
    </div>
  );
};
