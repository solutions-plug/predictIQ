import React from 'react';
import { useI18n } from '../lib/hooks/useI18n';
import { useDarkMode } from '../lib/hooks/useDarkMode';
import { type Locale } from '../lib/i18n';
import { api } from '../lib/api/client';
import { Statistics } from './Statistics';
import { ErrorBoundary } from './ErrorBoundary';
import { LoadingSpinner } from './LoadingSpinner';

interface LandingPageProps {
  className?: string;
}

export const LandingPage: React.FC<LandingPageProps> = ({ className }) => {
  const { t, locale, setLocale, availableLocales } = useI18n();
  const { isDarkMode, toggleDarkMode } = useDarkMode();
  const [email, setEmail] = React.useState('');
  const [emailError, setEmailError] = React.useState('');
  const [isSubmitted, setIsSubmitted] = React.useState(false);
  const [apiError, setApiError] = React.useState('');
  const [isLoading, setIsLoading] = React.useState(false);
  const formStatusRef = React.useRef<HTMLDivElement>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!email) {
      setEmailError(t('hero.emailRequired'));
      return;
    }
    if (!emailRegex.test(email)) {
      setEmailError(t('hero.emailInvalid'));
      return;
    }

    setEmailError('');
    setApiError('');
    setIsLoading(true);

    try {
      const result = await api.newsletterSubscribe({ email });
      if (result.success) {
        setIsSubmitted(true);
        if (formStatusRef.current) {
          formStatusRef.current.textContent = t('hero.successMessage');
        }
      } else {
        setApiError(result.message || 'Subscription failed');
      }
    } catch (err) {
      setApiError(err instanceof Error ? err.message : 'Network error occurred');
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLFormElement>) => {
    if (e.key === 'Enter') {
      e.currentTarget.requestSubmit();
    }
  };

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
                src="/logo.svg" 
                alt="PredictIQ Logo" 
                width="150" 
                height="50"
              />
            </div>
            <ul className="nav-menu" role="menubar">
              <li role="none"><a href="#features" role="menuitem">Features</a></li>
              <li role="none"><a href="#how-it-works" role="menuitem">How It Works</a></li>
              <li role="none"><a href="#about" role="menuitem">About</a></li>
              <li role="none"><a href="#contact" role="menuitem">Contact</a></li>
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
          <h1 id="hero-heading">
            {t('hero.title')}
          </h1>
          <p className="hero-description">
            {t('hero.description')}
          </p>
          
          {/* CTA Form */}
          <form 
            onSubmit={handleSubmit}
            onKeyDown={handleKeyDown}
            aria-labelledby="signup-heading"
            aria-busy={isLoading}
            noValidate
          >
            <h2 id="signup-heading" className="visually-hidden">
              {t('hero.signupHeading')}
            </h2>
            
            <div className="form-group">
              <label htmlFor="email-input">
                {t('hero.emailLabel')}
                <span aria-label="required" className="required">*</span>
              </label>
              <input
                id="email-input"
                type="email"
                value={email}
                onChange={(e) => {
                  setEmail(e.target.value);
                  setEmailError('');
                  setApiError('');
                }}
                aria-required="true"
                aria-invalid={!!emailError}
                aria-describedby={emailError ? 'email-error' : undefined}
                placeholder={t('hero.emailPlaceholder')}
                disabled={isSubmitted || isLoading}
              />
              {emailError && (
                <span id="email-error" role="alert" className="error-message">
                  {emailError}
                </span>
              )}
              {apiError && (
                <span id="api-error" role="alert" className="error-message">
                  {apiError}
                </span>
              )}
            </div>

            <button 
              type="submit" 
              disabled={isSubmitted || isLoading}
              aria-label={
                isLoading 
                  ? 'Submitting...' 
                  : isSubmitted 
                    ? t('hero.subscribedButton') 
                    : t('hero.submitButton')
              }
            >
              {isLoading ? (
                <LoadingSpinner size="small" aria-label="Submitting" />
              ) : isSubmitted ? (
                t('hero.subscribedButton')
              ) : (
                t('hero.submitButton')
              )}
            </button>

            {/* Screen reader announcement */}
            <div 
              ref={formStatusRef}
              id="form-status" 
              role="status" 
              aria-live="polite" 
              aria-atomic="true"
              className="visually-hidden"
            />
          </form>
        </section>

        {/* Statistics Section */}
        <ErrorBoundary section="statistics" fallback={
          <section className="statistics" aria-labelledby="statistics-heading">
            <h2 id="statistics-heading">Platform Statistics</h2>
            <div className="error-message" role="alert">
              <p>Unable to load statistics at this time. Please try again later.</p>
              <button
                className="retry-button"
                onClick={() => window.location.reload()}
                aria-label="Retry loading statistics"
              >
                Retry
              </button>
            </div>
          </section>
        }>
          <Statistics />
        </ErrorBoundary>

        {/* Features Section */}
        <section aria-labelledby="features-heading" id="features">
          <h2 id="features-heading">{t('features.heading')}</h2>
          
          <div className="features-grid">
            <article className="feature-card">
              <img 
                src="/icons/decentralized.svg" 
                alt="" 
                aria-hidden="true"
                width="64"
                height="64"
              />
              <h3>{t('features.decentralized.title')}</h3>
              <p>
                {t('features.decentralized.description')}
              </p>
            </article>

            <article className="feature-card">
              <img 
                src="/icons/secure.svg" 
                alt="" 
                aria-hidden="true"
                width="64"
                height="64"
              />
              <h3>{t('features.secure.title')}</h3>
              <p>
                {t('features.secure.description')}
              </p>
            </article>

            <article className="feature-card">
              <img 
                src="/icons/fast.svg" 
                alt="" 
                aria-hidden="true"
                width="64"
                height="64"
              />
              <h3>{t('features.fast.title')}</h3>
              <p>
                {t('features.fast.description')}
              </p>
            </article>
          </div>
        </section>

        {/* How It Works Section */}
        <section aria-labelledby="how-it-works-heading" id="how-it-works">
          <h2 id="how-it-works-heading">{t('howItWorks.heading')}</h2>
          
          <ol className="steps-list">
            <li>
              <h3>{t('howItWorks.step1.title')}</h3>
              <p>{t('howItWorks.step1.description')}</p>
            </li>
            <li>
              <h3>{t('howItWorks.step2.title')}</h3>
              <p>{t('howItWorks.step2.description')}</p>
            </li>
            <li>
              <h3>{t('howItWorks.step3.title')}</h3>
              <p>{t('howItWorks.step3.description')}</p>
            </li>
            <li>
              <h3>{t('howItWorks.step4.title')}</h3>
              <p>{t('howItWorks.step4.description')}</p>
            </li>
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
          <div className="footer-section">
            <h2>{t('footer.title')}</h2>
            <p>{t('footer.tagline')}</p>
          </div>
          
          <div className="footer-section">
            <h3>{t('footer.linksHeading')}</h3>
            <ul>
              <li><a href="/docs">{t('footer.documentation')}</a></li>
              <li><a href="/github">{t('footer.github')}</a></li>
              <li><a href="/discord">{t('footer.discord')}</a></li>
            </ul>
          </div>
          
          <div className="footer-section">
            <h3>{t('footer.legalHeading')}</h3>
            <ul>
              <li><a href="/privacy">{t('footer.privacy')}</a></li>
              <li><a href="/terms">{t('footer.terms')}</a></li>
            </ul>
          </div>
        </div>
        
        <div className="footer-bottom">
          <p>{t('footer.copyright')}</p>
        </div>
      </footer>
    </div>
  );
};

export default LandingPage;
