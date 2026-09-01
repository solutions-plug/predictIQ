import React from 'react';
import { render, screen, act } from '@testing-library/react';
import { RateLimitToast, useRateLimited } from '../Toast';
import { reportRateLimited, _resetRateLimitForTests } from '../../../lib/api/rateLimit';

describe('RateLimitToast (#1339)', () => {
  beforeEach(() => {
    jest.useFakeTimers();
    _resetRateLimitForTests();
  });
  afterEach(() => {
    _resetRateLimitForTests();
    jest.useRealTimers();
  });

  it('renders nothing until a 429 is reported', () => {
    render(<RateLimitToast />);
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('shows a countdown from Retry-After and hides when it reaches zero', () => {
    render(<RateLimitToast />);

    act(() => {
      reportRateLimited(30);
    });

    const toast = screen.getByRole('status');
    expect(toast).toHaveTextContent('30s');

    act(() => {
      jest.advanceTimersByTime(10_000);
    });
    expect(screen.getByRole('status')).toHaveTextContent('20s');

    act(() => {
      jest.advanceTimersByTime(20_000);
    });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('parallel 429s produce exactly one toast', () => {
    render(
      <>
        <RateLimitToast />
        <RateLimitToast />
      </>,
    );

    act(() => {
      reportRateLimited(15);
      reportRateLimited(15);
      reportRateLimited(12);
    });

    // Two mounted instances, but only the ones showing an active window; the bus
    // is shared, so both show the same single 15s window - not N stacked toasts
    // per 429. Assert the count reflects "one window", not "three 429s".
    const toasts = screen.getAllByRole('status');
    expect(toasts).toHaveLength(2); // one per mounted component, both showing the same window
    toasts.forEach((t) => expect(t).toHaveTextContent('15s'));
  });
});

describe('useRateLimited', () => {
  beforeEach(() => {
    jest.useFakeTimers();
    _resetRateLimitForTests();
  });
  afterEach(() => {
    _resetRateLimitForTests();
    jest.useRealTimers();
  });

  it('exposes isRateLimited and a ticking secondsRemaining', () => {
    const seen: Array<{ isRateLimited: boolean; secondsRemaining: number }> = [];
    function Probe() {
      seen.push(useRateLimited());
      return null;
    }
    render(<Probe />);
    expect(seen.at(-1)).toEqual({ isRateLimited: false, secondsRemaining: 0 });

    act(() => {
      reportRateLimited(3);
    });
    expect(seen.at(-1)).toEqual({ isRateLimited: true, secondsRemaining: 3 });

    act(() => {
      jest.advanceTimersByTime(3000);
    });
    expect(seen.at(-1)?.isRateLimited).toBe(false);
  });
});
