import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { Select } from '../Select';

describe('Select', () => {
  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  it('renders a <select> element', () => {
    render(
      <Select>
        <option value="a">Option A</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).toBeInTheDocument();
  });

  it('renders options as children', () => {
    render(
      <Select>
        <option value="yes">Yes</option>
        <option value="no">No</option>
      </Select>
    );
    expect(screen.getByRole('option', { name: 'Yes' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'No' })).toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Label
  // -------------------------------------------------------------------------

  it('renders a label when label prop is provided', () => {
    render(
      <Select label="Status">
        <option value="active">Active</option>
      </Select>
    );
    expect(screen.getByText('Status')).toBeInTheDocument();
  });

  it('associates label with the select via htmlFor/id', () => {
    render(
      <Select label="Status">
        <option value="active">Active</option>
      </Select>
    );
    expect(screen.getByLabelText('Status')).toBeInTheDocument();
  });

  it('uses a caller-supplied id when provided', () => {
    render(
      <Select id="my-select" label="Status">
        <option value="a">A</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).toHaveAttribute('id', 'my-select');
  });

  it('renders a required asterisk when required=true', () => {
    render(
      <Select label="Status" required>
        <option value="a">A</option>
      </Select>
    );
    expect(screen.getByText('*')).toBeInTheDocument();
  });

  it('does not render a required asterisk when required is omitted', () => {
    render(
      <Select label="Status">
        <option value="a">A</option>
      </Select>
    );
    expect(screen.queryByText('*')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Hint
  // -------------------------------------------------------------------------

  it('renders hint text when hint prop is provided', () => {
    render(
      <Select hint="Choose the current status">
        <option value="a">A</option>
      </Select>
    );
    expect(screen.getByText('Choose the current status')).toBeInTheDocument();
  });

  it('does not render hint text when hint is omitted', () => {
    render(
      <Select>
        <option value="a">A</option>
      </Select>
    );
    expect(screen.queryByText('Choose the current status')).not.toBeInTheDocument();
  });

  // -------------------------------------------------------------------------
  // Error / aria
  // -------------------------------------------------------------------------

  it('renders an error message in role=alert when error prop is provided', () => {
    render(
      <Select error="Selection required">
        <option value="">--</option>
      </Select>
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Selection required');
  });

  it('does not render role=alert when error is omitted', () => {
    render(
      <Select>
        <option value="a">A</option>
      </Select>
    );
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('sets aria-invalid="true" when error is provided', () => {
    render(
      <Select error="Required">
        <option value="">--</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).toHaveAttribute('aria-invalid', 'true');
  });

  it('sets aria-invalid="false" when there is no error', () => {
    render(
      <Select>
        <option value="a">A</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).toHaveAttribute('aria-invalid', 'false');
  });

  it('sets aria-describedby pointing to the error element', () => {
    render(
      <Select id="s" error="Bad value">
        <option value="">--</option>
      </Select>
    );
    const select = screen.getByRole('combobox');
    const describedById = select.getAttribute('aria-describedby');
    expect(describedById).toBeTruthy();
    expect(document.getElementById(describedById!)).toHaveTextContent('Bad value');
  });

  it('sets aria-describedby to include the hint id when hint is present', () => {
    render(
      <Select id="s2" hint="Pick one" error="Required">
        <option value="">--</option>
      </Select>
    );
    const describedBy = screen.getByRole('combobox').getAttribute('aria-describedby') ?? '';
    const ids = describedBy.split(' ');
    expect(ids.length).toBe(2);
    expect(document.getElementById(ids[0])).toHaveTextContent('Pick one');
    expect(document.getElementById(ids[1])).toHaveTextContent('Required');
  });

  it('does not set aria-describedby when neither hint nor error is present', () => {
    render(
      <Select>
        <option value="a">A</option>
      </Select>
    );
    expect(screen.getByRole('combobox')).not.toHaveAttribute('aria-describedby');
  });

  // -------------------------------------------------------------------------
  // Interaction
  // -------------------------------------------------------------------------

  it('reflects a user selection', async () => {
    render(
      <Select label="Pick">
        <option value="a">A</option>
        <option value="b">B</option>
      </Select>
    );
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    await userEvent.selectOptions(select, 'b');
    expect(select.value).toBe('b');
  });

  // -------------------------------------------------------------------------
  // Ref forwarding
  // -------------------------------------------------------------------------

  it('forwards ref to the underlying <select>', () => {
    const ref = React.createRef<HTMLSelectElement>();
    render(
      <Select ref={ref}>
        <option value="a">A</option>
      </Select>
    );
    expect(ref.current).toBeInstanceOf(HTMLSelectElement);
  });
});
