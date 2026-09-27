'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { api, ApiError } from '@/lib/api/admin-client';
import { Form, FormField, Input, Select, Button, StatusAlert } from '@/components/admin/Form';
import './preview.css';

interface TemplateOption {
  value: string;
  label: string;
  description: string;
}

const TEMPLATES: TemplateOption[] = [
  {
    value: 'newsletter_confirmation',
    label: 'Newsletter Confirmation',
    description: 'Sent when a user signs up for the newsletter to confirm their subscription.',
  },
  {
    value: 'waitlist_confirmation',
    label: 'Waitlist Confirmation',
    description: 'Sent when a user joins the early access waitlist.',
  },
  {
    value: 'contact_form_auto_response',
    label: 'Contact Form Auto-Response',
    description: 'Automated acknowledgment sent when an inquiry is submitted.',
  },
  {
    value: 'welcome_email',
    label: 'Welcome Email',
    description: 'Onboarding email with dashboard and documentation links.',
  },
];

interface EmailPreviewData {
  subject?: string;
  html_content?: string;
  text_content?: string;
  [key: string]: unknown;
}

export default function EmailPreviewPage() {
  const [selectedTemplate, setSelectedTemplate] = useState<string>(TEMPLATES[0].value);
  const [previewData, setPreviewData] = useState<EmailPreviewData | null>(null);
  const [isLoadingPreview, setIsLoadingPreview] = useState<boolean>(true);
  const [previewError, setPreviewError] = useState<string | null>(null);

  const [activeTab, setActiveTab] = useState<'preview' | 'text' | 'html'>('preview');

  // Test send state
  const [recipient, setRecipient] = useState<string>('');
  const [recipientError, setRecipientError] = useState<string>('');
  const [isSendingTest, setIsSendingTest] = useState<boolean>(false);
  const [sendSuccessMessage, setSendSuccessMessage] = useState<string | null>(null);
  const [sendErrorMessage, setSendErrorMessage] = useState<string | null>(null);

  // Fetch preview when template changes
  const fetchPreview = useCallback(async (templateName: string) => {
    setIsLoadingPreview(true);
    setPreviewError(null);
    try {
      const data = await api.emailPreview(templateName);
      setPreviewData(data as EmailPreviewData);
    } catch (err) {
      if (err instanceof ApiError) {
        setPreviewError(`Failed to load template preview: ${err.message} (${err.status})`);
      } else {
        setPreviewError('Failed to load email preview. Please check your connection.');
      }
      setPreviewData(null);
    } finally {
      setIsLoadingPreview(false);
    }
  }, []);

  useEffect(() => {
    fetchPreview(selectedTemplate);
  }, [selectedTemplate, fetchPreview]);

  // Handle test send submit
  const handleTestSend = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!recipient.trim()) {
      setRecipientError('Recipient email address is required.');
      return;
    }

    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(recipient.trim())) {
      setRecipientError('Please enter a valid email address.');
      return;
    }

    setRecipientError('');
    setSendSuccessMessage(null);
    setSendErrorMessage(null);
    setIsSendingTest(true);

    try {
      const res = await api.emailSendTest({
        recipient: recipient.trim(),
        template_name: selectedTemplate,
      });

      if (res && res.success !== false) {
        setSendSuccessMessage(
          `Test email sent successfully to ${recipient.trim()} (Message ID: ${res.message_id || 'N/A'})`
        );
        setRecipient('');
      } else {
        setSendErrorMessage(res.message || 'Failed to send test email.');
      }
    } catch (err) {
      if (err instanceof ApiError) {
        setSendErrorMessage(`Send failed: ${err.message}`);
      } else {
        setSendErrorMessage('An unexpected error occurred while sending the test email.');
      }
    } finally {
      setIsSendingTest(false);
    }
  };

  const currentTemplateInfo = TEMPLATES.find((t) => t.value === selectedTemplate);

  return (
    <div className="email-preview-page">
      {/* Page Header */}
      <div className="admin-page-header">
        <div>
          <h1 className="admin-page-title">Email Template Preview & Test Send</h1>
          <p className="admin-page-desc">
            Preview rendered email templates inside an isolated sandbox and dispatch live test emails to verify rendering and deliverability.
          </p>
        </div>
      </div>

      <div className="email-preview-columns">
        {/* Left Column: Preview Area */}
        <div>
          {/* Template Selection Card */}
          <div className="admin-card">
            <div className="u-flex u-items-center u-justify-between u-gap-lg u-flex-wrap">
              <div className="email-preview-select-wrap">
                <label htmlFor="template-select" className="email-preview-label">
                  Select Email Template
                </label>
                <Select
                  id="template-select"
                  value={selectedTemplate}
                  onChange={(e) => setSelectedTemplate(e.target.value)}
                  aria-label="Select Email Template to Preview"
                >
                  {TEMPLATES.map((tpl) => (
                    <option key={tpl.value} value={tpl.value}>
                      {tpl.label} ({tpl.value})
                    </option>
                  ))}
                </Select>
              </div>

              <div className="u-self-end">
                <Button
                  variant="secondary"
                  onClick={() => fetchPreview(selectedTemplate)}
                  isLoading={isLoadingPreview}
                  aria-label="Refresh template preview"
                >
                  Refresh Preview
                </Button>
              </div>
            </div>

            {currentTemplateInfo && <p className="email-preview-desc">{currentTemplateInfo.description}</p>}
          </div>

          {/* Email Preview Container */}
          <div className="admin-card">
            {/* Header with Subject and Tabs */}
            <div className="admin-card-header admin-card-header--stacked">
              <div className="u-flex u-justify-between u-items-center u-flex-wrap u-gap-sm">
                <div>
                  <span className="u-text-xs u-muted u-uppercase">Subject Line:</span>
                  <div className="email-preview-subject-value">
                    {previewData?.subject || (isLoadingPreview ? 'Loading subject...' : '(No subject)')}
                  </div>
                </div>

                {/* View Tabs */}
                <div className="email-preview-tabs">
                  <button
                    type="button"
                    onClick={() => setActiveTab('preview')}
                    className={`email-preview-tab ${activeTab === 'preview' ? 'email-preview-tab--active' : ''}`}
                  >
                    Sandboxed Preview
                  </button>
                  <button
                    type="button"
                    onClick={() => setActiveTab('text')}
                    className={`email-preview-tab ${activeTab === 'text' ? 'email-preview-tab--active' : ''}`}
                  >
                    Plain Text
                  </button>
                  <button
                    type="button"
                    onClick={() => setActiveTab('html')}
                    className={`email-preview-tab ${activeTab === 'html' ? 'email-preview-tab--active' : ''}`}
                  >
                    HTML Source
                  </button>
                </div>
              </div>
            </div>

            {/* Error state */}
            {previewError && (
              <StatusAlert
                type="error"
                title="Preview Error"
                message={previewError}
                onDismiss={() => setPreviewError(null)}
              />
            )}

            {/* Loading State */}
            {isLoadingPreview && (
              <div className="email-preview-loading">
                <span className="spinner spinner--md email-preview-loading__spinner" />
                <p className="email-preview-loading__text">Rendering email template preview...</p>
              </div>
            )}

            {/* Preview Content */}
            {!isLoadingPreview && previewData && (
              <div>
                {activeTab === 'preview' && (
                  <div className="admin-iframe-wrapper">
                    {/*
                      SECURITY CRITICAL REQUIREMENT:
                      Render inside a sandboxed <iframe> using srcDoc instead of directly injecting
                      arbitrary HTML into the admin page's DOM via dangerouslySetInnerHTML.
                      The sandbox attribute ensures scripts and dangerous actions are blocked,
                      while external images and fonts safely load.
                    */}
                    <iframe
                      srcDoc={previewData.html_content || '<p style="font-family:sans-serif;color:#666;padding:2rem;">No HTML content rendered.</p>'}
                      sandbox=""
                      title={`Sandboxed Email Preview for ${selectedTemplate}`}
                      className="email-preview-iframe"
                      aria-label="Email template HTML preview"
                    />
                  </div>
                )}

                {activeTab === 'text' && (
                  <pre className="email-preview-pre">{previewData.text_content || '(No plain text version available)'}</pre>
                )}

                {activeTab === 'html' && (
                  <pre className="email-preview-pre email-preview-pre--html">{previewData.html_content || '(Empty HTML content)'}</pre>
                )}
              </div>
            )}
          </div>
        </div>

        {/* Right Column: Test Send Panel */}
        <div>
          <div className="admin-card">
            <div className="admin-card-header">
              <h3 className="admin-card-title">Send Test Email</h3>
            </div>

            <p className="email-preview-test-desc">
              Dispatches a live test email using the selected template (<strong>{selectedTemplate}</strong>) with sample variables to your inbox.
            </p>

            {sendSuccessMessage && (
              <StatusAlert
                type="success"
                title="Email Dispatched"
                message={sendSuccessMessage}
                onDismiss={() => setSendSuccessMessage(null)}
              />
            )}

            {sendErrorMessage && (
              <StatusAlert
                type="error"
                title="Send Failed"
                message={sendErrorMessage}
                onDismiss={() => setSendErrorMessage(null)}
              />
            )}

            <Form onSubmit={handleTestSend}>
              <FormField
                id="recipient-email"
                label="Recipient Email Address"
                required
                hint="Sample variables will be populated automatically"
                error={recipientError}
              >
                <Input
                  id="recipient-email"
                  type="email"
                  placeholder="admin@example.com"
                  value={recipient}
                  onChange={(e) => {
                    setRecipient(e.target.value);
                    if (recipientError) setRecipientError('');
                  }}
                  error={recipientError}
                  disabled={isSendingTest}
                  required
                />
              </FormField>

              <div className="u-mt-xl">
                <Button type="submit" variant="primary" isLoading={isSendingTest} fullWidth>
                  Send Test Email
                </Button>
              </div>
            </Form>
          </div>

          {/* Sandbox Security Notice */}
          <div className="email-preview-sandbox-notice">
            <div className="email-preview-sandbox-notice__title">🔒 Sandboxed Rendering</div>
            Template HTML is rendered strictly inside an isolated <code>&lt;iframe sandbox=""&gt;</code> container to eliminate XSS risks and prevent arbitrary script execution.
          </div>
        </div>
      </div>
    </div>
  );
}
