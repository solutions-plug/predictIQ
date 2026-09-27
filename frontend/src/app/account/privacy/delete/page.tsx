'use client';

import React, { useState } from 'react';
import { api, ApiError } from '@/lib/api/public-client';
import { LoadingSpinner } from '@/components/LoadingSpinner';
import './delete.css';

const REQUIRED_CONFIRMATION_PHRASE = 'DELETE MY DATA PERMANENTLY';

export default function PrivacyDeletePage() {
  const [email, setEmail] = useState('');
  const [emailError, setEmailError] = useState('');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [confirmationInput, setConfirmationInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [statusMessage, setStatusMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

  const handleOpenModal = (e: React.FormEvent) => {
    e.preventDefault();
    if (!email) {
      setEmailError('Email is required.');
      return;
    }
    if (!emailRegex.test(email)) {
      setEmailError('Please enter a valid email address.');
      return;
    }
    setEmailError('');
    setStatusMessage(null);
    setConfirmationInput('');
    setIsModalOpen(true);
  };

  const handleCloseModal = () => {
    if (isLoading) return;
    setIsModalOpen(false);
    setConfirmationInput('');
  };

  const handleConfirmDeletion = async () => {
    if (confirmationInput !== REQUIRED_CONFIRMATION_PHRASE || isLoading) {
      return;
    }

    setIsLoading(true);
    setStatusMessage(null);

    try {
      const result = await api.newsletterGdprDelete(email);
      if (result && result.success !== false) {
        setStatusMessage({
          type: 'success',
          text: result.message || 'Your data has been permanently deleted according to GDPR regulations.',
        });
        setIsModalOpen(false);
        setEmail('');
        setConfirmationInput('');
      } else {
        setStatusMessage({
          type: 'error',
          text: result?.message || 'Failed to delete data. Please check the email and try again.',
        });
        setIsModalOpen(false);
      }
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : err instanceof Error ? err.message : 'An error occurred during deletion.';
      setStatusMessage({
        type: 'error',
        text: msg,
      });
      setIsModalOpen(false);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="gdpr-delete-page">
      <header className="gdpr-delete-page__header">
        <h1 className="gdpr-delete-page__title">GDPR Data Deletion Request</h1>
        <p className="gdpr-delete-page__intro">
          Under the General Data Protection Regulation (GDPR), you have the right to request the permanent erasure of
          your personal data, including newsletter subscriptions, contact records, and associated metadata.
        </p>
      </header>

      {statusMessage && (
        <div
          role="alert"
          aria-live="polite"
          className={`gdpr-delete-page__banner gdpr-delete-page__banner--${statusMessage.type}`}
        >
          <strong>{statusMessage.type === 'success' ? 'Success: ' : 'Error: '}</strong>
          {statusMessage.text}
        </div>
      )}

      <section aria-labelledby="deletion-warning-heading" className="gdpr-delete-page__panel">
        <h2 id="deletion-warning-heading" className="gdpr-delete-page__panel-heading">
          ⚠️ Irreversible Action
        </h2>
        <p className="gdpr-delete-page__panel-copy">
          This operation is immediate and permanently removes your email and history from our systems. Once deleted, this action cannot be undone.
        </p>

        <form onSubmit={handleOpenModal} noValidate>
          <div className="gdpr-delete-page__field">
            <label htmlFor="gdpr-delete-email" className="gdpr-delete-page__label">
              Subscriber Email Address <span aria-hidden="true" className="gdpr-delete-page__required">*</span>
            </label>
            <input
              id="gdpr-delete-email"
              type="email"
              required
              aria-required="true"
              aria-invalid={!!emailError}
              aria-describedby={emailError ? 'gdpr-email-error' : undefined}
              value={email}
              onChange={(e) => {
                setEmail(e.target.value);
                if (emailError) setEmailError('');
              }}
              placeholder="you@example.com"
              className={`gdpr-delete-page__input ${emailError ? 'gdpr-delete-page__input--error' : ''}`}
            />
            {emailError && (
              <span id="gdpr-email-error" role="alert" className="gdpr-delete-page__field-error">
                {emailError}
              </span>
            )}
          </div>

          <button type="submit" className="gdpr-delete-page__submit">
            Request Permanent Deletion
          </button>
        </form>
      </section>

      {/* Confirmation Modal */}
      {isModalOpen && (
        <div
          role="presentation"
          className="gdpr-delete-page__modal-backdrop"
          onClick={(e) => {
            if (e.target === e.currentTarget) handleCloseModal();
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-modal-title"
            aria-describedby="confirm-modal-desc"
            className="gdpr-delete-page__modal"
          >
            <h2 id="confirm-modal-title" className="gdpr-delete-page__modal-title">
              Confirm Permanent Deletion
            </h2>

            <p id="confirm-modal-desc" className="gdpr-delete-page__modal-desc">
              You are about to permanently delete all GDPR-covered data for <strong>{email}</strong>. This action cannot be reversed.
            </p>

            <div className="gdpr-delete-page__field">
              <label htmlFor="gdpr-confirm-phrase" className="gdpr-delete-page__modal-label">
                To confirm, type{' '}
                <code className="gdpr-delete-page__code">{REQUIRED_CONFIRMATION_PHRASE}</code>{' '}
                below:
              </label>
              <input
                id="gdpr-confirm-phrase"
                type="text"
                autoComplete="off"
                data-lpignore="true"
                data-1p-ignore="true"
                data-form-type="other"
                spellCheck={false}
                value={confirmationInput}
                onChange={(e) => setConfirmationInput(e.target.value)}
                placeholder={REQUIRED_CONFIRMATION_PHRASE}
                disabled={isLoading}
                className="gdpr-delete-page__input gdpr-delete-page__input--mono"
              />
            </div>

            <div className="gdpr-delete-page__modal-actions">
              <button
                type="button"
                onClick={handleCloseModal}
                disabled={isLoading}
                className="gdpr-delete-page__cancel-btn"
              >
                Cancel
              </button>

              <button
                type="button"
                onClick={handleConfirmDeletion}
                disabled={confirmationInput !== REQUIRED_CONFIRMATION_PHRASE || isLoading}
                aria-disabled={confirmationInput !== REQUIRED_CONFIRMATION_PHRASE || isLoading}
                className="gdpr-delete-page__confirm-btn"
              >
                {isLoading ? (
                  <>
                    <LoadingSpinner size="small" aria-label="Deleting data..." />
                    <span>Deleting...</span>
                  </>
                ) : (
                  'Permanently Delete'
                )}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
