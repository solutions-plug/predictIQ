'use client';

import React from 'react';
import { useI18n } from '../lib/hooks/useI18n';
import { useDarkMode } from '../lib/hooks/useDarkMode';
import { type Locale } from '../lib/i18n';
import { Statistics } from './Statistics';
import { ErrorBoundary } from './ErrorBoundary';
import { NewsletterSignup } from './NewsletterSignup';
import { FeatureCard } from './landing/FeatureCard';
import { Step } from './landing/Step';
import { FooterColumn } from './landing/FooterColumn';

interface LandingPageProps {
  className?: string;
}

export const LandingPage: React.FC<LandingPageProps> = ({ className }) => {
  const { t, locale, setLocale, availableLocales } = useI18n();
  const { isDarkMode, toggleDarkMode } = useDarkMode();

  const features = [
    { icon: '/icons/decentralized.svg', title: t('features.decentralized.title'), description: t('features.decentralized.description'), href: '/markets' },
    { icon: '/icons/secure.svg', title: t('features.secure.title'), description: t('features.secure.description'), href: '/markets' },
    { icon: '/icons/fast.svg', title: t('features.fast.title'), description: t('features.fast.description'), href: '/markets' },
  ];

  const steps = [
    { title: t('howItWorks.step1.title'), description: t('howItWorks.step1.description'), href: '/markets' },
    { title: t('howItWorks.step2.title'), description: t('howItWorks.step2.description'), href: '/markets' },
    { title: t('howItWorks.step3.title'), description: t('howItWorks.step3.description'), href: '/markets' },
    { title: t('howItWorks.step4.title'), description: t('howItWorks.step4.description'), href: '/account/bets' },
  ];

  // Product links point at routes that actually exist; Resources links are
  // external. The Legal column was dropped - there are no privacy/terms pages
  // yet, and #1346 requires no dead links.
  const footerColumns = [
    { heading: t('footer.title'), headingLevel: 'h2' as const, tagline: t('footer.tagline') },
    { heading: t('footer.productHeading'), links: [
      { href: '/markets', label: t('footer.markets') },
      { href: '/statistics', label: t('footer.statistics') },
      { href: '/markets/create', label: t('footer.createMarket') },
    ] },
    { heading: t('footer.resourcesHeading'), links: [
      { href: 'https://github.com/solutions-plug/predictIQ#readme', label: t('footer.documentation'), external: true },
      { href: 'https://github.com/solutions-plug/predictIQ', label: t('footer.github'), external: true },
    ] },
  ];

  return (
    <div className={className}>
      {/* Skip to main content link */}
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>

      {/* Header */}
      <header role="banner">
        <nav aria-label="Main navigation">
          <div className="nav-container">
            <div className="logo">
              <img
                src="/mark.svg"
                alt="PredictIQ Logo"
                width="40"
                height="40"
              />
              <span className="logo-text" aria-hidden="true">PredictIQ</span>
            </div>
            <ul className="nav-menu">
              <li><a href="#features">Features</a></li>
              <li><a href="#how-it-works">How It Works</a></li>
              <li><a href="#about">About</a></li>
              <li><a href="#contact">Contact</a></li>
            </ul>
            
            {/* Controls */}
            <div className="header-controls">
              {/* Dark Mode Toggle */}
              <button
                onClick={toggleDarkMode}
                aria-label={isDarkMode ? 'Switch to light mode' : 'Switch to dark mode'}
                className="dark-mode-toggle"
                title={isDarkMode ? 'Light mode' : 'Dark mode'}
              >
                {isDarkMode ? '☀️' : '🌙'}
              </button>

              {/* Language Selector */}
              <div className="language-selector">
                <label htmlFor="locale-select" className="visually-hidden">
                  Select language
                </label>
                <select
                  id="locale-select"
                  value={locale}
                  onChange={(e) => setLocale(e.target.value as Locale)}
                  aria-label="Language selection"
                >
                  {availableLocales.map((loc) => (
                    <option key={loc} value={loc}>
                      {loc.toUpperCase()}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </div>
        </nav>
      </header>

      {/* Main Content */}
      <main id="main-content" role="main">
        {/* Hero Section */}
        <section aria-labelledby="hero-heading" className="hero">
          <div className="hero-glow" aria-hidden="true" />
          <span className="eyebrow">Live on Stellar</span>
          <h1 id="hero-heading">
            {t('hero.title')}
          </h1>
          <p className="hero-description">
            {t('hero.description')}
          </p>

          {/* Primary CTAs into the live product. Plain links (not the newsletter
              form) so the hero works with JS pending and needs no client state. */}
          <div className="hero-cta-group">
            <a href="/markets" className="hero-cta hero-cta--primary">
              {t('hero.primaryCta')}
            </a>
            <a href="#how-it-works" className="hero-cta hero-cta--secondary">
              {t('hero.secondaryCta')}
            </a>
          </div>

          {/* Early-access signup */}
          <NewsletterSignup />
        </section>

        {/* Statistics Section */}
        <ErrorBoundary section="statistics" fallback={(reset) => (
          <section className="statistics" aria-labelledby="statistics-heading">
            <h2 id="statistics-heading">Platform Statistics</h2>
            <div className="error-message" role="alert">
              <p>Unable to load statistics at this time. Please try again later.</p>
              <button
                className="retry-button"
                onClick={reset}
                aria-label="Retry loading statistics"
              >
                Retry
              </button>
            </div>
          </section>
        )}>
          <Statistics />
        </ErrorBoundary>

        {/* Features Section */}
        <section aria-labelledby="features-heading" id="features">
          <h2 id="features-heading">{t('features.heading')}</h2>
          
          <div className="features-grid">
            {features.map((feature) => (
              <FeatureCard key={feature.title} {...feature} />
            ))}
          </div>
        </section>

        {/* How It Works Section */}
        <section aria-labelledby="how-it-works-heading" id="how-it-works">
          <h2 id="how-it-works-heading">{t('howItWorks.heading')}</h2>
          
          <ol className="steps-list">
            {steps.map((step) => (
              <Step key={step.title} {...step} />
            ))}
          </ol>
        </section>

        {/* About Section */}
        <section aria-labelledby="about-heading" id="about">
          <h2 id="about-heading">{t('about.heading')}</h2>
          <p>
            {t('about.description1')}
          </p>
          <p>
            {t('about.description2')}
          </p>
        </section>
      </main>

      {/* Footer */}
      <footer role="contentinfo" id="contact">
        <div className="footer-content">
          {footerColumns.map((column) => (
            <FooterColumn key={column.heading} {...column} />
          ))}
          <FooterColumn heading={t('footer.newsletterHeading')}>
            {/* The signup form itself lives in the hero (one set of field ids on
                the page); the footer points people back to it. */}
            <p>
              <a href="#main-content">{t('hero.signupHeading')}</a>
            </p>
          </FooterColumn>
        </div>

        <div className="footer-bottom">
          <p>{t('footer.copyright')}</p>
        </div>
      </footer>
    </div>
  );
};

export default LandingPage;
