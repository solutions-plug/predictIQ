import { render, screen, act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RateLimitToast, useRateLimited } from './Toast';
import * as rateLimit from '../../lib/rateLimit';

function Harness() {
  const { secondsRemaining } = useRateLimited();
  return <span data-testid="remaining">{secondsRemaining}</span>;
}

describe('useRateLimited', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('recomputes the countdown immediately when the tab becomes visible again', () => {
    const remainingSpy = vi
      .spyOn(rateLimit, 'rateLimitRemainingSeconds')
      .mockReturnValue(30);

    render(<Harness />);
    expect(screen.getByTestId('remaining').textContent).toBe('30');

    // Simulate the cooldown elapsing while the tab is backgrounded and the
    // interval is throttled, so no tick has fired to update the countdown.
    remainingSpy.mockReturnValue(5);
    expect(screen.getByTestId('remaining').textContent).toBe('30');

    act(() => {
      Object.defineProperty(document, 'visibilityState', {
        configurable: true,
        get: () => 'visible',
      });
      document.dispatchEvent(new Event('visibilitychange'));
    });

    expect(screen.getByTestId('remaining').textContent).toBe('5');
  });

  it('renders the RateLimitToast with the current countdown', () => {
    vi.spyOn(rateLimit, 'rateLimitRemainingSeconds').mockReturnValue(12);

    render(<RateLimitToast />);

    expect(screen.getByText(/12/)).toBeTruthy();
  });
});
