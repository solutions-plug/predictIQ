import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { TextInput } from '../TextInput';

describe('TextInput', () => {
  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  it('renders an <input> element', () => {
    render(<TextInput />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('renders without a label when label is omitted', () => {
    render(<TextInput />);
    expect(screen.queryByRole('label')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Label
  // -------------------------------------------------------------------------

  it('renders a label when label prop is provided', () => {
    render(<TextInput label="Email address" />);
    expect(screen.getByText('Email address')).toBeInTheDocument();
  });

  it('associates the label with the input via htmlFor', () => {
    render(<TextInput label="Username" />);
    expect(screen.getByLabelText('Username')).toBeInTheDocument();
  });

  it('uses the caller-supplied id when provided', () => {
    render(<TextInput id="my-input" label="Name" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('id', 'my-input');
    expect(screen.getByLabelText('Name')).toHaveAttribute('id', 'my-input');
  });

  it('renders a required asterisk when required=true', () => {
    render(<TextInput label="Name" required />);
    expect(screen.getByText('*')).toBeInTheDocument();
  });

  it('does not render a required asterisk when required is omitted', () => {
    render(<TextInput label="Name" />);
    expect(screen.queryByText('*')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Hint
  // -------------------------------------------------------------------------

  it('renders hint text when hint prop is provided', () => {
    render(<TextInput hint="Enter your email" />);
    expect(screen.getByText('Enter your email')).toBeInTheDocument();
  });

  it('does not render hint text when hint is omitted', () => {
    render(<TextInput />);
    expect(screen.queryByText('Enter your email')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Error / aria
  // -------------------------------------------------------------------------

  it('renders an error message in role=alert when error prop is provided', () => {
    render(<TextInput error="This field is required" />);
    expect(screen.getByRole('alert')).toHaveTextContent('This field is required');
  });

  it('does not render role=alert when error is omitted', () => {
    render(<TextInput />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('sets aria-invalid="true" when error is provided', () => {
    render(<TextInput error="Bad value" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true');
  });

  it('sets aria-invalid="false" when there is no error', () => {
    render(<TextInput />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'false');
  });

  it('sets aria-describedby pointing to the error element', () => {
    render(<TextInput id="email" error="Invalid email" />);
    const input = screen.getByRole('textbox');
    const describedById = input.getAttribute('aria-describedby');
    expect(describedById).toBeTruthy();
    expect(document.getElementById(describedById!)).toHaveTextContent('Invalid email');
  });

  it('sets aria-describedby to include the hint id when hint is present', () => {
    render(<TextInput id="email" hint="Format: user@example.com" error="Invalid email" />);
    const describedBy = screen.getByRole('textbox').getAttribute('aria-describedby') ?? '';
    const ids = describedBy.split(' ');
    expect(ids.length).toBe(2);
    expect(document.getElementById(ids[0])).toHaveTextContent('Format: user@example.com');
    expect(document.getElementById(ids[1])).toHaveTextContent('Invalid email');
  });

  it('does not set aria-describedby when neither hint nor error is present', () => {
    render(<TextInput />);
    expect(screen.getByRole('textbox')).not.toHaveAttribute('aria-describedby');
  });

  it('applies ui-input--error class on the input when error is set', () => {
    render(<TextInput error="Oops" />);
    expect(screen.getByRole('textbox')).toHaveClass('ui-input--error');
  });

  it('does not apply ui-input--error class when there is no error', () => {
    render(<TextInput />);
    expect(screen.getByRole('textbox')).not.toHaveClass('ui-input--error');
  });

  // -------------------------------------------------------------------------
  // Interaction
  // -------------------------------------------------------------------------

  it('accepts user input', async () => {
    render(<TextInput label="Name" />);
    const input = screen.getByRole('textbox');
    await userEvent.type(input, 'Alice');
    expect(input).toHaveValue('Alice');
  });

  it('forwards extra HTML attributes to the input', () => {
    render(<TextInput placeholder="Search…" maxLength={50} />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('placeholder', 'Search…');
    expect(input).toHaveAttribute('maxlength', '50');
  });

  // -------------------------------------------------------------------------
  // Ref forwarding
  // -------------------------------------------------------------------------

  it('forwards ref to the underlying <input>', () => {
    const ref = React.createRef<HTMLInputElement>();
    render(<TextInput ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });
});
