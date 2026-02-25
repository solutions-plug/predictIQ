import React from 'react';

interface LandingPageProps {
  className?: string;
}

export const LandingPage: React.FC<LandingPageProps> = ({ className }) => {
  const [email, setEmail] = React.useState('');
  const [emailError, setEmailError] = React.useState('');
  const [isSubmitted, setIsSubmitted] = React.useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    // Basic email validation
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!email) {
      setEmailError('Email is required');
      return;
    }
    if (!emailRegex.test(email)) {
      setEmailError('Please enter a valid email address');
      return;
    }
    
    setEmailError('');
    setIsSubmitted(true);
    
    // Announce success to screen readers
    const announcement = document.getElementById('form-status');
    if (announcement) {
      announcement.textContent = 'Successfully subscribed to updates!';
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
            <ul className="nav-menu">
              <li><a href="#features">Features</a></li>
              <li><a href="#how-it-works">How It Works</a></li>
              <li><a href="#about">About</a></li>
              <li><a href="#contact">Contact</a></li>
            </ul>
          </div>
        </nav>
      </header>

      {/* Main Content */}
      <main id="main-content" role="main">
        {/* Hero Section */}
        <section aria-labelledby="hero-heading" className="hero">
          <h1 id="hero-heading">
            Decentralized Prediction Markets on Stellar
          </h1>
          <p className="hero-description">
            Create, bet on, and resolve prediction markets with transparency, 
            security, and fairness powered by blockchain technology.
          </p>
          
          {/* CTA Form */}
          <form 
            onSubmit={handleSubmit} 
            aria-labelledby="signup-heading"
            noValidate
          >
            <h2 id="signup-heading" className="visually-hidden">
              Sign up for updates
            </h2>
            
            <div className="form-group">
              <label htmlFor="email-input">
                Email Address
                <span aria-label="required" className="required">*</span>
              </label>
              <input
                id="email-input"
                type="email"
                value={email}
                onChange={(e) => {
                  setEmail(e.target.value);
                  setEmailError('');
                }}
                aria-required="true"
                aria-invalid={!!emailError}
                aria-describedby={emailError ? 'email-error' : undefined}
                placeholder="you@example.com"
                disabled={isSubmitted}
              />
              {emailError && (
                <span id="email-error" role="alert" className="error-message">
                  {emailError}
                </span>
              )}
            </div>

            <button 
              type="submit" 
              disabled={isSubmitted}
              aria-label={isSubmitted ? 'Already subscribed' : 'Subscribe to updates'}
            >
              {isSubmitted ? 'Subscribed!' : 'Get Early Access'}
            </button>

            {/* Screen reader announcement */}
            <div 
              id="form-status" 
              role="status" 
              aria-live="polite" 
              aria-atomic="true"
              className="visually-hidden"
            />
          </form>
        </section>

        {/* Features Section */}
        <section aria-labelledby="features-heading" id="features">
          <h2 id="features-heading">Key Features</h2>
          
          <div className="features-grid">
            <article className="feature-card">
              <img 
                src="/icons/decentralized.svg" 
                alt="" 
                aria-hidden="true"
                width="64"
                height="64"
              />
              <h3>Fully Decentralized</h3>
              <p>
                No central authority. Markets run on smart contracts with 
                transparent, immutable rules.
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
              <h3>Secure & Audited</h3>
              <p>
                Smart contracts audited by leading security firms. Your funds 
                are protected by battle-tested code.
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
              <h3>Lightning Fast</h3>
              <p>
                Built on Stellar for near-instant transactions and minimal fees. 
                Trade without waiting.
              </p>
            </article>
          </div>
        </section>

        {/* How It Works Section */}
        <section aria-labelledby="how-it-works-heading" id="how-it-works">
          <h2 id="how-it-works-heading">How It Works</h2>
          
          <ol className="steps-list">
            <li>
              <h3>Create a Market</h3>
              <p>Define outcomes and set parameters for your prediction market.</p>
            </li>
            <li>
              <h3>Place Bets</h3>
              <p>Users bet on outcomes they believe will occur.</p>
            </li>
            <li>
              <h3>Oracle Resolution</h3>
              <p>Trusted oracles provide real-world data to resolve markets.</p>
            </li>
            <li>
              <h3>Claim Winnings</h3>
              <p>Winners automatically receive their share of the pool.</p>
            </li>
          </ol>
        </section>

        {/* About Section */}
        <section aria-labelledby="about-heading" id="about">
          <h2 id="about-heading">About PredictIQ</h2>
          <p>
            PredictIQ is a decentralized prediction market platform built on 
            the Stellar blockchain. We enable anyone to create, participate in, 
            and resolve prediction markets with complete transparency and fairness.
          </p>
          <p>
            Our smart contracts are open-source, audited, and designed with 
            security and user experience as top priorities.
          </p>
        </section>
      </main>

      {/* Footer */}
      <footer role="contentinfo" id="contact">
        <div className="footer-content">
          <div className="footer-section">
            <h2>PredictIQ</h2>
            <p>Decentralized prediction markets for everyone.</p>
          </div>
          
          <div className="footer-section">
            <h3>Links</h3>
            <ul>
              <li><a href="/docs">Documentation</a></li>
              <li><a href="/github">GitHub</a></li>
              <li><a href="/discord">Discord</a></li>
            </ul>
          </div>
          
          <div className="footer-section">
            <h3>Legal</h3>
            <ul>
              <li><a href="/privacy">Privacy Policy</a></li>
              <li><a href="/terms">Terms of Service</a></li>
            </ul>
          </div>
        </div>
        
        <div className="footer-bottom">
          <p>&copy; 2024 PredictIQ. All rights reserved.</p>
        </div>
      </footer>
    </div>
  );
};

export default LandingPage;
