import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import ExportButton from './ExportButton';

// Capture the Blob content passed to URL.createObjectURL so we can assert on it.
let capturedBlob: Blob | null = null;

beforeEach(() => {
  capturedBlob = null;
  vi.spyOn(URL, 'createObjectURL').mockImplementation((blob: Blob) => {
    capturedBlob = blob;
    return 'blob:mock';
  });
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

const sampleSections = [
  {
    title: 'Markets',
    rows: [
      { label: 'Café ☕', value: 42 },
      { label: 'Plain', value: 7 },
    ],
  },
];

describe('ExportButton', () => {
  it('prefixes the CSV export with a UTF-8 BOM', async () => {
    render(<ExportButton sections={sampleSections} filename="stats" />);

    fireEvent.click(screen.getByRole('button', { name: /csv/i }));

    await waitFor(() => expect(capturedBlob).not.toBeNull());
    const text = await capturedBlob!.text();
    expect(text.startsWith('\uFEFF')).toBe(true);
    expect(text).toContain('Café ☕');
  });

  it('does not prefix the JSON export with a BOM', async () => {
    render(<ExportButton sections={sampleSections} filename="stats" />);

    fireEvent.click(screen.getByRole('button', { name: /json/i }));

    await waitFor(() => expect(capturedBlob).not.toBeNull());
    const text = await capturedBlob!.text();
    expect(text.startsWith('\uFEFF')).toBe(false);
    expect(() => JSON.parse(text)).not.toThrow();
  });
});
