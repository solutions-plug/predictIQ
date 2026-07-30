/**
 * Simple i18n utility for frontend internationalization.
 * Supports multiple locales with fallback to English.
 */

export type Locale = 'en' | 'es' | 'fr' | 'de';

interface Translations {
  [key: string]: string | Translations;
}

interface LocaleData {
  [locale: string]: Translations;
}

const translations: LocaleData = {
  en: {
    nav: {
      features: 'Features',
      howItWorks: 'How It Works',
      about: 'About',
      contact: 'Contact',
    },
    hero: {
      title: 'Decentralized Prediction Markets on Stellar',
      description: 'Create, bet on, and resolve prediction markets with transparency, security, and fairness powered by blockchain technology.',
      signupHeading: 'Sign up for updates',
      emailLabel: 'Email Address',
      emailPlaceholder: 'you@example.com',
      emailRequired: 'Email is required',
      emailInvalid: 'Please enter a valid email address',
      submitButton: 'Get Early Access',
      subscribedButton: 'Subscribed!',
      successMessage: 'Successfully subscribed to updates!',
    },
    features: {
      heading: 'Key Features',
      decentralized: {
        title: 'Fully Decentralized',
        description: 'No central authority. Markets run on smart contracts with transparent, immutable rules.',
      },
      secure: {
        title: 'Secure & Audited',
        description: 'Smart contracts audited by leading security firms. Your funds are protected by battle-tested code.',
      },
      fast: {
        title: 'Lightning Fast',
        description: 'Built on Stellar for near-instant transactions and minimal fees. Trade without waiting.',
      },
    },
    howItWorks: {
      heading: 'How It Works',
      step1: {
        title: 'Create a Market',
        description: 'Define outcomes and set parameters for your prediction market.',
      },
      step2: {
        title: 'Place Bets',
        description: 'Users bet on outcomes they believe will occur.',
      },
      step3: {
        title: 'Oracle Resolution',
        description: 'Trusted oracles provide real-world data to resolve markets.',
      },
      step4: {
        title: 'Claim Winnings',
        description: 'Winners automatically receive their share of the pool.',
      },
    },
    about: {
      heading: 'About PredictIQ',
      description1: 'PredictIQ is a decentralized prediction market platform built on the Stellar blockchain. We enable anyone to create, participate in, and resolve prediction markets with complete transparency and fairness.',
      description2: 'Our smart contracts are open-source, audited, and designed with security and user experience as top priorities.',
    },
    footer: {
      title: 'PredictIQ',
      tagline: 'Decentralized prediction markets for everyone.',
      linksHeading: 'Links',
      legalHeading: 'Legal',
      documentation: 'Documentation',
      github: 'GitHub',
      discord: 'Discord',
      privacy: 'Privacy Policy',
      terms: 'Terms of Service',
      copyright: '© 2024 PredictIQ. All rights reserved.',
    },
  },
};

class I18n {
  private currentLocale: Locale = 'en';

  setLocale(locale: Locale): boolean {
    if (locale in translations) {
      this.currentLocale = locale;
      if (typeof window !== 'undefined') {
        localStorage.setItem('locale', locale);
      }
      return true;
    }
    return false;
  }

  getLocale(): Locale {
    return this.currentLocale;
  }

  loadLocaleFromStorage(): void {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('locale') as Locale | null;
      if (stored && stored in translations) {
        this.currentLocale = stored;
      }
    }
  }

  t(key: string, defaultValue?: string): string {
    const keys = key.split('.');
    let value: Translations | string = translations[this.currentLocale];

    for (const k of keys) {
      if (value && typeof value === 'object' && k in value) {
        value = value[k];
      } else {
        return defaultValue || key;
      }
    }

    return typeof value === 'string' ? value : (defaultValue || key);
  }

  getAvailableLocales(): Locale[] {
    return Object.keys(translations) as Locale[];
  }
}

export const i18n = new I18n();
