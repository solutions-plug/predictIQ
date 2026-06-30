import React from 'react';
import { render, screen } from '@testing-library/react';
import { LandingPage } from '../LandingPage';

// Mock the Statistics component to throw an error
jest.mock('../Statistics', () => {
  return {
    Statistics: () => {
      throw new Error('Failed to load statistics');
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
});
