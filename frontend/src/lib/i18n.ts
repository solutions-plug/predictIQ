/**
 * Simple i18n utility for frontend internationalization.
 * Supports multiple locales with fallback to English.
 */

export type Locale = 'en';

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
      markets: 'Markets',
      statistics: 'Statistics',
      createMarket: 'Create Market',
      admin: 'Admin',
    },
    hero: {
      title: 'Decentralized Prediction Markets on Stellar',
      description: 'Create, bet on, and resolve prediction markets with transparency, security, and fairness powered by blockchain technology.',
      signupHeading: 'Sign up for updates',
      emailLabel: 'Email Address',
      emailPlaceholder: 'you@example.com',
      emailRequired: 'Email is required',
      emailInvalid: 'Please enter a valid email address',
      primaryCta: 'Explore markets',
      secondaryCta: 'See how it works',
      submitButton: 'Get Early Access',
      subscribedButton: 'Subscribed!',
      successMessage: 'Successfully subscribed to updates!',
    },
    features: {
      heading: 'Key Features',
      decentralized: {
        title: 'Multi-outcome markets',
        description: 'Create markets with two or many outcomes. Rules live in an on-chain Soroban contract, not a company database.',
      },
      secure: {
        title: 'Hybrid oracle + community resolution',
        description: 'Markets resolve from Pyth and Reflector oracle data, with a community-vote fallback and a dispute window when the feeds disagree.',
      },
      fast: {
        title: 'Stellar speed, with referrals',
        description: 'Near-instant settlement and low fees on Stellar. Bring others in and earn a share through the built-in referral program.',
      },
    },
    howItWorks: {
      heading: 'How It Works',
      step1: {
        title: 'Connect your wallet',
        description: 'Link a Stellar wallet to sign transactions. No account, no email.',
      },
      step2: {
        title: 'Browse markets',
        description: 'Explore open prediction markets and their live odds and volume.',
      },
      step3: {
        title: 'Place a bet',
        description: 'Pick an outcome and stake on it. Your position settles on-chain.',
      },
      step4: {
        title: 'Claim your payout',
        description: 'Once a market resolves, winners claim their share of the pool.',
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
      productHeading: 'Product',
      markets: 'Markets',
      statistics: 'Statistics',
      createMarket: 'Create a market',
      resourcesHeading: 'Resources',
      documentation: 'Documentation',
      github: 'GitHub',
      newsletterHeading: 'Stay in the loop',
      copyright: '© 2026 PredictIQ. All rights reserved.',
    },
    appShell: {
      skipToMain: 'Skip to main content',
      home: 'PredictIQ Home',
      primaryNav: 'Primary navigation',
      exitToSite: 'Exit to Site →',
    },
    placeBetForm: {
      title: 'Place a bet',
      heading: 'Outcome',
      amountLabel: 'Amount',
      validationOutcome: 'Choose an outcome to bet on.',
      validationAmount: 'Enter an amount greater than 0.',
      connectWalletHint: 'Connect your wallet to place a bet on this market.',
      connected: 'Connected: {address}',
      walletNotInstalled: 'No Stellar wallet extension detected.',
      installFreighter: 'Install Freighter',
      connectWallet: 'Connect Wallet',
      connecting: 'Connecting…',
      placingBet: 'Placing Bet…',
      placeBet: 'Place Bet',
      betSubmitted: 'Bet submitted ({status})',
      betStillPending: 'Bet submitted (still pending). This may take longer on busy networks.',
      transactionLabel: 'Transaction: {hash}',
      checkStatusAgain: 'Check Status Again',
    },
    marketDetailView: {
      ariaLabel: 'Market details',
      loadingAriaLabel: 'Loading market',
      loadError: 'Failed to load this market. Please try again.',
      retry: 'Retry',
      loading: 'Loading…',
      viewPayoutDetails: 'View payout details',
    },
    tokenSelector: {
      searchPlaceholder: 'Search settlement asset…',
      noMatching: 'No matching assets',
      sorobanToken: 'Soroban token',
      classicAsset: 'Classic asset',
    },
    exportButton: {
      groupAriaLabel: 'Export statistics data',
      exportCsv: 'Export CSV',
      exportJson: 'Export JSON',
    },
    admin: {
      skipToContent: 'Skip to admin content',
      home: 'PredictIQ Home',
      badge: 'Admin',
      subNav: 'Admin sub-navigation',
      emailPreview: 'Email Preview',
      emailAnalytics: 'Email Analytics',
      blockchainReplay: 'Blockchain Replay',
      contentManagement: 'Content Management',
      auditLog: 'Audit Log',
      apiKeys: 'API Keys',
      apiKey: 'Admin API key',
      continue: 'Continue',
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
