import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { Textarea } from '../Textarea';

describe('Textarea', () => {
  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  it('renders a <textarea> element', () => {
    render(<Textarea />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('defaults to 4 rows', () => {
    render(<Textarea />);
    expect(screen.getByRole('textbox')).toHaveAttribute('rows', '4');
  });

  it('respects a custom rows value', () => {
    render(<Textarea rows={8} />);
    expect(screen.getByRole('textbox')).toHaveAttribute('rows', '8');
  });

  // -------------------------------------------------------------------------
  // Label
  // -------------------------------------------------------------------------

  it('renders a label when label prop is provided', () => {
    render(<Textarea label="Description" />);
    expect(screen.getByText('Description')).toBeInTheDocument();
  });

  it('associates the label with the textarea via htmlFor', () => {
    render(<Textarea label="Bio" />);
    expect(screen.getByLabelText('Bio')).toBeInTheDocument();
  });

  it('uses the caller-supplied id when provided', () => {
    render(<Textarea id="my-ta" label="Notes" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('id', 'my-ta');
    expect(screen.getByLabelText('Notes')).toHaveAttribute('id', 'my-ta');
  });

  it('renders a required asterisk when required=true', () => {
    render(<Textarea label="Notes" required />);
    expect(screen.getByText('*')).toBeInTheDocument();
  });

  it('does not render a required asterisk when required is omitted', () => {
    render(<Textarea label="Notes" />);
    expect(screen.queryByText('*')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Hint
  // -------------------------------------------------------------------------

  it('renders hint text when hint prop is provided', () => {
    render(<Textarea hint="Max 500 characters" />);
    expect(screen.getByText('Max 500 characters')).toBeInTheDocument();
  });

  it('does not render hint text when hint is omitted', () => {
    render(<Textarea />);
    expect(screen.queryByText('Max 500 characters')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Error / aria
  // -------------------------------------------------------------------------

  it('renders an error message in role=alert when error prop is provided', () => {
    render(<Textarea error="Description is required" />);
    expect(screen.getByRole('alert')).toHaveTextContent('Description is required');
  });

  it('does not render role=alert when error is omitted', () => {
    render(<Textarea />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('sets aria-invalid="true" when error is provided', () => {
    render(<Textarea error="Too short" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true');
  });

  it('sets aria-invalid="false" when there is no error', () => {
    render(<Textarea />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'false');
  });

  it('sets aria-describedby pointing to the error element', () => {
    render(<Textarea id="desc" error="Too short" />);
    const ta = screen.getByRole('textbox');
    const describedById = ta.getAttribute('aria-describedby');
    expect(describedById).toBeTruthy();
    expect(document.getElementById(describedById!)).toHaveTextContent('Too short');
  });

  it('sets aria-describedby to include hint id when hint is present', () => {
    render(<Textarea id="desc" hint="Write a summary" error="Required" />);
    const describedBy = screen.getByRole('textbox').getAttribute('aria-describedby') ?? '';
    const ids = describedBy.split(' ');
    expect(ids.length).toBe(2);
    expect(document.getElementById(ids[0])).toHaveTextContent('Write a summary');
    expect(document.getElementById(ids[1])).toHaveTextContent('Required');
  });

  it('does not set aria-describedby when neither hint nor error is present', () => {
    render(<Textarea />);
    expect(screen.getByRole('textbox')).not.toHaveAttribute('aria-describedby');
  });

  it('applies ui-textarea--error class on the textarea when error is set', () => {
    render(<Textarea error="Oops" />);
    expect(screen.getByRole('textbox')).toHaveClass('ui-textarea--error');
  });

  it('does not apply ui-textarea--error class when there is no error', () => {
    render(<Textarea />);
    expect(screen.getByRole('textbox')).not.toHaveClass('ui-textarea--error');
  });

  // -------------------------------------------------------------------------
  // Interaction
  // -------------------------------------------------------------------------

  it('accepts user input', async () => {
    render(<Textarea label="Bio" />);
    const ta = screen.getByRole('textbox');
    await userEvent.type(ta, 'Hello world');
    expect(ta).toHaveValue('Hello world');
  });

  // -------------------------------------------------------------------------
  // Ref forwarding
  // -------------------------------------------------------------------------

  it('forwards ref to the underlying <textarea>', () => {
    const ref = React.createRef<HTMLTextAreaElement>();
    render(<Textarea ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLTextAreaElement);
  });
});
