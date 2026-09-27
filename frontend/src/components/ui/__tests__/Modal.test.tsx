import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { Modal } from '../Modal';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface Options {
  open?: boolean;
  onClose?: jest.Mock;
  disableBackdropDismiss?: boolean;
  disableEscapeKey?: boolean;
  description?: string;
  footer?: React.ReactNode;
}

function renderModal({
  open = true,
  onClose = jest.fn(),
  disableBackdropDismiss = false,
  disableEscapeKey = false,
  description,
  footer,
}: Options = {}) {
  return render(
    <Modal
      open={open}
      onClose={onClose}
      title="Confirm"
      description={description}
      disableBackdropDismiss={disableBackdropDismiss}
      disableEscapeKey={disableEscapeKey}
      footer={footer}
    >
      <button type="button">OK</button>
      <button type="button">Cancel</button>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

describe('Modal rendering', () => {
  it('renders nothing when open=false', () => {
    renderModal({ open: false });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders the dialog when open=true', () => {
    renderModal();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('renders the title', () => {
    renderModal();
    expect(screen.getByText('Confirm')).toBeInTheDocument();
  });

  it('renders the description when provided', () => {
    renderModal({ description: 'This cannot be undone.' });
    expect(screen.getByText('This cannot be undone.')).toBeInTheDocument();
  });

  it('does not render a description paragraph when omitted', () => {
    renderModal();
    expect(screen.queryByText('This cannot be undone.')).not.toBeInTheDocument();
  });

  it('renders children inside the modal body', () => {
    renderModal();
    expect(screen.getByRole('button', { name: 'OK' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('renders an optional footer slot', () => {
    renderModal({ footer: <span data-testid="footer-content">Footer</span> });
    expect(screen.getByTestId('footer-content')).toBeInTheDocument();
  });

  it('does not render a footer element when footer is omitted', () => {
    const { container } = renderModal();
    expect(container.querySelector('.ui-modal__footer')).not.toBeInTheDocument();
  });

  it('renders a close button by default', () => {
    renderModal();
    expect(screen.getByRole('button', { name: /close dialog/i })).toBeInTheDocument();
  });

  it('hides the close button when disableBackdropDismiss=true', () => {
    renderModal({ disableBackdropDismiss: true });
    expect(screen.queryByRole('button', { name: /close dialog/i })).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // ARIA attributes
  // -------------------------------------------------------------------------

  it('sets role="dialog" and aria-modal="true"', () => {
    renderModal();
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
  });

  it('sets aria-labelledby pointing to the title element', () => {
    renderModal();
    const dialog = screen.getByRole('dialog');
    const labelledById = dialog.getAttribute('aria-labelledby');
    expect(labelledById).toBeTruthy();
    expect(document.getElementById(labelledById!)).toHaveTextContent('Confirm');
  });

  it('sets aria-describedby when a description is provided', () => {
    renderModal({ description: 'Details here.' });
    const dialog = screen.getByRole('dialog');
    const describedById = dialog.getAttribute('aria-describedby');
    expect(describedById).toBeTruthy();
    expect(document.getElementById(describedById!)).toHaveTextContent('Details here.');
  });

  it('does not set aria-describedby when description is omitted', () => {
    renderModal();
    expect(screen.getByRole('dialog')).not.toHaveAttribute('aria-describedby');
  });
});

// ---------------------------------------------------------------------------
// Open / close behaviour
// ---------------------------------------------------------------------------

describe('Modal open/close behaviour', () => {
  it('calls onClose when the close button is clicked', async () => {
    const onClose = jest.fn();
    renderModal({ onClose });
    await userEvent.click(screen.getByRole('button', { name: /close dialog/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when the backdrop is clicked', () => {
    const onClose = jest.fn();
    renderModal({ onClose });
    const backdrop = document.querySelector('.ui-modal-backdrop') as HTMLElement;
    fireEvent.click(backdrop, { target: backdrop });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not call onClose when clicking inside the dialog', () => {
    const onClose = jest.fn();
    renderModal({ onClose });
    fireEvent.click(screen.getByRole('dialog'));
    expect(onClose).not.toHaveBeenCalled();
  });

  it('does not call onClose on backdrop click when disableBackdropDismiss=true', () => {
    const onClose = jest.fn();
    renderModal({ onClose, disableBackdropDismiss: true });
    const backdrop = document.querySelector('.ui-modal-backdrop') as HTMLElement;
    fireEvent.click(backdrop, { target: backdrop });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('calls onClose when Escape is pressed', () => {
    const onClose = jest.fn();
    renderModal({ onClose });
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not call onClose on Escape when disableEscapeKey=true', () => {
    const onClose = jest.fn();
    renderModal({ onClose, disableEscapeKey: true });
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('unmounts the dialog when open transitions to false', () => {
    const { rerender } = renderModal({ open: true });
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    rerender(
      <Modal open={false} onClose={jest.fn()} title="Confirm">
        <button>OK</button>
      </Modal>
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Body scroll lock
// ---------------------------------------------------------------------------

describe('Modal body scroll lock', () => {
  afterEach(() => {
    document.body.style.overflow = '';
  });

  it('sets body overflow to hidden when open', () => {
    renderModal({ open: true });
    expect(document.body.style.overflow).toBe('hidden');
  });

  it('restores body overflow when closed via rerender', () => {
    const { rerender } = renderModal({ open: true });
    rerender(
      <Modal open={false} onClose={jest.fn()} title="Confirm">
        <button>OK</button>
      </Modal>
    );
    expect(document.body.style.overflow).toBe('');
  });

  it('restores body overflow when unmounted', () => {
    const { unmount } = renderModal({ open: true });
    unmount();
    expect(document.body.style.overflow).toBe('');
  });
});

// ---------------------------------------------------------------------------
// Focus management
// ---------------------------------------------------------------------------

describe('Modal focus management', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  it('focuses the first focusable element after opening', () => {
    renderModal({ open: true });
    act(() => { jest.runAllTimers(); });
    const focused = document.activeElement as HTMLElement;
    expect(focused.tagName).toBe('BUTTON');
    expect(screen.getByRole('dialog').contains(focused)).toBe(true);
  });

  it('restores focus to the previously focused element on close', () => {
    const trigger = document.createElement('button');
    trigger.textContent = 'Open';
    document.body.appendChild(trigger);
    trigger.focus();

    const { rerender } = renderModal({ open: true });
    act(() => { jest.runAllTimers(); });

    rerender(
      <Modal open={false} onClose={jest.fn()} title="Confirm">
        <button>OK</button>
      </Modal>
    );

    expect(document.activeElement).toBe(trigger);
    document.body.removeChild(trigger);
  });

  it('traps Tab focus: wraps forward from last to first focusable', () => {
    renderModal({ open: true });
    act(() => { jest.runAllTimers(); });

    const dialog = screen.getByRole('dialog');
    const focusable = dialog.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const last = focusable[focusable.length - 1];
    last.focus();
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: false });
    expect(document.activeElement).toBe(focusable[0]);
  });

  it('traps Shift+Tab focus: wraps backward from first to last focusable', () => {
    renderModal({ open: true });
    act(() => { jest.runAllTimers(); });

    const dialog = screen.getByRole('dialog');
    const focusable = dialog.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    first.focus();
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(last);
  });
});

// ---------------------------------------------------------------------------
// Event listener cleanup
// ---------------------------------------------------------------------------

describe('Modal event listener cleanup', () => {
  it('removes keydown listener when the modal closes', () => {
    const spy = jest.spyOn(document, 'removeEventListener');
    const { rerender } = renderModal({ open: true });
    rerender(
      <Modal open={false} onClose={jest.fn()} title="Confirm">
        <button>OK</button>
      </Modal>
    );
    expect(spy).toHaveBeenCalledWith('keydown', expect.any(Function));
    spy.mockRestore();
  });

  it('removes keydown listener when the component unmounts', () => {
    const spy = jest.spyOn(document, 'removeEventListener');
    const { unmount } = renderModal({ open: true });
    unmount();
    expect(spy).toHaveBeenCalledWith('keydown', expect.any(Function));
    spy.mockRestore();
  });
});
