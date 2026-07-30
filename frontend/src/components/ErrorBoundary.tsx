'use client';

import React, { ReactNode, ReactElement } from 'react';

type FallbackRenderer = (reset: () => void) => ReactElement;

interface Props {
  children: ReactNode;
  fallback?: ReactElement | FallbackRenderer;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
  section?: string;
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
    console.error('Error caught by boundary:', error, errorInfo);
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
            <p className="error-details">
              {this.state.error?.message}
            </p>
            <button 
              onClick={() => window.location.reload()}
              aria-label="Reload the page"
            >
              Reload Page
            </button>
          </div>
        )
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
