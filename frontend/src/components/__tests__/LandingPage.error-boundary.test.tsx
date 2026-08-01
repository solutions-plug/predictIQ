import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { LandingPage } from '../LandingPage';

// Mock the Statistics component. Controlled by `mockShouldThrow` so tests can
// simulate recovery after a retry (jest hoists jest.mock factories, so the
// controlling variable must be prefixed with "mock" to be referenced here).
let mockShouldThrow = true;
jest.mock('../Statistics', () => {
  return {
    Statistics: () => {
      if (mockShouldThrow) {
        throw new Error('Failed to load statistics');
      }
      return <div>Statistics loaded</div>;
    },
  };
});

// Mock the i18n hook
jest.mock('../../lib/hooks/useI18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
    locale: 'en',
    setLocale: jest.fn(),
    availableLocales: ['en', 'es'],
  }),
}));

// Mock the dark mode hook
jest.mock('../../lib/hooks/useDarkMode', () => ({
  useDarkMode: () => ({
    isDarkMode: false,
    toggleDarkMode: jest.fn(),
  }),
}));

describe('LandingPage with ErrorBoundary', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockShouldThrow = true;
  });

  it('should render error fallback when Statistics throws', () => {
    render(<LandingPage />);

    expect(screen.getByText('Unable to load statistics at this time. Please try again later.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /retry loading statistics/i })).toBeInTheDocument();
  });

  it('should display error message with role alert', () => {
    render(<LandingPage />);

    const errorMessage = screen.getByRole('alert');
    expect(errorMessage).toBeInTheDocument();
    expect(errorMessage).toHaveTextContent('Unable to load statistics at this time');
  });

  it('should still render other sections when Statistics fails', () => {
    render(<LandingPage />);

    expect(screen.getByText('hero.title')).toBeInTheDocument();
    expect(screen.getByText('features.heading')).toBeInTheDocument();
  });

  it('should have accessible statistics section heading', () => {
    render(<LandingPage />);

    const heading = screen.getByText('Platform Statistics');
    expect(heading).toBeInTheDocument();
    expect(heading.tagName).toBe('H2');
  });

  it('retries via a soft reset instead of reloading the page', () => {
    render(<LandingPage />);

    // Once Statistics recovers, the next render succeeds. A real
    // window.location.reload() would be a no-op in jsdom and could never
    // produce this text, so seeing it proves the boundary was reset in
    // place rather than the page being reloaded.
    mockShouldThrow = false;
    fireEvent.click(screen.getByRole('button', { name: /retry loading statistics/i }));

    expect(screen.getByText('Statistics loaded')).toBeInTheDocument();
  });
});
