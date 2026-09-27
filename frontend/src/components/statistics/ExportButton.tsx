import React, { useState } from 'react';
import { useI18n } from '../../lib/hooks/useI18n';
import './ExportButton.css';

export interface ExportSection {
  title: string;
  rows: Record<string, string | number>[];
}

interface ExportButtonProps {
  /** Data exactly as currently rendered on screen (post-filter). No network fetch happens here. */
  sections: ExportSection[];
  filenamePrefix: string;
  disabled?: boolean;
}

/**
 * UTF-8 byte order mark. Excel mis-detects the encoding of BOM-less UTF-8
 * CSVs and can mangle non-ASCII characters (e.g. market titles) on open, so
 * the CSV export is prefixed with this marker.
 */
const UTF8_BOM = '\uFEFF';

/**
 * Formats a numeric CSV cell with a fixed `.` decimal separator and no
 * thousands grouping, so files opened in a different-locale spreadsheet app
 * (which may treat `,` as the decimal separator or the column delimiter)
 * don't silently corrupt the value.
 */
function formatCsvNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2);
}

function formatCsvCell(value: string | number): string {
  const text = typeof value === 'number' ? formatCsvNumber(value) : value;
  if (/[",\n]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`;
  }
  return text;
}

function sectionsToCsv(sections: ExportSection[]): string {
  const blocks = sections.map(({ title, rows }) => {
    const lines = [`# ${title}`];
    if (rows.length > 0) {
      const headers = Object.keys(rows[0]);
      lines.push(headers.join(','));
      for (const row of rows) {
        lines.push(headers.map((key) => formatCsvCell(row[key] ?? '')).join(','));
      }
    }
    return lines.join('\n');
  });
  return UTF8_BOM + blocks.join('\n\n');
}

function sectionsToJson(sections: ExportSection[]): string {
  const payload = {
    exportedAt: new Date().toISOString(),
    sections: Object.fromEntries(sections.map(({ title, rows }) => [title, rows])),
  };
  return JSON.stringify(payload, null, 2);
}

function triggerDownload(filename: string, mimeType: string, content: string): string | null {
  try {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    URL.revokeObjectURL(url);
    return 'Export ready';
  } catch (error) {
    return `Export failed: ${error instanceof Error ? error.message : 'Unknown error'}`;
  }
}

export const ExportButton: React.FC<ExportButtonProps> = ({ sections, filenamePrefix, disabled }) => {
  const { t } = useI18n();
  const [liveRegionMessage, setLiveRegionMessage] = useState('');
  const isDisabled = disabled || sections.every((section) => section.rows.length === 0);

  const handleExportCsv = () => {
    const message = triggerDownload(`${filenamePrefix}.csv`, 'text/csv;charset=utf-8', sectionsToCsv(sections));
    if (message) setLiveRegionMessage(message);
  };

  const handleExportJson = () => {
    const message = triggerDownload(`${filenamePrefix}.json`, 'application/json;charset=utf-8', sectionsToJson(sections));
    if (message) setLiveRegionMessage(message);
  };

  return (
    <>
      <div
        className="sr-only"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {liveRegionMessage}
      </div>
      <div className="export-button-group" role="group" aria-label={t('exportButton.groupAriaLabel')}>
        <button type="button" className="export-button" onClick={handleExportCsv} disabled={isDisabled}>
          {t('exportButton.exportCsv')}
        </button>
        <button type="button" className="export-button" onClick={handleExportJson} disabled={isDisabled}>
          {t('exportButton.exportJson')}
        </button>
      </div>
    </>
  );
};
