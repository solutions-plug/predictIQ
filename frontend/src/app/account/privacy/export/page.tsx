'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import { api, ApiError } from '@/lib/api/public-client';
import { LoadingSpinner } from '@/components/LoadingSpinner';
import './export.css';

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
    <main className="gdpr-export-container">
      <div className="gdpr-export__intro">
        <Link href="/" className="gdpr-export__back-link">
          ← Back to PredictIQ
        </Link>
        <h1 className="gdpr-export__title">GDPR Data Export</h1>
        <p className="gdpr-export__desc">
          Request an export of all newsletter and account data stored with PredictIQ under GDPR /
          privacy regulations.
        </p>
      </div>

      {/* Progress Indicators */}
      <div className="gdpr-export__progress" aria-label="Progress steps">
        <div
          className={`gdpr-export__progress-step ${step === 'request-token' ? 'gdpr-export__progress-step--active' : ''}`}
        >
          <span>1. Request Code</span>
        </div>
        <div
          className={`gdpr-export__progress-step ${step === 'verify-and-export' ? 'gdpr-export__progress-step--active' : ''}`}
        >
          <span>2. Verify & Export</span>
        </div>
        <div
          className={`gdpr-export__progress-step ${step === 'export-complete' ? 'gdpr-export__progress-step--success' : ''}`}
        >
          <span>3. Complete</span>
        </div>
      </div>

      {errorMessage && (
        <div role="alert" aria-live="assertive" className="gdpr-export__banner gdpr-export__banner--error">
          <span aria-hidden="true">⚠️</span>
          <span>{errorMessage}</span>
        </div>
      )}

      {statusMessage && (
        <div role="status" aria-live="polite" className="gdpr-export__banner gdpr-export__banner--success">
          <span aria-hidden="true">✉️</span>
          <span>{statusMessage}</span>
        </div>
      )}

      {/* Step 1: Request Token Form */}
      {step === 'request-token' && (
        <form onSubmit={handleRequestToken} noValidate aria-busy={isLoading}>
          <div className="gdpr-export__field">
            <label htmlFor="gdpr-email-input" className="gdpr-export__label">
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
              className={`gdpr-export__input ${validationError ? 'gdpr-export__input--error' : ''}`}
            />
            {validationError && (
              <span id="gdpr-email-error" role="alert" className="gdpr-export__field-error">
                {validationError}
              </span>
            )}
          </div>

          <button type="submit" disabled={isLoading} className="gdpr-export__submit">
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
          <div className="gdpr-export__field">
            <label htmlFor="gdpr-token-input" className="gdpr-export__label">
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
              className={`gdpr-export__input gdpr-export__input--mono ${validationError ? 'gdpr-export__input--error' : ''}`}
            />
            {validationError && (
              <span id="gdpr-token-error" role="alert" className="gdpr-export__field-error">
                {validationError}
              </span>
            )}
          </div>

          <div className="gdpr-export__actions">
            <button
              type="button"
              onClick={() => {
                setStep('request-token');
                setErrorMessage('');
              }}
              disabled={isLoading}
              className="gdpr-export__back-btn"
            >
              Back
            </button>
            <button type="submit" disabled={isLoading} className="gdpr-export__submit gdpr-export__submit--flex">
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
          <div className="gdpr-export__complete-banner">
            <div className="gdpr-export__complete-banner-title">
              <span>✅</span>
              <span>Data export generated successfully</span>
            </div>
            <p className="gdpr-export__complete-banner-copy">
              Your data has been compiled and is ready for inspection or download.
            </p>
          </div>

          <div className="gdpr-export__record">
            <h2 className="gdpr-export__record-heading">Exported Data Record</h2>
            <pre className="gdpr-export__record-pre">{JSON.stringify(exportedData, null, 2)}</pre>
          </div>

          <div className="gdpr-export__actions">
            <button type="button" onClick={handleDownloadJson} className="gdpr-export__submit gdpr-export__submit--flex">
              Download JSON File
            </button>
            <button type="button" onClick={handleReset} className="gdpr-export__back-btn">
              New Request
            </button>
          </div>
        </div>
      )}
    </main>
  );
}
