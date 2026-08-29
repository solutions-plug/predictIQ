'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { api, ApiError } from '@/lib/api/admin-client';
import { Form, FormField, Input, Textarea, Button, StatusAlert } from '@/components/admin/Form';

interface ContentFields {
  hero_title: string;
  hero_subtitle: string;
  announcement_banner: string;
  announcement_url: string;
  feature_decentralized: string;
  feature_secure: string;
  feature_fast: string;
  footer_tagline: string;
  contact_email: string;
  [key: string]: string;
}

const DEFAULT_CONTENT: ContentFields = {
  hero_title: 'Predict the Future on Stellar',
  hero_subtitle:
    'Create, bet on, and resolve prediction markets with transparency, security, and fairness powered by the Stellar blockchain.',
  announcement_banner: '🚀 PredictIQ Beta is live on Stellar Mainnet! Join the waitlist for exclusive rewards.',
  announcement_url: 'https://predictiq.io/beta',
  feature_decentralized: 'Non-custodial smart contracts executed with verifiable on-chain logic.',
  feature_secure: 'Multi-oracle consensus and timelocked dispute resolution mechanisms.',
  feature_fast: 'Sub-second finality and near-zero transaction fees on Stellar Soroban.',
  footer_tagline: 'Decentralized prediction markets powered by the Stellar blockchain.',
  contact_email: 'support@predictiq.io',
};

export default function ContentManagementPage() {
  const [formData, setFormData] = useState<ContentFields>(DEFAULT_CONTENT);
  const [initialData, setInitialData] = useState<ContentFields>(DEFAULT_CONTENT);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [generalError, setGeneralError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [isSaving, setIsSaving] = useState<boolean>(false);

  // Fetch current published content
  const loadContent = useCallback(async () => {
    setIsLoading(true);
    setGeneralError(null);
    try {
      const data = await api.getContent();
      if (data && typeof data === 'object') {
        const merged: ContentFields = { ...DEFAULT_CONTENT };
        for (const [key, value] of Object.entries(data)) {
          if (typeof value === 'string') {
            merged[key] = value;
          }
        }
        setFormData(merged);
        setInitialData(merged);
      }
    } catch {
      // If fetching fails or endpoint is fresh, fallback gracefully to defaults
      setInitialData(DEFAULT_CONTENT);
      setFormData(DEFAULT_CONTENT);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadContent();
  }, [loadContent]);

  const handleFieldChange = (field: keyof ContentFields, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    // Clear field-level error as user edits the draft
    if (fieldErrors[field]) {
      setFieldErrors((prev) => {
        const next = { ...prev };
        delete next[field];
        return next;
      });
    }
  };

  const handleReset = () => {
    setFormData(initialData);
    setFieldErrors({});
    setGeneralError(null);
    setSuccessMessage(null);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();

    setIsSaving(true);
    setGeneralError(null);
    setSuccessMessage(null);
    setFieldErrors({});

    try {
      const res = await api.saveContent(formData);

      // Check if response reports business failure
      if (res && res.success === false) {
        setGeneralError((res.message as string) || 'Validation failed on server.');
        if (res.details && typeof res.details === 'object') {
          parseAndApplyFieldErrors(res.details as Record<string, unknown>);
        }
        return;
      }

      setSuccessMessage('Site content successfully updated and published.');
      setInitialData(formData);
    } catch (err) {
      if (err instanceof ApiError) {
        setGeneralError(`Save failed: ${err.message}`);

        // Surface server field-level validation errors
        if (err.details && typeof err.details === 'object') {
          parseAndApplyFieldErrors(err.details);
        }
      } else {
        setGeneralError(
          err instanceof Error ? err.message : 'An unexpected error occurred while saving content.'
        );
      }
    } finally {
      setIsSaving(false);
    }
  };

  /**
   * Helper to extract server field-level validation errors from various response formats
   * (e.g. { errors: { field: msg } }, { field_errors: { ... } }, or direct key-value map).
   */
  const parseAndApplyFieldErrors = (details: Record<string, unknown>) => {
    const extracted: Record<string, string> = {};

    const candidate =
      (details.field_errors as Record<string, unknown>) ||
      (details.errors as Record<string, unknown>) ||
      details;

    if (candidate && typeof candidate === 'object') {
      for (const [key, value] of Object.entries(candidate)) {
        if (typeof value === 'string') {
          extracted[key] = value;
        } else if (Array.isArray(value) && typeof value[0] === 'string') {
          extracted[key] = value[0];
        }
      }
    }

    if (Object.keys(extracted).length > 0) {
      setFieldErrors(extracted);
    }
  };

  const hasUnsavedChanges = JSON.stringify(formData) !== JSON.stringify(initialData);

  return (
    <div className="content-management-page">
      {/* Page Header */}
      <div className="admin-page-header">
        <div>
          <h1 className="admin-page-title">Editable Site Content Management</h1>
          <p className="admin-page-desc">
            Manage public landing-page copy, announcement banners, and marketing text. Server-side validation errors are surfaced directly on the affected fields without losing draft edits.
          </p>
        </div>

        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center' }}>
          <Button
            type="button"
            variant="secondary"
            onClick={handleReset}
            disabled={!hasUnsavedChanges || isSaving || isLoading}
          >
            Discard Changes
          </Button>
          <Button
            type="submit"
            form="content-edit-form"
            variant="primary"
            isLoading={isSaving}
            disabled={isLoading}
          >
            Save & Publish Content
          </Button>
        </div>
      </div>

      {/* Top Alerts */}
      {successMessage && (
        <StatusAlert
          type="success"
          title="Content Published"
          message={successMessage}
          onDismiss={() => setSuccessMessage(null)}
        />
      )}

      {generalError && (
        <StatusAlert
          type="error"
          title="Validation & Save Error"
          message={generalError}
          onDismiss={() => setGeneralError(null)}
        />
      )}

      {isLoading ? (
        <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--fg-muted)' }}>
          <span className="spinner" style={{ width: '32px', height: '32px', marginBottom: '1rem' }} />
          <p style={{ margin: 0, fontSize: 'var(--text-sm)' }}>Loading site content fields...</p>
        </div>
      ) : (
        <Form id="content-edit-form" onSubmit={handleSave}>
          <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: '1.5rem' }}>
            {/* Section 1: Hero & Announcements */}
            <div className="admin-card">
              <div className="admin-card-header">
                <h2 className="admin-card-title">Hero Section & Announcement</h2>
              </div>

              <FormField
                id="hero_title"
                label="Hero Headline (hero_title)"
                required
                hint="Main headline prominently displayed above the fold on the landing page."
                error={fieldErrors['hero_title']}
              >
                <Input
                  id="hero_title"
                  type="text"
                  value={formData.hero_title}
                  onChange={(e) => handleFieldChange('hero_title', e.target.value)}
                  error={fieldErrors['hero_title']}
                  required
                />
              </FormField>

              <FormField
                id="hero_subtitle"
                label="Hero Subtitle / Description (hero_subtitle)"
                required
                hint="Supporting value proposition underneath the primary headline."
                error={fieldErrors['hero_subtitle']}
              >
                <Textarea
                  id="hero_subtitle"
                  rows={3}
                  value={formData.hero_subtitle}
                  onChange={(e) => handleFieldChange('hero_subtitle', e.target.value)}
                  error={fieldErrors['hero_subtitle']}
                  required
                />
              </FormField>

              <FormField
                id="announcement_banner"
                label="Announcement Banner Text (announcement_banner)"
                hint="Top-level alert banner text displayed across the site header."
                error={fieldErrors['announcement_banner']}
              >
                <Input
                  id="announcement_banner"
                  type="text"
                  value={formData.announcement_banner}
                  onChange={(e) => handleFieldChange('announcement_banner', e.target.value)}
                  error={fieldErrors['announcement_banner']}
                />
              </FormField>

              <FormField
                id="announcement_url"
                label="Announcement Link URL (announcement_url)"
                hint="Target URL when users click the announcement banner."
                error={fieldErrors['announcement_url']}
              >
                <Input
                  id="announcement_url"
                  type="url"
                  placeholder="https://..."
                  value={formData.announcement_url}
                  onChange={(e) => handleFieldChange('announcement_url', e.target.value)}
                  error={fieldErrors['announcement_url']}
                />
              </FormField>
            </div>

            {/* Section 2: Features & Footer */}
            <div className="admin-card">
              <div className="admin-card-header">
                <h2 className="admin-card-title">Feature Highlights & Footer</h2>
              </div>

              <FormField
                id="feature_decentralized"
                label="Decentralized Feature Copy (feature_decentralized)"
                hint="Description for the Decentralized architecture card."
                error={fieldErrors['feature_decentralized']}
              >
                <Textarea
                  id="feature_decentralized"
                  rows={2}
                  value={formData.feature_decentralized}
                  onChange={(e) => handleFieldChange('feature_decentralized', e.target.value)}
                  error={fieldErrors['feature_decentralized']}
                />
              </FormField>

              <FormField
                id="feature_secure"
                label="Security Feature Copy (feature_secure)"
                hint="Description for the Multi-Oracle Consensus & Security card."
                error={fieldErrors['feature_secure']}
              >
                <Textarea
                  id="feature_secure"
                  rows={2}
                  value={formData.feature_secure}
                  onChange={(e) => handleFieldChange('feature_secure', e.target.value)}
                  error={fieldErrors['feature_secure']}
                />
              </FormField>

              <FormField
                id="feature_fast"
                label="Performance Feature Copy (feature_fast)"
                hint="Description for the Sub-Second Finality card."
                error={fieldErrors['feature_fast']}
              >
                <Textarea
                  id="feature_fast"
                  rows={2}
                  value={formData.feature_fast}
                  onChange={(e) => handleFieldChange('feature_fast', e.target.value)}
                  error={fieldErrors['feature_fast']}
                />
              </FormField>

              <FormField
                id="footer_tagline"
                label="Footer Tagline (footer_tagline)"
                hint="Brand tagline displayed in the site footer."
                error={fieldErrors['footer_tagline']}
              >
                <Input
                  id="footer_tagline"
                  type="text"
                  value={formData.footer_tagline}
                  onChange={(e) => handleFieldChange('footer_tagline', e.target.value)}
                  error={fieldErrors['footer_tagline']}
                />
              </FormField>

              <FormField
                id="contact_email"
                label="Official Contact Email (contact_email)"
                hint="Contact email shown on footer and legal pages."
                error={fieldErrors['contact_email']}
              >
                <Input
                  id="contact_email"
                  type="email"
                  value={formData.contact_email}
                  onChange={(e) => handleFieldChange('contact_email', e.target.value)}
                  error={fieldErrors['contact_email']}
                />
              </FormField>
            </div>
          </div>

          {/* Form Actions Footer */}
          <div
            style={{
              display: 'flex',
              justifyContent: 'flex-end',
              gap: '1rem',
              marginTop: '1.5rem',
              padding: '1rem',
              backgroundColor: 'var(--surface)',
              border: '1px solid var(--border)',
              borderRadius: 'var(--radius)',
            }}
          >
            <Button
              type="button"
              variant="secondary"
              onClick={handleReset}
              disabled={!hasUnsavedChanges || isSaving}
            >
              Discard Changes
            </Button>
            <Button
              type="submit"
              variant="primary"
              isLoading={isSaving}
            >
              Save & Publish Content
            </Button>
          </div>
        </Form>
      )}
    </div>
  );
}
