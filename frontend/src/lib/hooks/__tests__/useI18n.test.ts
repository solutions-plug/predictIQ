import { renderHook, act, waitFor } from '@testing-library/react';
import { useI18n } from '../useI18n';
import { i18n } from '../../i18n';

describe('useI18n', () => {
  beforeEach(() => {
    // Reset singleton and storage before each test
    i18n.setLocale('en');
    localStorage.clear();
    document.documentElement.lang = '';
  });

  it('initialises with en locale', async () => {
    const { result } = renderHook(() => useI18n());

    await waitFor(() => expect(result.current.locale).toBe('en'));
    expect(i18n.getLocale()).toBe('en');
  });

  it('sets document.documentElement.lang to the initial locale', async () => {
    const { result } = renderHook(() => useI18n());

    await waitFor(() => expect(result.current.locale).toBe('en'));
    expect(document.documentElement.lang).toBe('en');
  });

  describe('setLocale with implemented locale (en)', () => {
    it('updates React state when locale is implemented', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      act(() => {
        result.current.setLocale('en');
      });

      expect(result.current.locale).toBe('en');
      expect(i18n.getLocale()).toBe('en');
    });

    it('keeps React state and singleton in sync for en', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      act(() => {
        result.current.setLocale('en');
      });

      // Both sides agree
      expect(result.current.locale).toBe(i18n.getLocale());
    });
  });

  describe('setLocale with unimplemented locales (es, fr, de)', () => {
    it('does NOT update React state when locale is unimplemented (es)', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      act(() => {
        result.current.setLocale('es');
      });

      // React state must stay at 'en' — no desync
      expect(result.current.locale).toBe('en');
    });

    it('does NOT update React state when locale is unimplemented (fr)', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      act(() => {
        result.current.setLocale('fr');
      });

      expect(result.current.locale).toBe('en');
    });

    it('does NOT update React state when locale is unimplemented (de)', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      act(() => {
        result.current.setLocale('de');
      });

      expect(result.current.locale).toBe('en');
    });

    it('keeps React state and i18n singleton in sync when unimplemented locale is requested', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      act(() => {
        result.current.setLocale('es');
      });

      // The core invariant: hook-visible locale === what the singleton actually uses
      expect(result.current.locale).toBe(i18n.getLocale());
    });

    it('singleton internal locale stays en after unimplemented locale request', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      act(() => {
        result.current.setLocale('fr');
      });

      expect(i18n.getLocale()).toBe('en');
    });
  });

  describe('t() translations', () => {
    it('returns translation for a known key', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      expect(result.current.t('nav.features')).toBe('Features');
    });

    it('returns fallback for unknown key', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      expect(result.current.t('nonexistent.key', 'fallback')).toBe('fallback');
    });
  });

  describe('availableLocales', () => {
    it('returns at least en', async () => {
      const { result } = renderHook(() => useI18n());
      await waitFor(() => expect(result.current.locale).toBe('en'));

      expect(result.current.availableLocales).toContain('en');
    });
  });
});
