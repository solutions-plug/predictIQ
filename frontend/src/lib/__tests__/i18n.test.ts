import { i18n } from '../i18n';

describe('i18n', () => {
  beforeEach(() => {
    i18n.setLocale('en');
    localStorage.clear();
  });

  describe('setLocale and getLocale', () => {
    it('should set and get locale', () => {
      i18n.setLocale('en');
      expect(i18n.getLocale()).toBe('en');
    });

    it('should persist locale to localStorage', () => {
      i18n.setLocale('en');
      expect(localStorage.getItem('locale')).toBe('en');
    });

    it('should not set invalid locale', () => {
      i18n.setLocale('en');
      i18n.setLocale('invalid' as any);
      expect(i18n.getLocale()).toBe('en');
    });
  });

  describe('t (translation)', () => {
    it('should return translated string', () => {
      const result = i18n.t('hero.title');
      expect(result).toBe('Decentralized Prediction Markets on Stellar');
    });

    it('should return default value for missing key', () => {
      const result = i18n.t('non.existent.key', 'default');
      expect(result).toBe('default');
    });

    it('should return key as fallback if no default provided', () => {
      const result = i18n.t('non.existent.key');
      expect(result).toBe('non.existent.key');
    });

    it('should handle nested keys', () => {
      const result = i18n.t('features.decentralized.title');
      expect(result).toBe('Fully Decentralized');
    });
  });

  describe('loadLocaleFromStorage', () => {
    it('should load locale from localStorage', () => {
      localStorage.setItem('locale', 'en');
      i18n.loadLocaleFromStorage();
      expect(i18n.getLocale()).toBe('en');
    });

    it('should ignore invalid locale in storage', () => {
      localStorage.setItem('locale', 'invalid');
      i18n.setLocale('en');
      i18n.loadLocaleFromStorage();
      expect(i18n.getLocale()).toBe('en');
    });
  });

  describe('getAvailableLocales', () => {
    it('should return array of available locales', () => {
      const locales = i18n.getAvailableLocales();
      expect(Array.isArray(locales)).toBe(true);
      expect(locales.length).toBeGreaterThan(0);
      expect(locales).toContain('en');
    });

    it('should only offer locales that have translations implemented', () => {
      // Guards against the Locale type promising locales (e.g. 'es', 'fr', 'de')
      // that have no corresponding translations entry, which would render a
      // language selector option that silently falls back to English.
      const locales = i18n.getAvailableLocales();
      locales.forEach((locale) => {
        i18n.setLocale(locale);
        expect(i18n.getLocale()).toBe(locale);
        expect(i18n.t('hero.title')).not.toBe('hero.title');
      });
    });
  });
});
