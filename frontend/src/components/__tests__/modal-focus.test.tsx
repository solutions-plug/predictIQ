import React, { useState } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LegacyModal from '../Modal';
import { Modal as TrapModal } from '../ui/Modal';

function Harness({ Comp }: { Comp: typeof TrapModal }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button onClick={() => setOpen(true)}>Open</button>
      <Comp open={open} onClose={() => setOpen(false)} title="Dialog">
        <button>First</button>
        <button>Last</button>
      </Comp>
    </>
  );
}

describe('Modal focus management', () => {
  it('wraps Tab from the last focusable element to the first', async () => {
    const user = userEvent.setup();
    render(<Harness Comp={TrapModal} />);
    await user.click(screen.getByText('Open'));
    const dialog = screen.getByRole('dialog');
    const focusables = Array.from(dialog.querySelectorAll<HTMLElement>('button'));
    focusables[focusables.length - 1].focus();
    await user.tab();
    expect(dialog.contains(document.activeElement)).toBe(true);
    expect(document.activeElement).toBe(focusables[0]);
  });

  it.each([
    ['legacy Modal', LegacyModal],
    ['ui Modal', TrapModal],
  ])('%s restores focus to the trigger on close', async (_n, Comp) => {
    const user = userEvent.setup();
    render(<Harness Comp={Comp as typeof TrapModal} />);
    const trigger = screen.getByText('Open');
    await user.click(trigger);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it('renders nothing and does not trap focus while closed', async () => {
    const user = userEvent.setup();
    render(<Harness Comp={TrapModal} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await user.tab();
    expect(screen.getByText('Open')).toHaveFocus();
  });
});
