'use client';

import React, { useState } from 'react';
import { api, ApiError } from '@/lib/api/public-client';
import { LoadingSpinner } from '@/components/LoadingSpinner';

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
    <div style={{ maxWidth: '640px', margin: '0 auto', padding: '2.5rem 1rem' }}>
      <header style={{ marginBottom: '2rem' }}>
        <h1 style={{ fontSize: 'var(--text-2xl, 2rem)', marginBottom: '0.75rem', color: 'var(--fg, #f8fafc)' }}>
          GDPR Data Deletion Request
        </h1>
        <p style={{ color: 'var(--fg-muted, #9fb0cc)', fontSize: 'var(--text-base, 1rem)', lineHeight: 1.6 }}>
          Under the General Data Protection Regulation (GDPR), you have the right to request the permanent erasure of
          your personal data, including newsletter subscriptions, contact records, and associated metadata.
        </p>
      </header>

      {statusMessage && (
        <div
          role="alert"
          aria-live="polite"
          style={{
            padding: '1rem 1.25rem',
            borderRadius: 'var(--radius, 14px)',
            marginBottom: '1.5rem',
            backgroundColor: statusMessage.type === 'success' ? 'rgba(52, 211, 153, 0.15)' : 'rgba(248, 113, 113, 0.15)',
            border: `1px solid ${statusMessage.type === 'success' ? 'var(--success, #34d399)' : 'var(--destructive, #f87171)'}`,
            color: statusMessage.type === 'success' ? 'var(--success, #34d399)' : 'var(--destructive, #f87171)',
          }}
        >
          <strong>{statusMessage.type === 'success' ? 'Success: ' : 'Error: '}</strong>
          {statusMessage.text}
        </div>
      )}

      <section
        aria-labelledby="deletion-warning-heading"
        style={{
          backgroundColor: 'var(--surface, #111a2e)',
          border: '1px solid var(--border, #22304d)',
          borderRadius: 'var(--radius, 14px)',
          padding: '1.75rem',
          marginBottom: '2rem',
        }}
      >
        <h2 id="deletion-warning-heading" style={{ fontSize: 'var(--text-lg, 1.25rem)', color: 'var(--destructive, #f87171)', marginBottom: '0.75rem' }}>
          ⚠️ Irreversible Action
        </h2>
        <p style={{ color: 'var(--fg-muted, #9fb0cc)', marginBottom: '1.25rem', lineHeight: 1.5 }}>
          This operation is immediate and permanently removes your email and history from our systems. Once deleted, this action cannot be undone.
        </p>

        <form onSubmit={handleOpenModal} noValidate>
          <div style={{ marginBottom: '1.25rem' }}>
            <label
              htmlFor="gdpr-delete-email"
              style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 600, color: 'var(--fg, #f8fafc)' }}
            >
              Subscriber Email Address <span aria-hidden="true" style={{ color: 'var(--destructive, #f87171)' }}>*</span>
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
              style={{
                width: '100%',
                padding: '0.75rem 1rem',
                backgroundColor: 'var(--bg-deep, #070b16)',
                border: emailError ? '1px solid var(--destructive, #f87171)' : '1px solid var(--border-strong, #33436a)',
                borderRadius: 'var(--radius-sm, 8px)',
                color: 'var(--fg, #f8fafc)',
                fontSize: '1rem',
              }}
            />
            {emailError && (
              <span id="gdpr-email-error" role="alert" style={{ display: 'block', color: 'var(--destructive, #f87171)', marginTop: '0.5rem', fontSize: '0.875rem' }}>
                {emailError}
              </span>
            )}
          </div>

          <button
            type="submit"
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '0.5rem',
              backgroundColor: 'var(--destructive, #f87171)',
              color: '#0f172a',
              fontWeight: 600,
              padding: '0.75rem 1.5rem',
              borderRadius: 'var(--radius-pill, 999px)',
              border: 'none',
              cursor: 'pointer',
              fontSize: '1rem',
              transition: 'opacity 0.2s ease',
            }}
          >
            Request Permanent Deletion
          </button>
        </form>
      </section>

      {/* Confirmation Modal */}
      {isModalOpen && (
        <div
          role="presentation"
          style={{
            position: 'fixed',
            inset: 0,
            backgroundColor: 'rgba(0, 0, 0, 0.75)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '1rem',
            zIndex: 1000,
            backdropFilter: 'blur(4px)',
          }}
          onClick={(e) => {
            if (e.target === e.currentTarget) handleCloseModal();
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-modal-title"
            aria-describedby="confirm-modal-desc"
            style={{
              backgroundColor: 'var(--surface, #111a2e)',
              border: '1px solid var(--destructive, #f87171)',
              borderRadius: 'var(--radius, 14px)',
              padding: '2rem',
              maxWidth: '520px',
              width: '100%',
              boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.5)',
            }}
          >
            <h2
              id="confirm-modal-title"
              style={{
                fontSize: 'var(--text-xl, 1.5rem)',
                color: 'var(--destructive, #f87171)',
                marginBottom: '1rem',
              }}
            >
              Confirm Permanent Deletion
            </h2>

            <p id="confirm-modal-desc" style={{ color: 'var(--fg-muted, #9fb0cc)', marginBottom: '1rem', lineHeight: 1.5 }}>
              You are about to permanently delete all GDPR-covered data for <strong>{email}</strong>. This action cannot be reversed.
            </p>

            <div style={{ marginBottom: '1.5rem' }}>
              <label
                htmlFor="gdpr-confirm-phrase"
                style={{
                  display: 'block',
                  marginBottom: '0.5rem',
                  fontSize: '0.9rem',
                  color: 'var(--fg, #f8fafc)',
                  fontWeight: 600,
                }}
              >
                To confirm, type{' '}
                <code
                  style={{
                    backgroundColor: 'var(--bg-deep, #070b16)',
                    padding: '0.2rem 0.4rem',
                    borderRadius: '4px',
                    color: 'var(--gold, #f59e0b)',
                    userSelect: 'all',
                  }}
                >
                  {REQUIRED_CONFIRMATION_PHRASE}
                </code>{' '}
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
                style={{
                  width: '100%',
                  padding: '0.75rem 1rem',
                  backgroundColor: 'var(--bg-deep, #070b16)',
                  border: '1px solid var(--border-strong, #33436a)',
                  borderRadius: 'var(--radius-sm, 8px)',
                  color: 'var(--fg, #f8fafc)',
                  fontFamily: 'monospace',
                  fontSize: '0.9rem',
                }}
              />
            </div>

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
              <button
                type="button"
                onClick={handleCloseModal}
                disabled={isLoading}
                style={{
                  padding: '0.625rem 1.25rem',
                  borderRadius: 'var(--radius-pill, 999px)',
                  backgroundColor: 'transparent',
                  border: '1px solid var(--border-strong, #33436a)',
                  color: 'var(--fg-muted, #9fb0cc)',
                  cursor: isLoading ? 'not-allowed' : 'pointer',
                  fontWeight: 500,
                }}
              >
                Cancel
              </button>

              <button
                type="button"
                onClick={handleConfirmDeletion}
                disabled={confirmationInput !== REQUIRED_CONFIRMATION_PHRASE || isLoading}
                aria-disabled={confirmationInput !== REQUIRED_CONFIRMATION_PHRASE || isLoading}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '0.5rem',
                  padding: '0.625rem 1.25rem',
                  borderRadius: 'var(--radius-pill, 999px)',
                  backgroundColor:
                    confirmationInput === REQUIRED_CONFIRMATION_PHRASE && !isLoading
                      ? 'var(--destructive, #f87171)'
                      : 'var(--border, #22304d)',
                  color:
                    confirmationInput === REQUIRED_CONFIRMATION_PHRASE && !isLoading
                      ? '#0f172a'
                      : 'var(--fg-subtle, #6b7c9c)',
                  border: 'none',
                  fontWeight: 600,
                  cursor: confirmationInput === REQUIRED_CONFIRMATION_PHRASE && !isLoading ? 'pointer' : 'not-allowed',
                  transition: 'background-color 0.2s ease',
                }}
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
