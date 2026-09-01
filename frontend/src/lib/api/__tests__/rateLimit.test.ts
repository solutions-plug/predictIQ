import {
  onRateLimited,
  reportRateLimited,
  rateLimitRemainingSeconds,
  isRateLimited,
  _resetRateLimitForTests,
} from '../rateLimit';

describe('rate-limit signal bus (#1339)', () => {
  beforeEach(() => {
    jest.useFakeTimers();
    _resetRateLimitForTests();
  });
  afterEach(() => {
    _resetRateLimitForTests();
    jest.useRealTimers();
  });

  it('opens a cooldown window from Retry-After and reports the remaining seconds', () => {
    const listener = jest.fn();
    onRateLimited(listener);

    reportRateLimited(30);

    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith(30);
    expect(isRateLimited()).toBe(true);
    expect(rateLimitRemainingSeconds()).toBe(30);
  });

  it('coalesces parallel 429s into a single window (one notification)', () => {
    const listener = jest.fn();
    onRateLimited(listener);

    // three requests hit 429 at the same instant
    reportRateLimited(30);
    reportRateLimited(30);
    reportRateLimited(25);

    expect(listener).toHaveBeenCalledTimes(1);
    expect(rateLimitRemainingSeconds()).toBe(30);
  });

  it('extends the window when a later 429 reaches further out', () => {
    const listener = jest.fn();
    onRateLimited(listener);

    reportRateLimited(10);
    jest.advanceTimersByTime(3000);
    reportRateLimited(20); // ends further out than the remaining ~7s

    expect(listener).toHaveBeenCalledTimes(2);
    expect(rateLimitRemainingSeconds()).toBe(20);
  });

  it('counts down and clears when the window elapses', () => {
    reportRateLimited(5);
    expect(rateLimitRemainingSeconds()).toBe(5);

    jest.advanceTimersByTime(5000);
    expect(rateLimitRemainingSeconds()).toBe(0);
    expect(isRateLimited()).toBe(false);
  });

  it('treats a missing/invalid Retry-After as a 1s window', () => {
    reportRateLimited(NaN);
    expect(rateLimitRemainingSeconds()).toBe(1);
  });
});
