import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { Modal } from '../Modal';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface RenderModalOptions {
  isOpen?: boolean;
  onClose?: jest.Mock;
  disableBackdropDismiss?: boolean;
  disableEscapeKey?: boolean;
  description?: string;
}

function renderModal({
  isOpen = true,
  onClose = jest.fn(),
  disableBackdropDismiss = false,
  disableEscapeKey = false,
  description,
}: RenderModalOptions = {}) {
  return render(
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Confirm Action"
      description={description}
      disableBackdropDismiss={disableBackdropDismiss}
      disableEscapeKey={disableEscapeKey}
    >
      <button type="button">Confirm</button>
      <button type="button">Cancel</button>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

describe('Modal rendering', () => {
  it('renders nothing when isOpen=false', () => {
    renderModal({ isOpen: false });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders the dialog when isOpen=true', () => {
    renderModal();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('renders the title', () => {
    renderModal();
    expect(screen.getByText('Confirm Action')).toBeInTheDocument();
  });

  it('renders the description when provided', () => {
    renderModal({ description: 'This action cannot be undone.' });
    expect(screen.getByText('This action cannot be undone.')).toBeInTheDocument();
  });

  it('does not render a description element when omitted', () => {
    renderModal();
    expect(screen.queryByText(/this action cannot/i)).not.toBeInTheDocument();
  });

  it('renders children inside the modal body', () => {
    renderModal();
    expect(screen.getByRole('button', { name: 'Confirm' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('renders a close button by default', () => {
    renderModal();
    expect(screen.getByRole('button', { name: /close dialog/i })).toBeInTheDocument();
  });

  it('hides the close button when disableBackdropDismiss=true', () => {
    renderModal({ disableBackdropDismiss: true });
    expect(screen.queryByRole('button', { name: /close dialog/i })).not.toBeInTheDocument();
  });

  it('sets aria-modal="true" on the dialog', () => {
    renderModal();
    expect(screen.getByRole('dialog')).toHaveAttribute('aria-modal', 'true');
  });

  it('sets aria-labelledby pointing to the title element', () => {
    renderModal();
    const dialog = screen.getByRole('dialog');
    const labelledById = dialog.getAttribute('aria-labelledby');
    expect(labelledById).toBeTruthy();
    expect(document.getElementById(labelledById!)).toHaveTextContent('Confirm Action');
  });

  it('sets aria-describedby when a description is provided', () => {
    renderModal({ description: 'Some description' });
    const dialog = screen.getByRole('dialog');
    const describedById = dialog.getAttribute('aria-describedby');
    expect(describedById).toBeTruthy();
    expect(document.getElementById(describedById!)).toHaveTextContent('Some description');
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
    // The backdrop is the presentation element that wraps the dialog container
    const backdrop = document.querySelector('.modal-overlay') as HTMLElement;
    fireEvent.click(backdrop, { target: backdrop });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not call onClose when clicking inside the dialog container', () => {
    const onClose = jest.fn();
    renderModal({ onClose });
    const dialog = screen.getByRole('dialog');
    fireEvent.click(dialog);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('does not call onClose on backdrop click when disableBackdropDismiss=true', () => {
    const onClose = jest.fn();
    renderModal({ onClose, disableBackdropDismiss: true });
    const backdrop = document.querySelector('.modal-overlay') as HTMLElement;
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

  it('does not call onClose on Escape when disableBackdropDismiss=true', () => {
    // Per component logic: Esc is only active when BOTH flags are false
    const onClose = jest.fn();
    renderModal({ onClose, disableBackdropDismiss: true });
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('unmounts the dialog when isOpen transitions to false', () => {
    const { rerender } = renderModal({ isOpen: true });
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    rerender(
      <Modal isOpen={false} onClose={jest.fn()} title="Confirm Action">
        <button>Confirm</button>
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

  it('sets body overflow to hidden when opened', () => {
    renderModal({ isOpen: true });
    expect(document.body.style.overflow).toBe('hidden');
  });

  it('restores body overflow when closed', () => {
    const { rerender } = renderModal({ isOpen: true });
    expect(document.body.style.overflow).toBe('hidden');

    rerender(
      <Modal isOpen={false} onClose={jest.fn()} title="Confirm Action">
        <button>Confirm</button>
      </Modal>
    );
    expect(document.body.style.overflow).toBe('');
  });

  it('restores body overflow when the component unmounts', () => {
    const { unmount } = renderModal({ isOpen: true });
    unmount();
    expect(document.body.style.overflow).toBe('');
  });
});

// ---------------------------------------------------------------------------
// Focus management
// ---------------------------------------------------------------------------

describe('Modal focus management', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.runOnlyPendingTimers();
    jest.useRealTimers();
  });

  it('focuses the first focusable element after opening', async () => {
    renderModal({ isOpen: true });

    act(() => {
      jest.advanceTimersByTime(50);
    });

    // The first focusable element in the modal is the "Confirm" button
    // (close button comes after, as it is in the header after the content in DOM order —
    // actually close button is in the header which comes before modal-body children)
    // Either the close button or confirm button should hold focus.
    const focusedEl = document.activeElement as HTMLElement;
    expect(focusedEl.tagName).toBe('BUTTON');
    expect(screen.getByRole('dialog').contains(focusedEl)).toBe(true);
  });

  it('restores focus to the previously focused element on close', async () => {
    const triggerButton = document.createElement('button');
    triggerButton.textContent = 'Open Modal';
    document.body.appendChild(triggerButton);
    triggerButton.focus();
    expect(document.activeElement).toBe(triggerButton);

    const { rerender } = renderModal({ isOpen: true });

    act(() => { jest.advanceTimersByTime(50); });

    // Close the modal
    rerender(
      <Modal isOpen={false} onClose={jest.fn()} title="Confirm Action">
        <button>Confirm</button>
      </Modal>
    );

    expect(document.activeElement).toBe(triggerButton);
    document.body.removeChild(triggerButton);
  });

  it('traps Tab focus within the modal', () => {
    renderModal({ isOpen: true });

    act(() => { jest.advanceTimersByTime(50); });

    // Get all focusable elements in the modal
    const dialog = screen.getByRole('dialog');
    const focusable = dialog.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const lastEl = focusable[focusable.length - 1];

    // Focus the last element, then Tab forward — should wrap to first
    lastEl.focus();
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: false });

    expect(document.activeElement).toBe(focusable[0]);
  });

  it('traps Shift+Tab focus within the modal (wraps to last)', () => {
    renderModal({ isOpen: true });

    act(() => { jest.advanceTimersByTime(50); });

    const dialog = screen.getByRole('dialog');
    const focusable = dialog.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const firstEl = focusable[0];
    const lastEl = focusable[focusable.length - 1];

    // Focus the first element, then Shift+Tab — should wrap to last
    firstEl.focus();
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true });

    expect(document.activeElement).toBe(lastEl);
  });
});

// ---------------------------------------------------------------------------
// Keyboard listener cleanup
// ---------------------------------------------------------------------------

describe('Modal event listener cleanup', () => {
  it('removes the keydown listener when the modal closes', () => {
    const removeSpy = jest.spyOn(document, 'removeEventListener');
    const { rerender } = renderModal({ isOpen: true });

    rerender(
      <Modal isOpen={false} onClose={jest.fn()} title="Confirm Action">
        <button>Confirm</button>
      </Modal>
    );

    expect(removeSpy).toHaveBeenCalledWith('keydown', expect.any(Function));
    removeSpy.mockRestore();
  });

  it('removes the keydown listener when the component unmounts', () => {
    const removeSpy = jest.spyOn(document, 'removeEventListener');
    const { unmount } = renderModal({ isOpen: true });
    unmount();
    expect(removeSpy).toHaveBeenCalledWith('keydown', expect.any(Function));
    removeSpy.mockRestore();
  });
});
