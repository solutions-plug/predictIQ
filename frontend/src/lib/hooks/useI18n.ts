import { useState, useEffect } from 'react';
import { i18n, type Locale } from '../i18n';

/**
 * Hook for using i18n in React components
 */
export function useI18n() {
  const [locale, setLocaleState] = useState<Locale>('en');

  useEffect(() => {
    i18n.loadLocaleFromStorage();
    setLocaleState(i18n.getLocale());
  }, []);

  useEffect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.lang = locale;
    }
  }, [locale]);

  const setLocale = (newLocale: Locale) => {
    const applied = i18n.setLocale(newLocale);
    if (applied) {
      setLocaleState(newLocale);
    }
  };

  const t = (key: string, defaultValue?: string) => {
    return i18n.t(key, defaultValue);
  };

  return {
    locale,
    setLocale,
    t,
    availableLocales: i18n.getAvailableLocales(),
  };
}
