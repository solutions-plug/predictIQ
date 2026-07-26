import { renderHook, act, waitFor } from '@testing-library/react';
import { i18n } from '../../i18n';
import { useI18n } from '../useI18n';

describe('useI18n', () => {
  beforeEach(() => {
    localStorage.clear();
    i18n.setLocale('en');
    document.documentElement.lang = '';
  });

  it('should set document.documentElement.lang to the initial locale', async () => {
    const { result } = renderHook(() => useI18n());

    await waitFor(() => expect(result.current.locale).toBe('en'));

    expect(document.documentElement.lang).toBe('en');
  });

  it('should update document.documentElement.lang when the locale changes', async () => {
    const { result } = renderHook(() => useI18n());

    await waitFor(() => expect(result.current.locale).toBe('en'));

    act(() => {
      result.current.setLocale('es');
    });

    await waitFor(() => expect(result.current.locale).toBe('es'));
    expect(document.documentElement.lang).toBe('es');
  });
});
