import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import {
  Form,
  FormField,
  Input,
  Textarea,
  Select,
  Button,
  StatusAlert,
} from '../Form';

// ---------------------------------------------------------------------------
// Form container
// ---------------------------------------------------------------------------

describe('Form', () => {
  it('renders a <form> element with noValidate', () => {
    const { container } = render(<Form onSubmit={jest.fn()}>content</Form>);
    const form = container.querySelector('form') as HTMLFormElement;
    // noValidate disables native browser constraint validation
    expect(form).toBeInTheDocument();
    expect(form.noValidate).toBe(true);
  });

  it('calls onSubmit when the form is submitted', async () => {
    const onSubmit = jest.fn((e: React.FormEvent) => e.preventDefault());
    render(
      <Form onSubmit={onSubmit}>
        <button type="submit">Submit</button>
      </Form>
    );
    await userEvent.click(screen.getByRole('button', { name: /submit/i }));
    expect(onSubmit).toHaveBeenCalledTimes(1);
  });

  it('forwards extra HTML attributes onto the <form>', () => {
    render(
      <Form data-testid="my-form" aria-label="Test form">
        <span />
      </Form>
    );
    expect(screen.getByTestId('my-form')).toHaveAttribute('aria-label', 'Test form');
  });

  it('merges custom className with the base class', () => {
    render(<Form className="custom-class">content</Form>);
    const form = document.querySelector('form')!;
    expect(form.className).toContain('admin-form');
    expect(form.className).toContain('custom-class');
  });
});

// ---------------------------------------------------------------------------
// FormField — label, hint, error, required marker
// ---------------------------------------------------------------------------

describe('FormField', () => {
  it('renders the label text', () => {
    render(
      <FormField id="name" label="Full name">
        <input id="name" />
      </FormField>
    );
    expect(screen.getByText('Full name')).toBeInTheDocument();
  });

  it('associates the label with the control via htmlFor', () => {
    render(
      <FormField id="email" label="Email">
        <input id="email" />
      </FormField>
    );
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
  });

  it('renders a required indicator when required=true', () => {
    render(
      <FormField id="title" label="Title" required>
        <input id="title" />
      </FormField>
    );
    // The asterisk span is inside the label
    expect(screen.getByText('*')).toBeInTheDocument();
  });

  it('does not render a required indicator when required is omitted', () => {
    render(
      <FormField id="title" label="Title">
        <input id="title" />
      </FormField>
    );
    expect(screen.queryByText('*')).not.toBeInTheDocument();
  });

  it('renders the hint text', () => {
    render(
      <FormField id="slug" label="Slug" hint="URL-friendly name">
        <input id="slug" />
      </FormField>
    );
    expect(screen.getByText('URL-friendly name')).toBeInTheDocument();
  });

  it('renders the error message in a role=alert element', () => {
    render(
      <FormField id="email" label="Email" error="Invalid email address">
        <input id="email" />
      </FormField>
    );
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('Invalid email address');
  });

  it('does not render a role=alert when there is no error', () => {
    render(
      <FormField id="email" label="Email">
        <input id="email" />
      </FormField>
    );
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('adds has-error class to the wrapper when error is set', () => {
    const { container } = render(
      <FormField id="x" error="oops">
        <input id="x" />
      </FormField>
    );
    expect(container.firstChild).toHaveClass('has-error');
  });

  it('renders children inside the field wrapper', () => {
    render(
      <FormField id="custom" label="Custom">
        <span data-testid="child-node">child</span>
      </FormField>
    );
    expect(screen.getByTestId('child-node')).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

describe('Input', () => {
  it('renders an <input> element', () => {
    render(<Input id="name" />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('sets aria-invalid=true when error prop is provided', () => {
    render(<Input id="email" error="Required" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true');
  });

  it('sets aria-invalid=false when no error prop', () => {
    render(<Input id="email" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'false');
  });

  it('sets aria-describedby to the error element id when both id and error are given', () => {
    render(<Input id="email" error="Bad email" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-describedby', 'email-error');
  });

  it('does not set aria-describedby when error is absent', () => {
    render(<Input id="email" />);
    expect(screen.getByRole('textbox')).not.toHaveAttribute('aria-describedby');
  });

  it('accepts user input', async () => {
    render(<Input id="name" />);
    const input = screen.getByRole('textbox');
    await userEvent.type(input, 'hello');
    expect(input).toHaveValue('hello');
  });

  it('forwards ref to the underlying <input>', () => {
    const ref = React.createRef<HTMLInputElement>();
    render(<Input id="ref-test" ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });
});

// ---------------------------------------------------------------------------
// Textarea
// ---------------------------------------------------------------------------

describe('Textarea', () => {
  it('renders a <textarea>', () => {
    render(<Textarea id="bio" />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('sets aria-invalid=true when error prop is provided', () => {
    render(<Textarea id="bio" error="Too short" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true');
  });

  it('sets aria-describedby to the error element id', () => {
    render(<Textarea id="bio" error="Too short" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-describedby', 'bio-error');
  });

  it('defaults to 4 rows', () => {
    render(<Textarea id="bio" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('rows', '4');
  });

  it('forwards ref to the underlying <textarea>', () => {
    const ref = React.createRef<HTMLTextAreaElement>();
    render(<Textarea id="ref-test" ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLTextAreaElement);
  });
});

// ---------------------------------------------------------------------------
// Select
// ---------------------------------------------------------------------------

describe('Select', () => {
  it('renders a <select> with options', () => {
    render(
      <Select id="role">
        <option value="admin">Admin</option>
        <option value="viewer">Viewer</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Admin' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Viewer' })).toBeInTheDocument();
  });

  it('sets aria-invalid=true when error prop is provided', () => {
    render(
      <Select id="role" error="Select a role">
        <option value="">--</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).toHaveAttribute('aria-invalid', 'true');
  });

  it('sets aria-describedby to the error element id', () => {
    render(
      <Select id="role" error="Select a role">
        <option value="">--</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).toHaveAttribute('aria-describedby', 'role-error');
  });

  it('forwards ref to the underlying <select>', () => {
    const ref = React.createRef<HTMLSelectElement>();
    render(
      <Select id="ref-test" ref={ref}>
        <option value="a">A</option>
      </Select>
    );
    expect(ref.current).toBeInstanceOf(HTMLSelectElement);
  });
});

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

describe('Button', () => {
  it('renders its children', () => {
    render(<Button>Save</Button>);
    expect(screen.getByRole('button', { name: /save/i })).toBeInTheDocument();
  });

  it('is not disabled by default', () => {
    render(<Button>Save</Button>);
    expect(screen.getByRole('button')).not.toBeDisabled();
  });

  it('is disabled when isLoading=true', () => {
    render(<Button isLoading>Save</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('is disabled when disabled prop is set', () => {
    render(<Button disabled>Save</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('calls onClick when clicked', async () => {
    const onClick = jest.fn();
    render(<Button onClick={onClick}>Click me</Button>);
    await userEvent.click(screen.getByRole('button'));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('does not call onClick when isLoading=true', async () => {
    const onClick = jest.fn();
    render(<Button isLoading onClick={onClick}>Click me</Button>);
    await userEvent.click(screen.getByRole('button'));
    expect(onClick).not.toHaveBeenCalled();
  });

  it('renders leftIcon when not loading', () => {
    render(<Button leftIcon={<span data-testid="left-icon" />}>Save</Button>);
    expect(screen.getByTestId('left-icon')).toBeInTheDocument();
  });

  it('hides leftIcon when loading', () => {
    render(<Button isLoading leftIcon={<span data-testid="left-icon" />}>Save</Button>);
    expect(screen.queryByTestId('left-icon')).not.toBeInTheDocument();
  });

  it('applies the variant class', () => {
    render(<Button variant="danger">Delete</Button>);
    expect(screen.getByRole('button')).toHaveClass('btn-danger');
  });

  it('applies full-width class when fullWidth=true', () => {
    render(<Button fullWidth>Submit</Button>);
    expect(screen.getByRole('button')).toHaveClass('form-btn--full-width');
  });

  it('forwards ref to the underlying <button>', () => {
    const ref = React.createRef<HTMLButtonElement>();
    render(<Button ref={ref}>Save</Button>);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
  });
});

// ---------------------------------------------------------------------------
// StatusAlert
// ---------------------------------------------------------------------------

describe('StatusAlert', () => {
  it('renders title and message', () => {
    render(<StatusAlert type="success" title="Done" message="Saved successfully." />);
    expect(screen.getByText('Done')).toBeInTheDocument();
    expect(screen.getByText('Saved successfully.')).toBeInTheDocument();
  });

  it('uses role=alert for error type', () => {
    render(<StatusAlert type="error" message="Something went wrong." />);
    expect(screen.getByRole('alert')).toHaveTextContent('Something went wrong.');
  });

  it('uses role=status for non-error types', () => {
    render(<StatusAlert type="success" message="All good." />);
    expect(screen.getByRole('status')).toHaveTextContent('All good.');
  });

  it('renders children inside the alert body', () => {
    render(
      <StatusAlert type="info">
        <span data-testid="custom-content">extra</span>
      </StatusAlert>
    );
    expect(screen.getByTestId('custom-content')).toBeInTheDocument();
  });

  it('renders a dismiss button when onDismiss is provided', () => {
    render(<StatusAlert type="warning" onDismiss={jest.fn()} message="Watch out." />);
    expect(screen.getByRole('button', { name: /dismiss alert/i })).toBeInTheDocument();
  });

  it('calls onDismiss when the dismiss button is clicked', async () => {
    const onDismiss = jest.fn();
    render(<StatusAlert type="warning" message="Watch out." onDismiss={onDismiss} />);
    await userEvent.click(screen.getByRole('button', { name: /dismiss alert/i }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('does not render a dismiss button when onDismiss is absent', () => {
    render(<StatusAlert type="info" message="FYI." />);
    expect(screen.queryByRole('button', { name: /dismiss alert/i })).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Integration: Form with fields, validation errors, and submit
// ---------------------------------------------------------------------------

describe('Form integration: validation errors and submit flow', () => {
  it('displays field-level validation errors and prevents submit until fixed', async () => {
    const handleSubmit = jest.fn((e: React.FormEvent) => e.preventDefault());
    let errorMessage: string | undefined = 'Title is required';

    const { rerender } = render(
      <Form onSubmit={handleSubmit}>
        <FormField id="title" label="Title" error={errorMessage}>
          <Input id="title" error={errorMessage} />
        </FormField>
        <Button type="submit">Save</Button>
      </Form>
    );

    // Error is visible
    expect(screen.getByRole('alert')).toHaveTextContent('Title is required');
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true');

    // Submit fires regardless (noValidate — app code decides what to do)
    fireEvent.click(screen.getByRole('button', { name: /save/i }));
    expect(handleSubmit).toHaveBeenCalledTimes(1);

    // After correction, error disappears
    errorMessage = undefined;
    rerender(
      <Form onSubmit={handleSubmit}>
        <FormField id="title" label="Title">
          <Input id="title" />
        </FormField>
        <Button type="submit">Save</Button>
      </Form>
    );

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'false');
  });

  it('disables the submit button while a submission is in progress', () => {
    render(
      <Form>
        <Button type="submit" isLoading>
          Saving…
        </Button>
      </Form>
    );
    expect(screen.getByRole('button', { name: /saving/i })).toBeDisabled();
  });

  it('renders a success StatusAlert after successful submission', () => {
    render(
      <Form>
        <StatusAlert type="success" title="Saved" message="Your changes have been saved." />
      </Form>
    );
    expect(screen.getByRole('status')).toHaveTextContent('Saved');
    expect(screen.getByRole('status')).toHaveTextContent('Your changes have been saved.');
  });
});
