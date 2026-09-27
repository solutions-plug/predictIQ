'use client';

import React, { ReactNode, ReactElement } from 'react';

type FallbackRenderer = (reset: () => void) => ReactElement;

interface Props {
  children: ReactNode;
  fallback?: ReactElement | FallbackRenderer;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
  section?: string;
  reportIssueUrl?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
    this.reset = this.reset.bind(this);
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    const isDevelopment = process.env.NODE_ENV === 'development';
    if (isDevelopment) {
      console.error('Error caught by boundary:', error, errorInfo);
    } else {
      console.error('Error caught by boundary (production):', errorInfo);
    }
    this.props.onError?.(error, errorInfo);
  }

  // Soft-resets the boundary so children remount and retry on their own,
  // instead of forcing a full page reload for recoverable errors.
  reset() {
    this.setState({ hasError: false, error: null });
  }

  render() {
    if (this.state.hasError) {
      if (typeof this.props.fallback === 'function') {
        return this.props.fallback(this.reset);
      }

      const reportUrl =
        this.props.reportIssueUrl ||
        'https://github.com/solutions-plug/predictIQ/issues/new';

      const isDevelopment = process.env.NODE_ENV === 'development';
      const shouldShowErrorMessage = isDevelopment && this.state.error?.message;

      return (
        this.props.fallback || (
          <div
            role="alert"
            className="error-boundary-fallback"
            aria-labelledby="error-title"
          >
            <h2 id="error-title">Something went wrong</h2>
            <p>
              {this.props.section
                ? `An error occurred in the ${this.props.section} section.`
                : 'An unexpected error occurred.'}
            </p>
            {shouldShowErrorMessage && (
              <p className="error-details">
                {this.state.error?.message}
              </p>
            )}
            <div className="error-actions">
              <button
                type="button"
                onClick={() => {
                  if (typeof window !== 'undefined') {
                    window.location.reload();
                  }
                }}
                aria-label="Reload the page"
              >
                Reload Page
              </button>
              <a
                href={reportUrl}
                target="_blank"
                rel="noopener noreferrer"
                aria-label="Report this issue"
                className="report-issue-link"
              >
                Report Issue
              </a>
            </div>
          </div>
        )
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
