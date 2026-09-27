'use client';

import React, { Suspense, useEffect, useState, useCallback } from 'react';
import Link from 'next/link';
import { useSearchParams } from 'next/navigation';
import { LoadingSpinner } from '@/components/LoadingSpinner';
import { getEnvConfig } from '@/lib/env';
import './unsubscribe.css';

type UnsubscribeStatus = 'idle' | 'loading' | 'success' | 'already-unsubscribed' | 'error';

function UnsubscribeContent() {
  const searchParams = useSearchParams();
  const tokenParam = searchParams.get('token') || '';
  const emailParam = searchParams.get('email') || '';

  const [emailInput, setEmailInput] = useState(emailParam);
  const [status, setStatus] = useState<UnsubscribeStatus>('idle');
  const [message, setMessage] = useState<string>('');
  const [validationError, setValidationError] = useState<string>('');

  const executeUnsubscribe = useCallback(async (token?: string, email?: string) => {
    setStatus('loading');
    setMessage('');
    setValidationError('');

    const targetToken = token || tokenParam;
    const targetEmail = email || emailParam || emailInput;

    try {
      const config = getEnvConfig();
      const baseUrl = config.NEXT_PUBLIC_API_URL.replace(/\/$/, '');

      let res: Response;

      if (targetToken) {
        // Try GET with token first as defined in API router
        res = await fetch(`${baseUrl}/api/v1/newsletter/unsubscribe?token=${encodeURIComponent(targetToken)}`, {
          method: 'GET',
          headers: { 'Accept': 'application/json' },
        });

        // Fallback to POST/DELETE if GET returns 405 Method Not Allowed
        if (res.status === 405) {
          res = await fetch(`${baseUrl}/api/v1/newsletter/unsubscribe`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
            body: JSON.stringify({ token: targetToken, email: targetEmail || undefined }),
          });
        }
      } else if (targetEmail) {
        // Unsubscribe with email body
        res = await fetch(`${baseUrl}/api/v1/newsletter/unsubscribe`, {
          method: 'DELETE',
          headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
          body: JSON.stringify({ email: targetEmail.trim() }),
        });

        if (res.status === 405) {
          res = await fetch(`${baseUrl}/api/v1/newsletter/unsubscribe`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
            body: JSON.stringify({ email: targetEmail.trim() }),
          });
        }
      } else {
        setStatus('idle');
        setValidationError('Please provide a valid token or email address.');
        return;
      }

      let data: { success?: boolean; message?: string; code?: string } = {};
      try {
        data = await res.json();
      } catch {
        // Empty response body
      }

      const msg = data.message || '';
      const isAlreadyUnsubscribed =
        /already\s*(unsubscribed|removed|opted out)/i.test(msg) ||
        data.code === 'ALREADY_UNSUBSCRIBED';

      if (res.ok) {
        if (isAlreadyUnsubscribed) {
          setStatus('already-unsubscribed');
          setMessage(msg || 'You are already unsubscribed from our newsletter.');
        } else {
          setStatus('success');
          setMessage(msg || 'You have been successfully unsubscribed.');
        }
      } else {
        // Check if error response signifies an already-unsubscribed or expired link
        if (
          isAlreadyUnsubscribed ||
          /already\s*(unsubscribed|removed)/i.test(msg)
        ) {
          setStatus('already-unsubscribed');
          setMessage(msg || 'You are already unsubscribed from our newsletter.');
        } else {
          setStatus('error');
          setMessage(
            msg ||
            'We were unable to process your unsubscribe request. The link may have expired or is invalid.'
          );
        }
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Network error occurred.';
      if (/already\s*(unsubscribed|removed)/i.test(errorMsg)) {
        setStatus('already-unsubscribed');
        setMessage(errorMsg);
      } else {
        setStatus('error');
        setMessage(`Unable to reach the server. Please check your connection and try again.`);
      }
    }
  }, [tokenParam, emailParam, emailInput]);

  useEffect(() => {
    // If arriving from an emailed link with a token or email query param, execute on load
    if (tokenParam || emailParam) {
      executeUnsubscribe(tokenParam, emailParam);
    }
  }, [tokenParam, emailParam, executeUnsubscribe]);

  const handleManualSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = emailInput.trim();
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!trimmed || !emailRegex.test(trimmed)) {
      setValidationError('Please enter a valid email address.');
      return;
    }
    executeUnsubscribe(undefined, trimmed);
  };

  return (
    <main className="unsubscribe-page-container">
      <div className="unsubscribe-page__intro">
        <Link href="/" className="unsubscribe-page__back-link">
          ← Back to PredictIQ
        </Link>
        <h1 className="unsubscribe-page__title">Newsletter Unsubscribe</h1>
        <p className="unsubscribe-page__desc">Manage your PredictIQ newsletter subscription preferences</p>
      </div>

      {status === 'loading' && (
        <div role="status" aria-live="polite" className="unsubscribe-page__loading">
          <LoadingSpinner size="large" aria-label="Processing your unsubscribe request" />
          <p className="unsubscribe-page__loading-copy">Processing your unsubscribe request...</p>
        </div>
      )}

      {status === 'success' && (
        <div role="status" aria-live="polite" tabIndex={0} className="unsubscribe-page__panel unsubscribe-page__panel--success">
          <div className="unsubscribe-page__icon" aria-hidden="true">
            ✅
          </div>
          <h2 className="unsubscribe-page__panel-heading">Unsubscribed Successfully</h2>
          <p className="unsubscribe-page__panel-copy">
            {message || 'You have been successfully unsubscribed from PredictIQ newsletter updates.'}
          </p>
          <p className="unsubscribe-page__panel-subcopy">
            You will no longer receive promotional and newsletter emails from us.
          </p>
          <Link href="/" className="unsubscribe-page__cta unsubscribe-page__cta--primary">
            Return to Home
          </Link>
        </div>
      )}

      {status === 'already-unsubscribed' && (
        <div role="status" aria-live="polite" tabIndex={0} className="unsubscribe-page__panel unsubscribe-page__panel--info">
          <div className="unsubscribe-page__icon" aria-hidden="true">
            ℹ️
          </div>
          <h2 className="unsubscribe-page__panel-heading">Already Unsubscribed</h2>
          <p className="unsubscribe-page__panel-copy">
            {message || 'You are already unsubscribed from our newsletter list.'}
          </p>
          <p className="unsubscribe-page__panel-subcopy">No further emails will be sent to your address.</p>
          <Link href="/" className="unsubscribe-page__cta unsubscribe-page__cta--secondary">
            Return to Home
          </Link>
        </div>
      )}

      {status === 'error' && (
        <div role="alert" aria-live="assertive" className="unsubscribe-page__panel unsubscribe-page__panel--error">
          <div className="unsubscribe-page__icon" aria-hidden="true">
            ⚠️
          </div>
          <h2 className="unsubscribe-page__panel-heading">Unsubscribe Notice</h2>
          <p className="unsubscribe-page__panel-copy">{message}</p>
          <button type="button" onClick={() => setStatus('idle')} className="unsubscribe-page__retry-btn">
            Enter Email Manually
          </button>
        </div>
      )}

      {status === 'idle' && (
        <form onSubmit={handleManualSubmit} noValidate className="unsubscribe-page__form">
          <label htmlFor="unsubscribe-email" className="unsubscribe-page__label">
            Email address to unsubscribe:
          </label>
          <div className="unsubscribe-page__form-row">
            <input
              id="unsubscribe-email"
              type="email"
              required
              value={emailInput}
              onChange={(e) => {
                setEmailInput(e.target.value);
                setValidationError('');
              }}
              placeholder="you@example.com"
              aria-invalid={!!validationError}
              aria-describedby={validationError ? 'unsubscribe-val-error' : undefined}
              className={`unsubscribe-page__input ${validationError ? 'unsubscribe-page__input--error' : ''}`}
            />
            {validationError && (
              <span id="unsubscribe-val-error" role="alert" className="unsubscribe-page__field-error">
                {validationError}
              </span>
            )}
            <button type="submit" className="unsubscribe-page__submit">
              Unsubscribe
            </button>
          </div>
        </form>
      )}
    </main>
  );
}

export default function UnsubscribePage() {
  return (
    <Suspense
      fallback={
        <div className="unsubscribe-page__suspense-fallback">
          <LoadingSpinner size="large" aria-label="Loading unsubscribe page" />
        </div>
      }
    >
      <UnsubscribeContent />
    </Suspense>
  );
}
