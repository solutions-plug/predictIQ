'use client';

import React, { Suspense, useEffect, useState, useCallback } from 'react';
import Link from 'next/link';
import { useSearchParams } from 'next/navigation';
import { LoadingSpinner } from '@/components/LoadingSpinner';
import { getEnvConfig } from '@/lib/env';

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
    <main className="unsubscribe-page-container" style={{
      maxWidth: '600px',
      margin: '80px auto',
      padding: '32px 24px',
      borderRadius: 'var(--radius, 14px)',
      backgroundColor: 'var(--surface, #111a2e)',
      border: '1px solid var(--border, #22304d)',
      boxShadow: 'var(--shadow-lg, 0 10px 25px -5px rgba(0, 0, 0, 0.5))',
      color: 'var(--fg, #f8fafc)',
      textAlign: 'center',
      fontFamily: 'var(--font-body, system-ui, sans-serif)',
    }}>
      <div style={{ marginBottom: '24px' }}>
        <Link href="/" style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          color: 'var(--fg-muted, #9fb0cc)',
          textDecoration: 'none',
          fontSize: 'var(--text-sm, 0.9375rem)',
          marginBottom: '16px',
        }}>
          ← Back to PredictIQ
        </Link>
        <h1 style={{
          fontFamily: 'var(--font-display, Orbitron, sans-serif)',
          fontSize: 'var(--text-xl, 1.5rem)',
          margin: '0 0 8px',
          color: 'var(--fg, #f8fafc)',
        }}>
          Newsletter Unsubscribe
        </h1>
        <p style={{
          color: 'var(--fg-muted, #9fb0cc)',
          fontSize: 'var(--text-sm, 0.9375rem)',
          margin: 0,
        }}>
          Manage your PredictIQ newsletter subscription preferences
        </p>
      </div>

      {status === 'loading' && (
        <div role="status" aria-live="polite" style={{ padding: '32px 0' }}>
          <LoadingSpinner size="large" aria-label="Processing your unsubscribe request" />
          <p style={{ marginTop: '16px', color: 'var(--fg-muted, #9fb0cc)' }}>
            Processing your unsubscribe request...
          </p>
        </div>
      )}

      {status === 'success' && (
        <div
          role="status"
          aria-live="polite"
          tabIndex={0}
          style={{
            padding: '24px',
            backgroundColor: 'rgba(52, 211, 153, 0.1)',
            border: '1px solid var(--success, #34d399)',
            borderRadius: 'var(--radius-sm, 8px)',
            marginBottom: '24px',
          }}
        >
          <div style={{ fontSize: '2.5rem', marginBottom: '12px' }} aria-hidden="true">
            ✅
          </div>
          <h2 style={{ fontSize: 'var(--text-lg, 1.25rem)', margin: '0 0 8px', color: 'var(--fg, #f8fafc)' }}>
            Unsubscribed Successfully
          </h2>
          <p style={{ margin: '0 0 16px', color: 'var(--fg-muted, #9fb0cc)', lineHeight: 1.5 }}>
            {message || 'You have been successfully unsubscribed from PredictIQ newsletter updates.'}
          </p>
          <p style={{ fontSize: 'var(--text-xs, 0.8125rem)', color: 'var(--fg-subtle, #6b7c9c)', margin: '0 0 20px' }}>
            You will no longer receive promotional and newsletter emails from us.
          </p>
          <Link
            href="/"
            style={{
              display: 'inline-block',
              padding: '10px 20px',
              backgroundColor: 'var(--primary, #f59e0b)',
              color: 'var(--on-primary, #0f172a)',
              borderRadius: 'var(--radius-pill, 999px)',
              textDecoration: 'none',
              fontWeight: 600,
              fontSize: 'var(--text-sm, 0.9375rem)',
            }}
          >
            Return to Home
          </Link>
        </div>
      )}

      {status === 'already-unsubscribed' && (
        <div
          role="status"
          aria-live="polite"
          tabIndex={0}
          style={{
            padding: '24px',
            backgroundColor: 'rgba(139, 92, 246, 0.1)',
            border: '1px solid var(--accent, #8b5cf6)',
            borderRadius: 'var(--radius-sm, 8px)',
            marginBottom: '24px',
          }}
        >
          <div style={{ fontSize: '2.5rem', marginBottom: '12px' }} aria-hidden="true">
            ℹ️
          </div>
          <h2 style={{ fontSize: 'var(--text-lg, 1.25rem)', margin: '0 0 8px', color: 'var(--fg, #f8fafc)' }}>
            Already Unsubscribed
          </h2>
          <p style={{ margin: '0 0 16px', color: 'var(--fg-muted, #9fb0cc)', lineHeight: 1.5 }}>
            {message || 'You are already unsubscribed from our newsletter list.'}
          </p>
          <p style={{ fontSize: 'var(--text-xs, 0.8125rem)', color: 'var(--fg-subtle, #6b7c9c)', margin: '0 0 20px' }}>
            No further emails will be sent to your address.
          </p>
          <Link
            href="/"
            style={{
              display: 'inline-block',
              padding: '10px 20px',
              backgroundColor: 'var(--surface-2, #16223b)',
              color: 'var(--fg, #f8fafc)',
              border: '1px solid var(--border-strong, #33436a)',
              borderRadius: 'var(--radius-pill, 999px)',
              textDecoration: 'none',
              fontWeight: 600,
              fontSize: 'var(--text-sm, 0.9375rem)',
            }}
          >
            Return to Home
          </Link>
        </div>
      )}

      {status === 'error' && (
        <div
          role="alert"
          aria-live="assertive"
          style={{
            padding: '24px',
            backgroundColor: 'rgba(248, 113, 113, 0.1)',
            border: '1px solid var(--destructive, #f87171)',
            borderRadius: 'var(--radius-sm, 8px)',
            marginBottom: '24px',
          }}
        >
          <div style={{ fontSize: '2.5rem', marginBottom: '12px' }} aria-hidden="true">
            ⚠️
          </div>
          <h2 style={{ fontSize: 'var(--text-lg, 1.25rem)', margin: '0 0 8px', color: 'var(--fg, #f8fafc)' }}>
            Unsubscribe Notice
          </h2>
          <p style={{ margin: '0 0 20px', color: 'var(--fg-muted, #9fb0cc)', lineHeight: 1.5 }}>
            {message}
          </p>
          <button
            type="button"
            onClick={() => setStatus('idle')}
            style={{
              padding: '8px 16px',
              backgroundColor: 'transparent',
              color: 'var(--fg, #f8fafc)',
              border: '1px solid var(--border-strong, #33436a)',
              borderRadius: 'var(--radius-sm, 8px)',
              cursor: 'pointer',
              fontWeight: 500,
            }}
          >
            Enter Email Manually
          </button>
        </div>
      )}

      {status === 'idle' && (
        <form onSubmit={handleManualSubmit} noValidate style={{ textAlign: 'left', marginTop: '20px' }}>
          <label
            htmlFor="unsubscribe-email"
            style={{
              display: 'block',
              fontSize: 'var(--text-sm, 0.9375rem)',
              marginBottom: '8px',
              color: 'var(--fg-muted, #9fb0cc)',
            }}
          >
            Email address to unsubscribe:
          </label>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
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
              style={{
                width: '100%',
                padding: '12px 16px',
                borderRadius: 'var(--radius-sm, 8px)',
                backgroundColor: 'var(--surface-2, #16223b)',
                border: validationError ? '1px solid var(--destructive, #f87171)' : '1px solid var(--border, #22304d)',
                color: 'var(--fg, #f8fafc)',
                fontSize: 'var(--text-base, 1rem)',
                boxSizing: 'border-box',
              }}
            />
            {validationError && (
              <span id="unsubscribe-val-error" role="alert" style={{ color: 'var(--destructive, #f87171)', fontSize: 'var(--text-xs, 0.8125rem)' }}>
                {validationError}
              </span>
            )}
            <button
              type="submit"
              style={{
                padding: '12px 20px',
                backgroundColor: 'var(--primary, #f59e0b)',
                color: 'var(--on-primary, #0f172a)',
                border: 'none',
                borderRadius: 'var(--radius-sm, 8px)',
                fontWeight: 600,
                fontSize: 'var(--text-base, 1rem)',
                cursor: 'pointer',
                transition: 'opacity 0.2s',
              }}
            >
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
        <div style={{ textAlign: 'center', padding: '100px 0' }}>
          <LoadingSpinner size="large" aria-label="Loading unsubscribe page" />
        </div>
      }
    >
      <UnsubscribeContent />
    </Suspense>
  );
}
