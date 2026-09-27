import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { Button } from '../Button';

describe('Button', () => {
  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  it('renders its children', () => {
    render(<Button>Save</Button>);
    expect(screen.getByRole('button', { name: /save/i })).toBeInTheDocument();
  });

  it('defaults to type="button"', () => {
    render(<Button>Click</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('type', 'button');
  });

  it('respects an explicit type="submit"', () => {
    render(<Button type="submit">Submit</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('type', 'submit');
  });

  it('applies the primary variant class by default', () => {
    render(<Button>Default</Button>);
    expect(screen.getByRole('button')).toHaveClass('ui-btn--primary');
  });

  it.each(['primary', 'secondary', 'danger', 'ghost'] as const)(
    'applies the %s variant class',
    (variant) => {
      render(<Button variant={variant}>Btn</Button>);
      expect(screen.getByRole('button')).toHaveClass(`ui-btn--${variant}`);
    }
  );

  it('merges a custom className', () => {
    render(<Button className="my-class">Btn</Button>);
    const btn = screen.getByRole('button');
    expect(btn).toHaveClass('ui-btn');
    expect(btn).toHaveClass('my-class');
  });

  // -------------------------------------------------------------------------
  // Disabled / loading states
  // -------------------------------------------------------------------------

  it('is not disabled by default', () => {
    render(<Button>Btn</Button>);
    expect(screen.getByRole('button')).not.toBeDisabled();
  });

  it('is disabled when disabled prop is set', () => {
    render(<Button disabled>Btn</Button>);
    const btn = screen.getByRole('button');
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute('aria-disabled', 'true');
  });

  it('is disabled when isLoading=true', () => {
    render(<Button isLoading>Btn</Button>);
    const btn = screen.getByRole('button');
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute('aria-disabled', 'true');
  });

  it('sets aria-busy="true" when isLoading=true', () => {
    render(<Button isLoading>Btn</Button>);
    expect(screen.getByRole('button')).toHaveAttribute('aria-busy', 'true');
  });

  it('does not set aria-busy when not loading', () => {
    render(<Button>Btn</Button>);
    expect(screen.getByRole('button')).not.toHaveAttribute('aria-busy');
  });

  it('renders a spinner element when isLoading=true', () => {
    const { container } = render(<Button isLoading>Btn</Button>);
    expect(container.querySelector('.ui-btn__spinner')).toBeInTheDocument();
  });

  it('does not render a spinner when not loading', () => {
    const { container } = render(<Button>Btn</Button>);
    expect(container.querySelector('.ui-btn__spinner')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Icons
  // -------------------------------------------------------------------------

  it('renders leftIcon when not loading', () => {
    render(<Button leftIcon={<span data-testid="left" />}>Btn</Button>);
    expect(screen.getByTestId('left')).toBeInTheDocument();
  });

  it('renders rightIcon when not loading', () => {
    render(<Button rightIcon={<span data-testid="right" />}>Btn</Button>);
    expect(screen.getByTestId('right')).toBeInTheDocument();
  });

  it('hides leftIcon when isLoading=true', () => {
    render(<Button isLoading leftIcon={<span data-testid="left" />}>Btn</Button>);
    expect(screen.queryByTestId('left')).not.toBeInTheDocument();
  });

  it('hides rightIcon when isLoading=true', () => {
    render(<Button isLoading rightIcon={<span data-testid="right" />}>Btn</Button>);
    expect(screen.queryByTestId('right')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Interaction
  // -------------------------------------------------------------------------

  it('calls onClick when clicked', async () => {
    const onClick = jest.fn();
    render(<Button onClick={onClick}>Click me</Button>);
    await userEvent.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('does not fire onClick when disabled', async () => {
    const onClick = jest.fn();
    render(<Button disabled onClick={onClick}>Btn</Button>);
    await userEvent.click(screen.getByRole('button'));
    expect(onClick).not.toHaveBeenCalled();
  });

  it('does not fire onClick when isLoading', async () => {
    const onClick = jest.fn();
    render(<Button isLoading onClick={onClick}>Btn</Button>);
    await userEvent.click(screen.getByRole('button'));
    expect(onClick).not.toHaveBeenCalled();
  });

  // -------------------------------------------------------------------------
  // Ref forwarding
  // -------------------------------------------------------------------------

  it('forwards ref to the underlying <button>', () => {
    const ref = React.createRef<HTMLButtonElement>();
    render(<Button ref={ref}>Btn</Button>);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
  });
});
