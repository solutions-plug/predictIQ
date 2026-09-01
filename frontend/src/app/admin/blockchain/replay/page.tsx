'use client';

import React, { useState } from 'react';
import { api, ApiError } from '@/lib/api/admin-client';
import { Modal } from '@/components/admin/Modal';
import { Form, FormField, Input, Button, StatusAlert } from '@/components/admin/Form';

interface ReplayResult {
  from_ledger?: number;
  to_ledger?: number;
  events_replayed?: number;
  status?: string;
  message?: string;
  timestamp: string;
  [key: string]: unknown;
}

const REQUIRED_CONFIRM_PHRASE = 'CONFIRM REPLAY';

export default function BlockchainReplayPage() {
  const [fromLedger, setFromLedger] = useState<string>('');
  const [fromLedgerError, setFromLedgerError] = useState<string>('');

  // Permission tier simulation / state
  const [hasPermission, setHasPermission] = useState<boolean>(true);

  // Modal & Confirmation state
  const [isConfirmModalOpen, setIsConfirmModalOpen] = useState<boolean>(false);
  const [confirmPhrase, setConfirmPhrase] = useState<string>('');
  const [confirmPhraseError, setConfirmPhraseError] = useState<string>('');

  // Execution state
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [executionResult, setExecutionResult] = useState<ReplayResult | null>(null);
  const [executionError, setExecutionError] = useState<string | null>(null);
  const [auditLogs, setAuditLogs] = useState<ReplayResult[]>([]);

  // Validate form and open confirmation dialog
  const handleInitiateReplay = (e: React.FormEvent) => {
    e.preventDefault();

    if (!hasPermission) {
      setFromLedgerError('Your session does not have permission to trigger blockchain replays.');
      return;
    }

    const ledgerNum = parseInt(fromLedger, 10);
    if (!fromLedger || isNaN(ledgerNum) || ledgerNum <= 0) {
      setFromLedgerError('Please enter a valid positive ledger sequence number (e.g. 100000).');
      return;
    }

    setFromLedgerError('');
    setConfirmPhrase('');
    setConfirmPhraseError('');
    setIsConfirmModalOpen(true);
  };

  // Submit replay request after confirmation
  const handleConfirmAndExecute = async () => {
    if (confirmPhrase.trim() !== REQUIRED_CONFIRM_PHRASE) {
      setConfirmPhraseError(`You must type "${REQUIRED_CONFIRM_PHRASE}" exactly to proceed.`);
      return;
    }

    const ledgerNum = parseInt(fromLedger, 10);
    if (isNaN(ledgerNum) || ledgerNum <= 0) {
      return;
    }

    setIsExecuting(true);
    setExecutionError(null);
    setExecutionResult(null);

    try {
      const res = await api.blockchainReplay({ from_ledger: ledgerNum });
      const record: ReplayResult = {
        from_ledger: (res && typeof res.from_ledger === 'number') ? res.from_ledger : ledgerNum,
        to_ledger: (res && typeof res.to_ledger === 'number') ? res.to_ledger : undefined,
        events_replayed: (res && typeof res.events_replayed === 'number') ? res.events_replayed : 0,
        status: (res && typeof res.status === 'string') ? res.status : 'COMPLETED',
        message: (res && typeof res.message === 'string') ? res.message : 'Events replayed successfully',
        timestamp: new Date().toISOString(),
      };

      setExecutionResult(record);
      setAuditLogs((prev) => [record, ...prev]);
      setIsConfirmModalOpen(false);
      setFromLedger('');
      setConfirmPhrase('');
    } catch (err) {
      if (err instanceof ApiError) {
        setExecutionError(`Replay operation failed: ${err.message} (${err.status})`);
      } else {
        setExecutionError('An unexpected network or server error occurred during replay execution.');
      }
      setIsConfirmModalOpen(false);
    } finally {
      setIsExecuting(false);
    }
  };

  const isConfirmationPhraseValid = confirmPhrase.trim() === REQUIRED_CONFIRM_PHRASE;

  return (
    <div className="blockchain-replay-page">
      {/* Page Header */}
      <div className="admin-page-header">
        <div>
          <h1 className="admin-page-title">Blockchain Event Replay Tooling</h1>
          <p className="admin-page-desc">
            Operational tool to reprocess Soroban contract events and sync missing ledger ranges. This is a state-mutating operation with no automated undo.
          </p>
        </div>
      </div>

      {/* Permission Tier Status Banner */}
      <div className="admin-card" style={{ marginBottom: '1.5rem', backgroundColor: hasPermission ? 'var(--surface)' : 'rgba(248, 113, 113, 0.08)' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: '1rem' }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.25rem' }}>
              <span style={{ fontSize: 'var(--text-xs)', textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--fg-muted)', fontWeight: 600 }}>
                Authorization Status:
              </span>
              <span
                style={{
                  fontSize: 'var(--text-xs)',
                  fontWeight: 700,
                  padding: '0.15rem 0.5rem',
                  borderRadius: 'var(--radius-pill)',
                  backgroundColor: hasPermission ? 'rgba(52, 211, 153, 0.15)' : 'rgba(248, 113, 113, 0.15)',
                  color: hasPermission ? 'var(--success)' : 'var(--destructive)',
                }}
              >
                {hasPermission ? 'blockchain:replay GRANTED (Super Admin)' : 'ACCESS DENIED (Lacks permission)'}
              </span>
            </div>
            <p style={{ margin: 0, fontSize: 'var(--text-xs)', color: 'var(--fg-muted)' }}>
              {hasPermission
                ? 'Your authenticated admin session holds the operational role required to initiate ledger state replays.'
                : 'This action is fully disabled for sessions without explicit blockchain:replay authorization.'}
            </p>
          </div>

          <button
            type="button"
            onClick={() => setHasPermission(!hasPermission)}
            style={{
              fontSize: 'var(--text-xs)',
              padding: '0.35rem 0.65rem',
              backgroundColor: 'var(--surface-2)',
              border: '1px solid var(--border)',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--fg-muted)',
              cursor: 'pointer',
            }}
            title="Toggle permission tier to test disabled edge case"
          >
            {hasPermission ? 'Simulate Lower Permission Tier' : 'Restore Admin Permissions'}
          </button>
        </div>
      </div>

      {/* Execution Feedback Alerts */}
      {executionResult && (
        <StatusAlert
          type="success"
          title="Blockchain Replay Completed"
          message={`Successfully replayed ${executionResult.events_replayed} events starting from ledger #${executionResult.from_ledger}.`}
          onDismiss={() => setExecutionResult(null)}
        />
      )}

      {executionError && (
        <StatusAlert
          type="error"
          title="Replay Execution Failed"
          message={executionError}
          onDismiss={() => setExecutionError(null)}
        />
      )}

      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 380px', gap: '1.5rem', alignItems: 'start' }}>
        {/* Left Column: Replay Request Form */}
        <div className="admin-card">
          <div className="admin-card-header">
            <h2 className="admin-card-title">Trigger Ledger Replay</h2>
          </div>

          {!hasPermission && (
            <div
              role="alert"
              style={{
                backgroundColor: 'rgba(248, 113, 113, 0.1)',
                border: '1px solid var(--destructive)',
                borderRadius: 'var(--radius-sm)',
                padding: '0.85rem 1rem',
                color: 'var(--destructive)',
                fontSize: 'var(--text-sm)',
                marginBottom: '1.25rem',
                fontWeight: 500,
              }}
            >
              🔒 <strong>Operational Action Locked:</strong> Your current admin session does not possess the <code>blockchain:replay</code> permission. The controls below are disabled.
            </div>
          )}

          <Form onSubmit={handleInitiateReplay}>
            <FormField
              id="from-ledger"
              label="Starting Ledger Sequence (from_ledger)"
              required
              hint="Specify the starting ledger number from which missing events will be fetched and reprocessed."
              error={fromLedgerError}
            >
              <Input
                id="from-ledger"
                type="number"
                min="1"
                step="1"
                placeholder="e.g. 5240192"
                value={fromLedger}
                onChange={(e) => {
                  setFromLedger(e.target.value);
                  if (fromLedgerError) setFromLedgerError('');
                }}
                error={fromLedgerError}
                disabled={!hasPermission || isExecuting}
                required
              />
            </FormField>

            <div style={{ marginTop: '1.75rem', display: 'flex', gap: '1rem', alignItems: 'center' }}>
              <Button
                type="submit"
                variant="danger"
                disabled={!hasPermission || isExecuting || !fromLedger}
                isLoading={isExecuting}
              >
                Initiate Blockchain Replay
              </Button>
            </div>
          </Form>

          {/* Operational Notes */}
          <div
            style={{
              marginTop: '2rem',
              padding: '1rem',
              backgroundColor: 'var(--surface-2)',
              borderRadius: 'var(--radius-sm)',
              border: '1px solid var(--border)',
              fontSize: 'var(--text-xs)',
              color: 'var(--fg-muted)',
              lineHeight: 1.5,
            }}
          >
            <h3 style={{ margin: '0 0 0.5rem', fontSize: 'var(--text-xs)', color: 'var(--fg)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Operational Considerations
            </h3>
            <ul style={{ margin: 0, paddingLeft: '1.2rem' }}>
              <li>Replays bypass standard ingestion deduplication by design.</li>
              <li>Running overlapping replays concurrently can degrade database throughput.</li>
              <li>Always check Soroban RPC node rate limits prior to selecting large ledger spans.</li>
            </ul>
          </div>
        </div>

        {/* Right Column: Execution History & Audit */}
        <div className="admin-card">
          <div className="admin-card-header">
            <h3 className="admin-card-title">Recent Session Replays</h3>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--fg-muted)' }}>
              {auditLogs.length} logged
            </span>
          </div>

          {auditLogs.length === 0 ? (
            <p style={{ fontSize: 'var(--text-sm)', color: 'var(--fg-muted)', margin: 0, textAlign: 'center', padding: '2rem 1rem' }}>
              No replays triggered in this session.
            </p>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
              {auditLogs.map((log, index) => (
                <div
                  key={index}
                  style={{
                    backgroundColor: 'var(--surface-2)',
                    border: '1px solid var(--border)',
                    borderRadius: 'var(--radius-sm)',
                    padding: '0.75rem 1rem',
                    fontSize: 'var(--text-xs)',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.25rem' }}>
                    <span style={{ fontWeight: 600, color: 'var(--fg)' }}>
                      Ledger #{log.from_ledger}
                    </span>
                    <span style={{ color: 'var(--success)', fontWeight: 600 }}>
                      {log.events_replayed} events
                    </span>
                  </div>
                  <div style={{ color: 'var(--fg-muted)' }}>
                    {new Date(log.timestamp).toLocaleTimeString()} — {log.status}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/*
        CRITICAL REQUIREMENT (#16):
        Confirmation Modal with:
        - disableBackdropDismiss={true}: Clicking outside overlay does NOT dismiss
        - disableEscapeKey={true}: Accidental escape key does NOT dismiss
        - Explicit confirmation phrase required ("CONFIRM REPLAY")
      */}
      <Modal
        isOpen={isConfirmModalOpen}
        onClose={() => {
          if (!isExecuting) {
            setIsConfirmModalOpen(false);
            setConfirmPhrase('');
            setConfirmPhraseError('');
          }
        }}
        title="⚠️ Confirm State-Mutating Action"
        description="Double confirmation required for operational blockchain replay"
        disableBackdropDismiss={true}
        disableEscapeKey={true}
        maxWidth="540px"
      >
        <div>
          {/* High-severity warning block */}
          <div
            style={{
              backgroundColor: 'rgba(248, 113, 113, 0.12)',
              border: '1px solid var(--destructive)',
              borderRadius: 'var(--radius-sm)',
              padding: '1rem',
              color: 'var(--fg)',
              marginBottom: '1.25rem',
            }}
          >
            <div style={{ fontWeight: 700, color: 'var(--destructive)', marginBottom: '0.4rem', fontSize: 'var(--text-sm)' }}>
              WARNING: Irreversible Operation
            </div>
            <p style={{ margin: 0, fontSize: 'var(--text-xs)', lineHeight: 1.5 }}>
              You are about to reprocess blockchain events starting from ledger <strong>#{fromLedger}</strong>. This operational mutation will re-evaluate contract states in the database and cannot be undone.
            </p>
          </div>

          <div style={{ marginBottom: '1.25rem' }}>
            <label
              htmlFor="confirm-phrase-input"
              style={{ display: 'block', fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: '0.4rem' }}
            >
              Type <span style={{ color: 'var(--destructive)', fontFamily: 'monospace', fontWeight: 700 }}>{REQUIRED_CONFIRM_PHRASE}</span> to confirm:
            </label>
            <Input
              id="confirm-phrase-input"
              type="text"
              value={confirmPhrase}
              onChange={(e) => {
                setConfirmPhrase(e.target.value);
                if (confirmPhraseError) setConfirmPhraseError('');
              }}
              placeholder={REQUIRED_CONFIRM_PHRASE}
              error={confirmPhraseError}
              autoComplete="off"
              disabled={isExecuting}
            />
            {confirmPhraseError && (
              <p style={{ color: 'var(--destructive)', fontSize: 'var(--text-xs)', marginTop: '0.35rem', margin: 0 }}>
                {confirmPhraseError}
              </p>
            )}
          </div>

          {/* Actions */}
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.75rem', marginTop: '1.5rem' }}>
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setIsConfirmModalOpen(false);
                setConfirmPhrase('');
                setConfirmPhraseError('');
              }}
              disabled={isExecuting}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="danger"
              onClick={handleConfirmAndExecute}
              disabled={!isConfirmationPhraseValid || isExecuting || !hasPermission}
              isLoading={isExecuting}
            >
              Execute Replay
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
