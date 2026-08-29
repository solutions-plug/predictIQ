'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import { api, ApiError } from '@/lib/api/public-client';
import { LoadingSpinner } from '@/components/LoadingSpinner';

type Step = 'request-token' | 'verify-and-export' | 'export-complete';

export default function GdprExportPage() {
  const [step, setStep] = useState<Step>('request-token');
  const [email, setEmail] = useState('');
  const [token, setToken] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState('');
  const [statusMessage, setStatusMessage] = useState('');
  const [validationError, setValidationError] = useState('');
  const [exportedData, setExportedData] = useState<Record<string, unknown> | null>(null);

  const validateEmail = (val: string): boolean => {
    const trimmed = val.trim();
    if (!trimmed) {
      setValidationError('Email address is required.');
      return false;
    }
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(trimmed)) {
      setValidationError('Please enter a valid email address.');
      return false;
    }
    setValidationError('');
    return true;
  };

  /**
   * Step 1: Request verification token
   * Strictly uses POST request body: POST /api/v1/newsletter/gdpr/request-token
   * NEVER puts the email in query string or URL.
   */
  const handleRequestToken = async (e: React.FormEvent) => {
    e.preventDefault();
    if (isLoading) return;

    if (!validateEmail(email)) {
      return;
    }

    setIsLoading(true);
    setErrorMessage('');
    setStatusMessage('');

    try {
      const response = await api.newsletterGdprRequestToken({
        email: email.trim(),
      });

      setStatusMessage(
        response.message ||
          'If this email is subscribed, a verification code has been sent to your inbox.'
      );
      setStep('verify-and-export');
    } catch (err: unknown) {
      if (err instanceof ApiError) {
        setErrorMessage(err.message || 'Failed to request verification token. Please try again.');
      } else {
        setErrorMessage(err instanceof Error ? err.message : 'Network error occurred.');
      }
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * Step 2: Submit export request with verification token
   * Strictly uses POST request body: POST /api/v1/newsletter/gdpr/export
   * Email and token are passed in JSON body, never in URL parameters.
   */
  const handleExportData = async (e: React.FormEvent) => {
    e.preventDefault();
    if (isLoading) return;

    const trimmedToken = token.trim();
    if (!trimmedToken) {
      setValidationError('Verification token is required.');
      return;
    }

    setIsLoading(true);
    setErrorMessage('');

    try {
      const response = await api.newsletterGdprExport({
        email: email.trim(),
        token: trimmedToken,
      });

      if (response.success && response.data) {
        setExportedData(response.data);
        setStep('export-complete');
      } else {
        setErrorMessage(response.message || 'Failed to export data.');
      }
    } catch (err: unknown) {
      if (err instanceof ApiError) {
        setErrorMessage(
          err.message || 'Invalid or expired verification token. Please try again.'
        );
      } else {
        setErrorMessage(err instanceof Error ? err.message : 'Network error occurred.');
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleDownloadJson = () => {
    if (!exportedData) return;
    const blob = new Blob([JSON.stringify(exportedData, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `predictiq-gdpr-data-export.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const handleReset = () => {
    setEmail('');
    setToken('');
    setExportedData(null);
    setErrorMessage('');
    setStatusMessage('');
    setValidationError('');
    setStep('request-token');
  };

  return (
    <main
      className="gdpr-export-container"
      style={{
        maxWidth: '680px',
        margin: '60px auto',
        padding: '36px 28px',
        borderRadius: 'var(--radius, 14px)',
        backgroundColor: 'var(--surface, #111a2e)',
        border: '1px solid var(--border, #22304d)',
        boxShadow: 'var(--shadow-lg, 0 10px 25px -5px rgba(0, 0, 0, 0.5))',
        color: 'var(--fg, #f8fafc)',
        fontFamily: 'var(--font-body, system-ui, sans-serif)',
      }}
    >
      <div style={{ marginBottom: '28px' }}>
        <Link
          href="/"
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            color: 'var(--fg-muted, #9fb0cc)',
            textDecoration: 'none',
            fontSize: 'var(--text-sm, 0.9375rem)',
            marginBottom: '16px',
          }}
        >
          ← Back to PredictIQ
        </Link>
        <h1
          style={{
            fontFamily: 'var(--font-display, Orbitron, sans-serif)',
            fontSize: 'var(--text-2xl, 1.75rem)',
            margin: '0 0 10px',
            color: 'var(--fg, #f8fafc)',
          }}
        >
          GDPR Data Export
        </h1>
        <p
          style={{
            color: 'var(--fg-muted, #9fb0cc)',
            fontSize: 'var(--text-base, 1rem)',
            lineHeight: 1.5,
            margin: 0,
          }}
        >
          Request an export of all newsletter and account data stored with PredictIQ under GDPR /
          privacy regulations.
        </p>
      </div>

      {/* Progress Indicators */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          marginBottom: '28px',
          paddingBottom: '16px',
          borderBottom: '1px solid var(--border, #22304d)',
        }}
        aria-label="Progress steps"
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            color:
              step === 'request-token'
                ? 'var(--primary, #f59e0b)'
                : 'var(--fg-muted, #9fb0cc)',
            fontWeight: step === 'request-token' ? 600 : 400,
            fontSize: 'var(--text-sm, 0.9375rem)',
          }}
        >
          <span>1. Request Code</span>
        </div>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            color:
              step === 'verify-and-export'
                ? 'var(--primary, #f59e0b)'
                : 'var(--fg-muted, #9fb0cc)',
            fontWeight: step === 'verify-and-export' ? 600 : 400,
            fontSize: 'var(--text-sm, 0.9375rem)',
          }}
        >
          <span>2. Verify & Export</span>
        </div>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            color:
              step === 'export-complete'
                ? 'var(--success, #34d399)'
                : 'var(--fg-muted, #9fb0cc)',
            fontWeight: step === 'export-complete' ? 600 : 400,
            fontSize: 'var(--text-sm, 0.9375rem)',
          }}
        >
          <span>3. Complete</span>
        </div>
      </div>

      {errorMessage && (
        <div
          role="alert"
          aria-live="assertive"
          style={{
            padding: '16px',
            backgroundColor: 'rgba(248, 113, 113, 0.1)',
            border: '1px solid var(--destructive, #f87171)',
            borderRadius: 'var(--radius-sm, 8px)',
            marginBottom: '24px',
            color: 'var(--fg, #f8fafc)',
            fontSize: 'var(--text-sm, 0.9375rem)',
            display: 'flex',
            alignItems: 'center',
            gap: '12px',
          }}
        >
          <span aria-hidden="true">⚠️</span>
          <span>{errorMessage}</span>
        </div>
      )}

      {statusMessage && (
        <div
          role="status"
          aria-live="polite"
          style={{
            padding: '16px',
            backgroundColor: 'rgba(52, 211, 153, 0.1)',
            border: '1px solid var(--success, #34d399)',
            borderRadius: 'var(--radius-sm, 8px)',
            marginBottom: '24px',
            color: 'var(--fg, #f8fafc)',
            fontSize: 'var(--text-sm, 0.9375rem)',
            display: 'flex',
            alignItems: 'center',
            gap: '12px',
          }}
        >
          <span aria-hidden="true">✉️</span>
          <span>{statusMessage}</span>
        </div>
      )}

      {/* Step 1: Request Token Form */}
      {step === 'request-token' && (
        <form onSubmit={handleRequestToken} noValidate aria-busy={isLoading}>
          <div style={{ marginBottom: '20px' }}>
            <label
              htmlFor="gdpr-email-input"
              style={{
                display: 'block',
                fontSize: 'var(--text-sm, 0.9375rem)',
                marginBottom: '8px',
                color: 'var(--fg-muted, #9fb0cc)',
                fontWeight: 500,
              }}
            >
              Enter the email address associated with your subscription:
            </label>
            <input
              id="gdpr-email-input"
              type="email"
              required
              value={email}
              onChange={(e) => {
                setEmail(e.target.value);
                setValidationError('');
                setErrorMessage('');
              }}
              placeholder="you@example.com"
              disabled={isLoading}
              aria-invalid={!!validationError}
              aria-describedby={validationError ? 'gdpr-email-error' : undefined}
              style={{
                width: '100%',
                padding: '12px 16px',
                borderRadius: 'var(--radius-sm, 8px)',
                backgroundColor: 'var(--surface-2, #16223b)',
                border: validationError
                  ? '1px solid var(--destructive, #f87171)'
                  : '1px solid var(--border, #22304d)',
                color: 'var(--fg, #f8fafc)',
                fontSize: 'var(--text-base, 1rem)',
                boxSizing: 'border-box',
              }}
            />
            {validationError && (
              <span
                id="gdpr-email-error"
                role="alert"
                style={{
                  display: 'block',
                  marginTop: '6px',
                  color: 'var(--destructive, #f87171)',
                  fontSize: 'var(--text-xs, 0.8125rem)',
                }}
              >
                {validationError}
              </span>
            )}
          </div>

          <button
            type="submit"
            disabled={isLoading}
            style={{
              width: '100%',
              padding: '12px 24px',
              backgroundColor: 'var(--primary, #f59e0b)',
              color: 'var(--on-primary, #0f172a)',
              border: 'none',
              borderRadius: 'var(--radius-sm, 8px)',
              fontWeight: 600,
              fontSize: 'var(--text-base, 1rem)',
              cursor: isLoading ? 'not-allowed' : 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
            }}
          >
            {isLoading ? (
              <LoadingSpinner size="small" aria-label="Sending verification code..." />
            ) : (
              'Send Verification Code'
            )}
          </button>
        </form>
      )}

      {/* Step 2: Verify Token and Download Form */}
      {step === 'verify-and-export' && (
        <form onSubmit={handleExportData} noValidate aria-busy={isLoading}>
          <div style={{ marginBottom: '20px' }}>
            <label
              htmlFor="gdpr-token-input"
              style={{
                display: 'block',
                fontSize: 'var(--text-sm, 0.9375rem)',
                marginBottom: '8px',
                color: 'var(--fg-muted, #9fb0cc)',
                fontWeight: 500,
              }}
            >
              Enter the verification code sent to your email:
            </label>
            <input
              id="gdpr-token-input"
              type="text"
              required
              value={token}
              onChange={(e) => {
                setToken(e.target.value);
                setValidationError('');
                setErrorMessage('');
              }}
              placeholder="Paste verification token here"
              disabled={isLoading}
              aria-invalid={!!validationError}
              aria-describedby={validationError ? 'gdpr-token-error' : undefined}
              style={{
                width: '100%',
                padding: '12px 16px',
                borderRadius: 'var(--radius-sm, 8px)',
                backgroundColor: 'var(--surface-2, #16223b)',
                border: validationError
                  ? '1px solid var(--destructive, #f87171)'
                  : '1px solid var(--border, #22304d)',
                color: 'var(--fg, #f8fafc)',
                fontSize: 'var(--text-base, 1rem)',
                boxSizing: 'border-box',
                fontFamily: 'monospace',
              }}
            />
            {validationError && (
              <span
                id="gdpr-token-error"
                role="alert"
                style={{
                  display: 'block',
                  marginTop: '6px',
                  color: 'var(--destructive, #f87171)',
                  fontSize: 'var(--text-xs, 0.8125rem)',
                }}
              >
                {validationError}
              </span>
            )}
          </div>

          <div style={{ display: 'flex', gap: '12px' }}>
            <button
              type="button"
              onClick={() => {
                setStep('request-token');
                setErrorMessage('');
              }}
              disabled={isLoading}
              style={{
                padding: '12px 20px',
                backgroundColor: 'transparent',
                color: 'var(--fg-muted, #9fb0cc)',
                border: '1px solid var(--border, #22304d)',
                borderRadius: 'var(--radius-sm, 8px)',
                fontWeight: 500,
                fontSize: 'var(--text-base, 1rem)',
                cursor: 'pointer',
              }}
            >
              Back
            </button>
            <button
              type="submit"
              disabled={isLoading}
              style={{
                flex: 1,
                padding: '12px 24px',
                backgroundColor: 'var(--primary, #f59e0b)',
                color: 'var(--on-primary, #0f172a)',
                border: 'none',
                borderRadius: 'var(--radius-sm, 8px)',
                fontWeight: 600,
                fontSize: 'var(--text-base, 1rem)',
                cursor: isLoading ? 'not-allowed' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
              }}
            >
              {isLoading ? (
                <LoadingSpinner size="small" aria-label="Verifying token and exporting data..." />
              ) : (
                'Export Data'
              )}
            </button>
          </div>
        </form>
      )}

      {/* Step 3: Complete / Data Preview and Download */}
      {step === 'export-complete' && exportedData && (
        <div role="status" aria-live="polite">
          <div
            style={{
              padding: '20px',
              backgroundColor: 'rgba(52, 211, 153, 0.1)',
              border: '1px solid var(--success, #34d399)',
              borderRadius: 'var(--radius-sm, 8px)',
              marginBottom: '24px',
            }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '10px',
                color: 'var(--success, #34d399)',
                fontWeight: 600,
                marginBottom: '8px',
              }}
            >
              <span>✅</span>
              <span>Data export generated successfully</span>
            </div>
            <p
              style={{
                margin: 0,
                color: 'var(--fg-muted, #9fb0cc)',
                fontSize: 'var(--text-sm, 0.9375rem)',
              }}
            >
              Your data has been compiled and is ready for inspection or download.
            </p>
          </div>

          <div style={{ marginBottom: '24px' }}>
            <h2
              style={{
                fontSize: 'var(--text-base, 1rem)',
                margin: '0 0 8px',
                color: 'var(--fg, #f8fafc)',
              }}
            >
              Exported Data Record
            </h2>
            <pre
              style={{
                padding: '16px',
                backgroundColor: 'var(--surface-2, #16223b)',
                borderRadius: 'var(--radius-sm, 8px)',
                border: '1px solid var(--border, #22304d)',
                overflowX: 'auto',
                fontSize: 'var(--text-xs, 0.8125rem)',
                color: 'var(--fg, #f8fafc)',
                maxHeight: '320px',
              }}
            >
              {JSON.stringify(exportedData, null, 2)}
            </pre>
          </div>

          <div style={{ display: 'flex', gap: '12px' }}>
            <button
              type="button"
              onClick={handleDownloadJson}
              style={{
                flex: 1,
                padding: '12px 24px',
                backgroundColor: 'var(--primary, #f59e0b)',
                color: 'var(--on-primary, #0f172a)',
                border: 'none',
                borderRadius: 'var(--radius-sm, 8px)',
                fontWeight: 600,
                fontSize: 'var(--text-base, 1rem)',
                cursor: 'pointer',
              }}
            >
              Download JSON File
            </button>
            <button
              type="button"
              onClick={handleReset}
              style={{
                padding: '12px 20px',
                backgroundColor: 'transparent',
                color: 'var(--fg-muted, #9fb0cc)',
                border: '1px solid var(--border, #22304d)',
                borderRadius: 'var(--radius-sm, 8px)',
                fontWeight: 500,
                fontSize: 'var(--text-base, 1rem)',
                cursor: 'pointer',
              }}
            >
              New Request
            </button>
          </div>
        </div>
      )}
    </main>
  );
}
