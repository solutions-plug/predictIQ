import { renderHook } from '@testing-library/react';
import { useWalletAddress } from '../useWalletAddress';

const mockUseAccount = jest.fn();

jest.mock('wagmi', () => ({
  useAccount: () => mockUseAccount(),
}));

describe('useWalletAddress', () => {
  beforeEach(() => {
    mockUseAccount.mockReset();
  });

  it('returns the connected address when a wallet is connected', () => {
    mockUseAccount.mockReturnValue({
      address: '0x1234567890abcdef1234567890abcdef12345678',
      isConnected: true,
    });

    const { result } = renderHook(() => useWalletAddress());

    expect(result.current).toBe('0x1234567890abcdef1234567890abcdef12345678');
  });

  it('returns undefined when the wallet is disconnected', () => {
    mockUseAccount.mockReturnValue({
      address: undefined,
      isConnected: false,
    });

    const { result } = renderHook(() => useWalletAddress());

    expect(result.current).toBeUndefined();
  });

  it('returns undefined when connected but no address is available', () => {
    mockUseAccount.mockReturnValue({
      address: undefined,
      isConnected: true,
    });

    const { result } = renderHook(() => useWalletAddress());

    expect(result.current).toBeUndefined();
  });
});
