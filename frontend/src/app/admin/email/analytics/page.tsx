'use client';

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { api, ApiError } from '@/lib/api/admin-client';
import { Select, Button, StatusAlert } from '@/components/admin/Form';
import './analytics.css';

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
        <div className="u-flex u-items-center u-justify-between u-gap-lg u-flex-wrap">
          <div className="u-flex u-gap-lg u-flex-wrap u-flex-1">
            <div className="analytics-filter analytics-filter--template">
              <label htmlFor="template-filter" className="analytics-filter__label">
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

            <div className="analytics-filter analytics-filter--days">
              <label htmlFor="days-filter" className="analytics-filter__label">
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

          <div className="u-self-end">
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
          <span className="admin-metric-value u-color-success">{deliveryRate}</span>
          <span className="admin-metric-sub">
            {aggregatedTotals.delivered.toLocaleString()} of {aggregatedTotals.sent.toLocaleString()}
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Open Rate</span>
          <span className="admin-metric-value u-color-gold">{openRate}</span>
          <span className="admin-metric-sub">
            {aggregatedTotals.opened.toLocaleString()} unique opens
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Click Rate (CTR)</span>
          <span className="admin-metric-value u-color-purple">{clickRate}</span>
          <span className="admin-metric-sub">
            {aggregatedTotals.clicked.toLocaleString()} link clicks (CTOR: {clickToOpenRate})
          </span>
        </div>

        <div className="admin-metric-card">
          <span className="admin-metric-label">Bounce Rate</span>
          <span className={`admin-metric-value ${aggregatedTotals.bounced > 0 ? 'u-color-danger' : ''}`}>
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
          <span className="u-text-xs u-muted">Showing {analyticsData.length} records</span>
        </div>

        {/* Loading State */}
        {isLoading && (
          <div className="analytics-loading">
            <span className="spinner spinner--md analytics-loading__spinner" />
            <p className="analytics-loading__text">Loading email analytics metrics...</p>
          </div>
        )}

        {/* Zero-send / Empty State */}
        {!isLoading && analyticsData.length === 0 && (
          <div className="analytics-empty">
            <h3 className="analytics-empty__title">No Email Activity Found</h3>
            <p className="analytics-empty__desc">
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
                  <th scope="col" className="u-text-right">Sent</th>
                  <th scope="col" className="u-text-right">Delivered</th>
                  <th scope="col" className="u-text-right">Delivery %</th>
                  <th scope="col" className="u-text-right">Opened</th>
                  <th scope="col" className="u-text-right">Open %</th>
                  <th scope="col" className="u-text-right">Clicked</th>
                  <th scope="col" className="u-text-right">Click %</th>
                  <th scope="col" className="u-text-right">Bounced</th>
                  <th scope="col" className="u-text-right">Bounce %</th>
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
                      <td className="analytics-date-cell">{row.date}</td>
                      <td>
                        <span className="analytics-template-chip">{row.template_name}</span>
                      </td>
                      <td className="u-text-right">{(row.sent_count || 0).toLocaleString()}</td>
                      <td className="u-text-right">{(row.delivered_count || 0).toLocaleString()}</td>
                      <td className="u-text-right u-color-success">{rowDeliveryRate}</td>
                      <td className="u-text-right">{(row.opened_count || 0).toLocaleString()}</td>
                      <td className="u-text-right u-color-gold">{rowOpenRate}</td>
                      <td className="u-text-right">{(row.clicked_count || 0).toLocaleString()}</td>
                      <td className="u-text-right u-color-purple">{rowClickRate}</td>
                      <td className="u-text-right">{(row.bounced_count || 0).toLocaleString()}</td>
                      <td className={`u-text-right ${row.bounced_count > 0 ? 'u-color-danger' : 'u-muted'}`}>
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
