/**
 * Dedicated tests for NewsletterSignup's cooldown / rate-limit state machine
 * (#1584).
 *
 * Scope: the component's own CLIENT_SUBMIT_COOLDOWN_SECS, isServerRateLimited,
 * and lastSubmittedEmailRef branches — isolated from the LandingPage suites
 * that exercise the surrounding page.
 *
 * Timer strategy:
 *   Tests that only need to observe the result of an API call (no countdown)
 *   run with real timers.  Tests that advance the countdown call
 *   jest.useFakeTimers() inside the test body AFTER the async interaction
 *   has resolved, so userEvent never runs against fake timers.
 */

import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { NewsletterSignup } from '../NewsletterSignup';
import { api, ApiError } from '../../lib/api/public-client';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const VALID_EMAIL = 'user@example.com';
const OTHER_EMAIL = 'other@example.com';

function renderForm(props: Partial<React.ComponentProps<typeof NewsletterSignup>> = {}) {
  return render(<NewsletterSignup {...props} />);
}

/** Type into the email field by label. */
async function typeEmail(email: string) {
  const input = screen.getByLabelText(/email address/i);
  await userEvent.clear(input);
  await userEvent.type(input, email);
}

/** Click the submit button (real timers — must be called before switching to fake). */
async function clickSubmit() {
  await userEvent.click(screen.getByRole('button', { name: /get early access/i }));
}

// ---------------------------------------------------------------------------
// Rendering basics
// ---------------------------------------------------------------------------

describe('NewsletterSignup — rendering', () => {
  afterEach(() => jest.restoreAllMocks());

  it('renders an email input and a submit button', () => {
    renderForm();
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument();
    expect(screen.getByRole('button')).toBeInTheDocument();
  });

  it('the submit button is enabled before any submission', () => {
    renderForm();
    expect(screen.getByRole('button')).not.toBeDisabled();
  });

  it('has a live-region status div for screen-reader announcements', () => {
    renderForm();
    const status = document.getElementById('form-status');
    expect(status).toBeInTheDocument();
    expect(status).toHaveAttribute('role', 'status');
    expect(status).toHaveAttribute('aria-live', 'polite');
  });
});

// ---------------------------------------------------------------------------
// Successful submission
// ---------------------------------------------------------------------------

describe('NewsletterSignup — successful submission', () => {
  afterEach(() => jest.restoreAllMocks());

  it('shows the subscribed state and calls onSuccess after a successful API response', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockResolvedValue({ success: true, message: 'OK' });
    const onSuccess = jest.fn();
    renderForm({ onSuccess });

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(/successfully subscribed/i);
    });
    expect(onSuccess).toHaveBeenCalledTimes(1);
  });

  it('passes source prop through to the API call', async () => {
    const spy = jest.spyOn(api, 'newsletterSubscribe').mockResolvedValue({ success: true, message: 'OK' });
    renderForm({ source: 'hero' });

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith({ email: VALID_EMAIL, source: 'hero' });
    });
  });
});

// ---------------------------------------------------------------------------
// Client-side validation
// ---------------------------------------------------------------------------

describe('NewsletterSignup — client-side validation', () => {
  afterEach(() => jest.restoreAllMocks());

  it('shows an email-required error when submitted empty', async () => {
    renderForm();
    await userEvent.click(screen.getByRole('button'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/email is required/i);
    });
  });

  it('shows an invalid email error for a malformed address', async () => {
    renderForm();
    await typeEmail('notanemail');
    await userEvent.click(screen.getByRole('button'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/valid email/i);
    });
  });

  it('clears the email error when the user starts typing a correction', async () => {
    renderForm();
    // Trigger error
    await userEvent.click(screen.getByRole('button'));
    await waitFor(() => expect(screen.getByRole('alert')).toBeInTheDocument());
    // Fix it
    await typeEmail(VALID_EMAIL);
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
  });
});

// ---------------------------------------------------------------------------
// AC 1: 429 response → server-side cooldown with retry_after
// ---------------------------------------------------------------------------

describe('NewsletterSignup — 429 / server rate limit', () => {
  afterEach(() => {
    jest.useRealTimers();
    jest.restoreAllMocks();
  });

  it('shows an error and disables the button after a 429 response', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Rate limited', 429, 'RATE_LIMITED', { retry_after: 60 })
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/too many requests/i);
    });
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('uses the server-provided retry_after value for the cooldown', async () => {
    const RETRY_AFTER = 45;
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Rate limited', 429, 'RATE_LIMITED', { retry_after: RETRY_AFTER })
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(`${RETRY_AFTER}s`);
    });
  });

  it('falls back to 30 s cooldown when retry_after is absent from 429 details', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Rate limited', 429, 'RATE_LIMITED')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      // DEFAULT_RATE_LIMIT_COOLDOWN_SECS = 30
      expect(screen.getByRole('alert')).toHaveTextContent('30s');
    });
  });

  it('button remains disabled while the 429 cooldown is active, even after typing into email field', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Rate limited', 429, 'RATE_LIMITED', { retry_after: 30 })
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
    });

    // Typing a character while under server rate limit should NOT reset the cooldown
    await userEvent.type(screen.getByLabelText(/email address/i), 'x');
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('the countdown ticks down every second and re-enables the button at zero', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Rate limited', 429, 'RATE_LIMITED', { retry_after: 3 })
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    // The button text updates with each tick; the alert text is set once on error.
    await waitFor(() => {
      expect(screen.getByRole('button')).toHaveTextContent(/wait \(3s\)/i);
    });

    // Each real second the interval fires; check the button label decrements
    await waitFor(
      () => expect(screen.getByRole('button')).toHaveTextContent(/wait \(2s\)/i),
      { timeout: 3000 }
    );

    // Wait for the full cooldown to expire and button to re-enable
    await waitFor(
      () => expect(screen.getByRole('button')).not.toBeDisabled(),
      { timeout: 5000 }
    );
  }, 10_000);
});

// ---------------------------------------------------------------------------
// AC 2: Non-429 failure → short CLIENT_SUBMIT_COOLDOWN_SECS (3 s)
// ---------------------------------------------------------------------------

describe('NewsletterSignup — non-429 failure cooldown', () => {
  afterEach(() => {
    jest.useRealTimers();
    jest.restoreAllMocks();
  });

  it('shows the error and starts a 3-second cooldown after a generic network error', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new Error('Network error occurred')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/network error/i);
    });
    // Button is disabled for the same email (cooldown active)
    expect(screen.getByRole('button')).toBeDisabled();
    expect(screen.getByRole('button')).toHaveTextContent(/wait \(3s\)/i);
  });

  it('starts a 3-second cooldown when the API returns { success: false }', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockResolvedValue({
      success: false,
      message: 'Already subscribed',
    });
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/already subscribed/i);
    });
    expect(screen.getByRole('button')).toHaveTextContent(/wait \(3s\)/i);
  });

  it('client cooldown ticks down and re-enables the button after 3 seconds', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Server error', 500, 'SERVER_ERROR')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('button')).toHaveTextContent(/wait \(3s\)/i);
    });

    // CLIENT_SUBMIT_COOLDOWN_SECS = 3; wait for real setInterval to expire
    await waitFor(
      () => {
        expect(screen.getByRole('button')).not.toBeDisabled();
        expect(screen.getByRole('button')).toHaveTextContent(/get early access/i);
      },
      { timeout: 6000 }
    );
  }, 10_000);

  it('does not set isServerRateLimited for a 500 error (editing email re-enables button)', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Server error', 500, 'SERVER_ERROR')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
    });

    // isServerRateLimited should be false → changing email resets cooldown
    const input = screen.getByLabelText(/email address/i);
    await userEvent.clear(input);
    await userEvent.type(input, OTHER_EMAIL);

    await waitFor(() => {
      expect(screen.getByRole('button')).not.toBeDisabled();
    });
  });
});

// ---------------------------------------------------------------------------
// AC 3: Same-email resubmit blocked / unblocked by editing
// ---------------------------------------------------------------------------

describe('NewsletterSignup — same-email resubmit guard', () => {
  afterEach(() => {
    jest.useRealTimers();
    jest.restoreAllMocks();
  });

  it('blocks resubmission with the same email while cooldown is active', async () => {
    const subscribeSpy = jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Server error', 500, 'SERVER_ERROR')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    // First submit triggered the error + cooldown
    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
    });

    subscribeSpy.mockClear();

    // Dispatch form submit directly (bypasses disabled-button visual state)
    act(() => {
      const form = document.querySelector('form')!;
      form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });

    await waitFor(() => {
      // handleSubmit detects cooldown > 0 && email === lastSubmitted → resubmit error
      expect(screen.getByRole('alert')).toHaveTextContent(/please wait/i);
    });

    // API should NOT have been called again
    expect(subscribeSpy).not.toHaveBeenCalled();
  });

  it('re-enables the button immediately when the email is changed to a new value', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Server error', 500, 'SERVER_ERROR')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
    });

    // Change to a different email → cooldown reset (isServerRateLimited is false)
    const input = screen.getByLabelText(/email address/i);
    await userEvent.clear(input);
    await userEvent.type(input, OTHER_EMAIL);

    await waitFor(() => {
      expect(screen.getByRole('button')).not.toBeDisabled();
    });
  });

  it('the submit button stays disabled while the cooldown is still active (no edit)', async () => {
    // This directly exercises the isButtonDisabled guard:
    // cooldown > 0 && email === lastSubmittedEmail → disabled.
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Server error', 500, 'SERVER_ERROR')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    // Cooldown is active and email hasn't changed
    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
      expect(screen.getByRole('button')).toHaveTextContent(/wait \(3s\)/i);
    });

    // No edits — button remains disabled for the duration
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('allows a fresh successful submission after changing the email under a client cooldown', async () => {
    jest
      .spyOn(api, 'newsletterSubscribe')
      .mockRejectedValueOnce(new ApiError('Server error', 500, 'SERVER_ERROR'))
      .mockResolvedValueOnce({ success: true, message: 'Subscribed' });

    renderForm();

    // First submit: fails, starts cooldown
    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
    });

    // Change email → cooldown cleared
    const input = screen.getByLabelText(/email address/i);
    await userEvent.clear(input);
    await userEvent.type(input, OTHER_EMAIL);

    // Second submit: succeeds
    await userEvent.click(screen.getByRole('button', { name: /get early access/i }));

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(/successfully subscribed/i);
    });
  });
});

// ---------------------------------------------------------------------------
// Button label transitions
// ---------------------------------------------------------------------------

describe('NewsletterSignup — button label transitions', () => {
  afterEach(() => {
    jest.useRealTimers();
    jest.restoreAllMocks();
  });

  it('shows "Wait (Ns)" on the button while the client cooldown is active for the same email', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockRejectedValue(
      new ApiError('Server error', 500, 'SERVER_ERROR')
    );
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('button')).toHaveTextContent(/wait \(3s\)/i);
    });
  });

  it('shows the subscribed label and disables the button after success', async () => {
    jest.spyOn(api, 'newsletterSubscribe').mockResolvedValue({ success: true, message: 'OK' });
    renderForm();

    await typeEmail(VALID_EMAIL);
    await clickSubmit();

    await waitFor(() => {
      expect(screen.getByRole('button')).toBeDisabled();
      expect(screen.getByRole('button')).toHaveTextContent(/subscribed/i);
    });
  });
});
