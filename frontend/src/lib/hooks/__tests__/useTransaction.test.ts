import { act, renderHook } from '@testing-library/react';
import { useTransaction } from '../useTransaction';

describe('useTransaction', () => {
  it('starts in the idle state', () => {
    const { result } = renderHook(() => useTransaction());

    expect(result.current.status).toBe('idle');
    expect(result.current.hash).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.isPending).toBe(false);
  });

  it('transitions through pending -> signed -> submitted -> confirmed', async () => {
    const { result } = renderHook(() => useTransaction());

    let runPromise: Promise<unknown>;
    act(() => {
      runPromise = result.current.run(async (tx) => {
        tx.setSigned('0xsigned');
        tx.setSubmitted('0xsubmitted');
        tx.setConfirmed('0xconfirmed');
        return '0xconfirmed';
      });
    });

    expect(result.current.status).toBe('pending');
    expect(result.current.isPending).toBe(true);

    await act(async () => {
      await runPromise;
    });

    expect(result.current.status).toBe('confirmed');
    expect(result.current.hash).toBe('0xconfirmed');
    expect(result.current.error).toBeNull();
    expect(result.current.isPending).toBe(false);
  });

  it('records a rejected state with the error', async () => {
    const { result } = renderHook(() => useTransaction());
    const failure = new Error('boom');

    await act(async () => {
      await result.current.run(async () => {
        throw failure;
      });
    });

    expect(result.current.status).toBe('rejected');
    expect(result.current.error).toBe(failure);
    expect(result.current.isPending).toBe(false);
  });

  it('resets to idle when the error is a user rejection', async () => {
    const { result } = renderHook(() => useTransaction());
    const rejection = Object.assign(new Error('User rejected the request'), {
      code: 4001,
    });

    await act(async () => {
      await result.current.run(async () => {
        throw rejection;
      });
    });

    expect(result.current.status).toBe('idle');
    expect(result.current.error).toBeNull();
    expect(result.current.isPending).toBe(false);
  });

  it('ignores a stale run after reset()', async () => {
    const { result } = renderHook(() => useTransaction());
    let resolveRun: (value: string) => void = () => {};

    act(() => {
      result.current.run(
        () =>
          new Promise<string>((resolve) => {
            resolveRun = resolve;
          }),
      );
    });

    expect(result.current.status).toBe('pending');

    act(() => {
      result.current.reset();
    });

    expect(result.current.status).toBe('idle');

    await act(async () => {
      resolveRun('0xstale');
    });

    expect(result.current.status).toBe('idle');
    expect(result.current.hash).toBeNull();
  });

  it('ignores a stale run after a newer run() starts', async () => {
    const { result } = renderHook(() => useTransaction());
    let resolveFirst: (value: string) => void = () => {};

    act(() => {
      result.current.run(
        () =>
          new Promise<string>((resolve) => {
            resolveFirst = resolve;
          }),
      );
    });

    await act(async () => {
      await result.current.run(async () => '0xsecond');
    });

    expect(result.current.status).toBe('confirmed');
    expect(result.current.hash).toBe('0xsecond');

    await act(async () => {
      resolveFirst('0xfirst');
    });

    expect(result.current.status).toBe('confirmed');
    expect(result.current.hash).toBe('0xsecond');
  });
});
