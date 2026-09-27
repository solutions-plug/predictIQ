import { renderHook, act } from '@testing-library/react';
import { useReferral } from './useReferral';

const STORAGE_KEY = 'referral_code';

// Mock next/navigation so the hook's usePathname/useSearchParams values can be
// driven by the test, simulating client-side navigation between URLs.
let mockPathname = '/';
let mockSearchParams = new URLSearchParams();

jest.mock('next/navigation', () => ({
  usePathname: () => mockPathname,
  useSearchParams: () => mockSearchParams,
}));

describe('useReferral', () => {
  beforeEach(() => {
    window.localStorage.clear();
    mockPathname = '/';
    mockSearchParams = new URLSearchParams();
  });

  it('captures a ref query param on mount', () => {
    mockPathname = '/';
    mockSearchParams = new URLSearchParams('ref=first-code');

    renderHook(() => useReferral());

    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('first-code');
  });

  it('recaptures when client-side navigation changes the ref param', () => {
    mockPathname = '/';
    mockSearchParams = new URLSearchParams('ref=first-code');

    const { rerender } = renderHook(() => useReferral());

    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('first-code');

    // Simulate a client-side navigation to a different URL with a new ref code.
    act(() => {
      mockPathname = '/markets';
      mockSearchParams = new URLSearchParams('ref=second-code');
    });
    rerender();

    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('second-code');
  });

  it('does not overwrite the stored code when navigation has no ref param', () => {
    mockPathname = '/';
    mockSearchParams = new URLSearchParams('ref=first-code');

    const { rerender } = renderHook(() => useReferral());

    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('first-code');

    act(() => {
      mockPathname = '/markets';
      mockSearchParams = new URLSearchParams();
    });
    rerender();

    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('first-code');
  });
});
