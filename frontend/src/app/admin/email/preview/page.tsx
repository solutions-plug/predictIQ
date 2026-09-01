'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { api, ApiError } from '@/lib/api/admin-client';
import { Form, FormField, Input, Select, Button, StatusAlert } from '@/components/admin/Form';

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

      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 340px', gap: '1.5rem', alignItems: 'start' }}>
        {/* Left Column: Preview Area */}
        <div>
          {/* Template Selection Card */}
          <div className="admin-card">
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '1rem', flexWrap: 'wrap' }}>
              <div style={{ flex: '1 1 260px' }}>
                <label
                  htmlFor="template-select"
                  style={{ display: 'block', fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: '0.4rem' }}
                >
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

              <div style={{ alignSelf: 'flex-end' }}>
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

            {currentTemplateInfo && (
              <p style={{ margin: '0.75rem 0 0', fontSize: 'var(--text-xs)', color: 'var(--fg-muted)' }}>
                {currentTemplateInfo.description}
              </p>
            )}
          </div>

          {/* Email Preview Container */}
          <div className="admin-card">
            {/* Header with Subject and Tabs */}
            <div className="admin-card-header" style={{ flexDirection: 'column', alignItems: 'stretch', gap: '0.75rem' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.5rem' }}>
                <div>
                  <span style={{ fontSize: 'var(--text-xs)', color: 'var(--fg-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                    Subject Line:
                  </span>
                  <div style={{ fontSize: 'var(--text-base)', fontWeight: 600, color: 'var(--fg)', marginTop: '0.2rem' }}>
                    {previewData?.subject || (isLoadingPreview ? 'Loading subject...' : '(No subject)')}
                  </div>
                </div>

                {/* View Tabs */}
                <div style={{ display: 'flex', gap: '0.35rem', backgroundColor: 'var(--surface-2)', padding: '0.25rem', borderRadius: 'var(--radius-sm)' }}>
                  <button
                    type="button"
                    onClick={() => setActiveTab('preview')}
                    style={{
                      border: 'none',
                      backgroundColor: activeTab === 'preview' ? 'var(--surface)' : 'transparent',
                      color: activeTab === 'preview' ? 'var(--gold)' : 'var(--fg-muted)',
                      padding: '0.35rem 0.75rem',
                      borderRadius: 'var(--radius-sm)',
                      fontSize: 'var(--text-xs)',
                      fontWeight: 600,
                      cursor: 'pointer',
                    }}
                  >
                    Sandboxed Preview
                  </button>
                  <button
                    type="button"
                    onClick={() => setActiveTab('text')}
                    style={{
                      border: 'none',
                      backgroundColor: activeTab === 'text' ? 'var(--surface)' : 'transparent',
                      color: activeTab === 'text' ? 'var(--gold)' : 'var(--fg-muted)',
                      padding: '0.35rem 0.75rem',
                      borderRadius: 'var(--radius-sm)',
                      fontSize: 'var(--text-xs)',
                      fontWeight: 600,
                      cursor: 'pointer',
                    }}
                  >
                    Plain Text
                  </button>
                  <button
                    type="button"
                    onClick={() => setActiveTab('html')}
                    style={{
                      border: 'none',
                      backgroundColor: activeTab === 'html' ? 'var(--surface)' : 'transparent',
                      color: activeTab === 'html' ? 'var(--gold)' : 'var(--fg-muted)',
                      padding: '0.35rem 0.75rem',
                      borderRadius: 'var(--radius-sm)',
                      fontSize: 'var(--text-xs)',
                      fontWeight: 600,
                      cursor: 'pointer',
                    }}
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
              <div style={{ padding: '3rem', textAlign: 'center', color: 'var(--fg-muted)' }}>
                <span className="spinner" style={{ width: '28px', height: '28px', marginBottom: '0.75rem' }} />
                <p style={{ margin: 0, fontSize: 'var(--text-sm)' }}>Rendering email template preview...</p>
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
                  <pre
                    style={{
                      backgroundColor: 'var(--surface-2)',
                      padding: '1.25rem',
                      borderRadius: 'var(--radius-sm)',
                      border: '1px solid var(--border)',
                      fontSize: 'var(--text-xs)',
                      fontFamily: 'monospace',
                      color: 'var(--fg)',
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                      maxHeight: '540px',
                      overflowY: 'auto',
                      margin: 0,
                    }}
                  >
                    {previewData.text_content || '(No plain text version available)'}
                  </pre>
                )}

                {activeTab === 'html' && (
                  <pre
                    style={{
                      backgroundColor: 'var(--surface-2)',
                      padding: '1.25rem',
                      borderRadius: 'var(--radius-sm)',
                      border: '1px solid var(--border)',
                      fontSize: 'var(--text-xs)',
                      fontFamily: 'monospace',
                      color: 'var(--fg-muted)',
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-all',
                      maxHeight: '540px',
                      overflowY: 'auto',
                      margin: 0,
                    }}
                  >
                    {previewData.html_content || '(Empty HTML content)'}
                  </pre>
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

            <p style={{ fontSize: 'var(--text-xs)', color: 'var(--fg-muted)', marginTop: 0, marginBottom: '1.25rem', lineHeight: 1.4 }}>
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

              <div style={{ marginTop: '1.5rem' }}>
                <Button
                  type="submit"
                  variant="primary"
                  isLoading={isSendingTest}
                  style={{ width: '100%' }}
                >
                  Send Test Email
                </Button>
              </div>
            </Form>
          </div>

          {/* Sandbox Security Notice */}
          <div
            style={{
              padding: '1rem',
              backgroundColor: 'rgba(245, 158, 11, 0.08)',
              border: '1px solid rgba(245, 158, 11, 0.2)',
              borderRadius: 'var(--radius-sm)',
              fontSize: 'var(--text-xs)',
              color: 'var(--fg-muted)',
              lineHeight: 1.4,
            }}
          >
            <div style={{ fontWeight: 600, color: 'var(--gold)', marginBottom: '0.25rem' }}>
              🔒 Sandboxed Rendering
            </div>
            Template HTML is rendered strictly inside an isolated <code>&lt;iframe sandbox=""&gt;</code> container to eliminate XSS risks and prevent arbitrary script execution.
          </div>
        </div>
      </div>
    </div>
  );
}
