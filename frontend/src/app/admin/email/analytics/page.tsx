'use client';

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { api, ApiError } from '@/lib/api/admin-client';
import { Select, Button, StatusAlert } from '@/components/admin/Form';

export interface EmailAnalyticsRecord {
  template_name: string;
  variant_name?: string;
  date: string;
  sent_count: number;
  delivered_count: number;
  opened_count: number;
  clicked_count: number;
  bounced_count: number;
  complained_count?: number;
  unsubscribed_count?: number;
}

/**
 * Safe rate calculation helper.
 * Strictly guards against division by zero (e.g. brand new deploy or zero-send period).
 * Guaranteed to return "0%" (or specified fallback) instead of NaN/Infinity.
 */
export function computeRate(
  numerator: number | undefined | null,
  denominator: number | undefined | null,
  fallback: string = '0%'
): string {
  const num = Number(numerator ?? 0);
  const den = Number(denominator ?? 0);

  if (!den || den <= 0 || isNaN(num) || isNaN(den) || !isFinite(den) || !isFinite(num)) {
    return fallback;
  }

  const rate = (num / den) * 100;
  if (isNaN(rate) || !isFinite(rate)) {
    return fallback;
  }

  return `${rate.toFixed(1)}%`;
}

const TEMPLATE_FILTER_OPTIONS = [
  { value: '', label: 'All Templates' },
  { value: 'newsletter_confirmation', label: 'Newsletter Confirmation' },
  { value: 'waitlist_confirmation', label: 'Waitlist Confirmation' },
  { value: 'contact_form_auto_response', label: 'Contact Form Auto-Response' },
  { value: 'welcome_email', label: 'Welcome Email' },
];

const DAYS_OPTIONS = [
  { value: 7, label: 'Last 7 Days' },
  { value: 14, label: 'Last 14 Days' },
  { value: 30, label: 'Last 30 Days' },
  { value: 90, label: 'Last 90 Days' },
  { value: 365, label: 'Last 365 Days' },
];

export default function EmailAnalyticsPage() {
  const [selectedTemplate, setSelectedTemplate] = useState<string>('');
  const [selectedDays, setSelectedDays] = useState<number>(30);
  const [analyticsData, setAnalyticsData] = useState<EmailAnalyticsRecord[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const fetchAnalytics = useCallback(async () => {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const data = await api.getEmailAnalytics({
        template_name: selectedTemplate || undefined,
        days: selectedDays,
      });

      if (Array.isArray(data)) {
        setAnalyticsData(data as EmailAnalyticsRecord[]);
      } else if (data && typeof data === 'object' && Array.isArray((data as Record<string, unknown>).records)) {
        setAnalyticsData((data as Record<string, unknown>).records as EmailAnalyticsRecord[]);
      } else {
        setAnalyticsData([]);
      }
    } catch (err) {
      if (err instanceof ApiError) {
        setErrorMessage(`Failed to fetch email analytics: ${err.message} (${err.status})`);
      } else {
        setErrorMessage('Failed to load email analytics. Please try again.');
      }
      setAnalyticsData([]);
    } finally {
      setIsLoading(false);
    }
  }, [selectedTemplate, selectedDays]);

  useEffect(() => {
    fetchAnalytics();
  }, [fetchAnalytics]);

  // Aggregate totals across all returned records
  const aggregatedTotals = useMemo(() => {
    return analyticsData.reduce(
      (acc, item) => {
        acc.sent += item.sent_count || 0;
        acc.delivered += item.delivered_count || 0;
        acc.opened += item.opened_count || 0;
        acc.clicked += item.clicked_count || 0;
        acc.bounced += item.bounced_count || 0;
        acc.complained += item.complained_count || 0;
        acc.unsubscribed += item.unsubscribed_count || 0;
        return acc;
      },
      {
        sent: 0,
        delivered: 0,
        opened: 0,
        clicked: 0,
        bounced: 0,
        complained: 0,
        unsubscribed: 0,
      }
    );
  }, [analyticsData]);

  // Computed summary rates with zero-division safety
  const openRate = computeRate(aggregatedTotals.opened, aggregatedTotals.delivered || aggregatedTotals.sent, '0%');
  const clickRate = computeRate(aggregatedTotals.clicked, aggregatedTotals.delivered || aggregatedTotals.sent, '0%');
  const clickToOpenRate = computeRate(aggregatedTotals.clicked, aggregatedTotals.opened, '0%');
  const bounceRate = computeRate(aggregatedTotals.bounced, aggregatedTotals.sent, '0%');
  const deliveryRate = computeRate(aggregatedTotals.delivered, aggregatedTotals.sent, '0%');
  const unsubscribeRate = computeRate(aggregatedTotals.unsubscribed, aggregatedTotals.sent, '0%');

  return (
    <div className="email-analytics-page">
      {/* Page Header */}
      <div className="admin-page-header">
        <div>
          <h1 className="admin-page-title">Email Delivery Analytics</h1>
          <p className="admin-page-desc">
            Monitor email send volume, delivery reliability, open rates, and user engagement metrics across all transactional and marketing templates.
          </p>
        </div>
      </div>

      {/* Filter Controls Card */}
      <div className="admin-card">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '1rem', flexWrap: 'wrap' }}>
          <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', flex: 1 }}>
            <div style={{ minWidth: '220px' }}>
              <label
                htmlFor="template-filter"
                style={{ display: 'block', fontSize: 'var(--text-xs)', fontWeight: 600, color: 'var(--fg-muted)', marginBottom: '0.35rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}
              >
                Template
              </label>
              <Select
                id="template-filter"
                value={selectedTemplate}
                onChange={(e) => setSelectedTemplate(e.target.value)}
                aria-label="Filter by email template"
              >
                {TEMPLATE_FILTER_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </Select>
            </div>

            <div style={{ minWidth: '160px' }}>
              <label
                htmlFor="days-filter"
                style={{ display: 'block', fontSize: 'var(--text-xs)', fontWeight: 600, color: 'var(--fg-muted)', marginBottom: '0.35rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}
              >
                Time Window
              </label>
              <Select
                id="days-filter"
                value={selectedDays}
                onChange={(e) => setSelectedDays(Number(e.target.value))}
                aria-label="Filter by time range in days"
              >
                {DAYS_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </Select>
            </div>
          </div>

          <div style={{ alignSelf: 'flex-end' }}>
            <Button
              variant="secondary"
              onClick={fetchAnalytics}
              isLoading={isLoading}
              aria-label="Refresh email analytics"
            >
              Refresh Analytics
            </Button>
          </div>
        </div>
      </div>

      {/* Error Alert */}
      {errorMessage && (
        <StatusAlert
          type="error"
          title="Analytics Error"
          message={errorMessage}
          onDismiss={() => setErrorMessage(null)}
        />
      )}

      {/* Summary KPI Cards */}
      <div className="admin-metrics-grid">
        <div className="admin-metric-card">
          <span className="admin-metric-label">Total Sent</span>
          <span className="admin-metric-value">{aggregatedTotals.sent.toLocaleString()}</span>
          <span className="admin-metric-sub">
            {aggregatedTotals.sent === 0 ? 'Zero sends recorded in period' : `${aggregatedTotals.delivered.toLocaleString()} delivered`}
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Delivery Rate</span>
          <span className="admin-metric-value" style={{ color: 'var(--success)' }}>
            {deliveryRate}
          </span>
          <span className="admin-metric-sub">
            {aggregatedTotals.delivered.toLocaleString()} of {aggregatedTotals.sent.toLocaleString()}
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Open Rate</span>
          <span className="admin-metric-value" style={{ color: 'var(--gold)' }}>
            {openRate}
          </span>
          <span className="admin-metric-sub">
            {aggregatedTotals.opened.toLocaleString()} unique opens
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Click Rate (CTR)</span>
          <span className="admin-metric-value" style={{ color: 'var(--purple-soft)' }}>
            {clickRate}
          </span>
          <span className="admin-metric-sub">
            {aggregatedTotals.clicked.toLocaleString()} link clicks (CTOR: {clickToOpenRate})
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Bounce Rate</span>
          <span
            className="admin-metric-value"
            style={{ color: aggregatedTotals.bounced > 0 ? 'var(--destructive)' : 'var(--fg)' }}
          >
            {bounceRate}
          </span>
          <span className="admin-metric-sub">
            {aggregatedTotals.bounced.toLocaleString()} bounced
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Unsubscribe / Complaints</span>
          <span className="admin-metric-value">
            {aggregatedTotals.unsubscribed.toLocaleString()} / {aggregatedTotals.complained.toLocaleString()}
          </span>
          <span className="admin-metric-sub">
            Unsub rate: {unsubscribeRate}
          </span>
        </div>
      </div>

      {/* Detailed Breakdown Card */}
      <div className="admin-card">
        <div className="admin-card-header">
          <h2 className="admin-card-title">Daily & Template Breakdown</h2>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--fg-muted)' }}>
            Showing {analyticsData.length} records
          </span>
        </div>

        {/* Loading State */}
        {isLoading && (
          <div style={{ padding: '3rem', textAlign: 'center', color: 'var(--fg-muted)' }}>
            <span className="spinner" style={{ width: '28px', height: '28px', marginBottom: '0.75rem' }} />
            <p style={{ margin: 0, fontSize: 'var(--text-sm)' }}>Loading email analytics metrics...</p>
          </div>
        )}

        {/* Zero-send / Empty State */}
        {!isLoading && analyticsData.length === 0 && (
          <div
            style={{
              padding: '3rem 1.5rem',
              textAlign: 'center',
              backgroundColor: 'var(--surface-2)',
              borderRadius: 'var(--radius-sm)',
              border: '1px dashed var(--border)',
            }}
          >
            <h3 style={{ margin: '0 0 0.5rem', fontSize: 'var(--text-base)', color: 'var(--fg)' }}>
              No Email Activity Found
            </h3>
            <p style={{ margin: 0, fontSize: 'var(--text-sm)', color: 'var(--fg-muted)', maxWidth: '480px', marginInline: 'auto' }}>
              No emails were recorded during the selected period. Computed rates remain safely at <strong>0%</strong> (or N/A) without division-by-zero errors.
            </p>
          </div>
        )}

        {/* Table View */}
        {!isLoading && analyticsData.length > 0 && (
          <div className="admin-table-container">
            <table className="admin-table">
              <thead>
                <tr>
                  <th scope="col">Date</th>
                  <th scope="col">Template</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Sent</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Delivered</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Delivery %</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Opened</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Open %</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Clicked</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Click %</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Bounced</th>
                  <th scope="col" style={{ textAlign: 'right' }}>Bounce %</th>
                </tr>
              </thead>
              <tbody>
                {analyticsData.map((row, idx) => {
                  const rowDeliveryRate = computeRate(row.delivered_count, row.sent_count, '0%');
                  const rowOpenRate = computeRate(row.opened_count, row.delivered_count || row.sent_count, '0%');
                  const rowClickRate = computeRate(row.clicked_count, row.delivered_count || row.sent_count, '0%');
                  const rowBounceRate = computeRate(row.bounced_count, row.sent_count, '0%');

                  return (
                    <tr key={`${row.template_name}-${row.date}-${idx}`}>
                      <td style={{ fontWeight: 500 }}>{row.date}</td>
                      <td>
                        <span
                          style={{
                            fontFamily: 'monospace',
                            fontSize: 'var(--text-xs)',
                            backgroundColor: 'var(--surface-2)',
                            padding: '0.2rem 0.4rem',
                            borderRadius: '4px',
                          }}
                        >
                          {row.template_name}
                        </span>
                      </td>
                      <td style={{ textAlign: 'right' }}>{(row.sent_count || 0).toLocaleString()}</td>
                      <td style={{ textAlign: 'right' }}>{(row.delivered_count || 0).toLocaleString()}</td>
                      <td style={{ textAlign: 'right', color: 'var(--success)' }}>{rowDeliveryRate}</td>
                      <td style={{ textAlign: 'right' }}>{(row.opened_count || 0).toLocaleString()}</td>
                      <td style={{ textAlign: 'right', color: 'var(--gold)' }}>{rowOpenRate}</td>
                      <td style={{ textAlign: 'right' }}>{(row.clicked_count || 0).toLocaleString()}</td>
                      <td style={{ textAlign: 'right', color: 'var(--purple-soft)' }}>{rowClickRate}</td>
                      <td style={{ textAlign: 'right' }}>{(row.bounced_count || 0).toLocaleString()}</td>
                      <td style={{ textAlign: 'right', color: row.bounced_count > 0 ? 'var(--destructive)' : 'var(--fg-muted)' }}>
                        {rowBounceRate}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
