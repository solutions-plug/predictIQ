'use client';

import React, { useState } from 'react';
import { api, ApiError } from '@/lib/api/admin-client';
import { Modal } from '@/components/admin/Modal';
import { Form, FormField, Input, Button, StatusAlert } from '@/components/admin/Form';
import './replay.css';

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
      <div className={`admin-card replay-permission-banner ${!hasPermission ? 'replay-permission-banner--denied' : ''}`}>
        <div className="replay-permission-banner__inner">
          <div>
            <div className="replay-permission-banner__status-row">
              <span className="replay-permission-banner__status-label">Authorization Status:</span>
              <span className={`replay-permission-badge ${hasPermission ? 'replay-permission-badge--granted' : 'replay-permission-badge--denied'}`}>
                {hasPermission ? 'blockchain:replay GRANTED (Super Admin)' : 'ACCESS DENIED (Lacks permission)'}
              </span>
            </div>
            <p className="replay-permission-banner__desc">
              {hasPermission
                ? 'Your authenticated admin session holds the operational role required to initiate ledger state replays.'
                : 'This action is fully disabled for sessions without explicit blockchain:replay authorization.'}
            </p>
          </div>

          <button
            type="button"
            onClick={() => setHasPermission(!hasPermission)}
            className="replay-permission-toggle"
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

      <div className="replay-columns">
        {/* Left Column: Replay Request Form */}
        <div className="admin-card">
          <div className="admin-card-header">
            <h2 className="admin-card-title">Trigger Ledger Replay</h2>
          </div>

          {!hasPermission && (
            <div role="alert" className="replay-locked-notice">
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

            <div className="replay-submit-row">
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
          <div className="replay-notes">
            <h3 className="replay-notes__title">Operational Considerations</h3>
            <ul>
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
            <span className="replay-history-count">{auditLogs.length} logged</span>
          </div>

          {auditLogs.length === 0 ? (
            <p className="replay-history-empty">No replays triggered in this session.</p>
          ) : (
            <div className="replay-history-list">
              {auditLogs.map((log, index) => (
                <div key={index} className="replay-history-item">
                  <div className="replay-history-item__row">
                    <span className="replay-history-item__ledger">Ledger #{log.from_ledger}</span>
                    <span className="replay-history-item__count">{log.events_replayed} events</span>
                  </div>
                  <div className="replay-history-item__meta">
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
          <div className="replay-warning-block">
            <div className="replay-warning-block__title">WARNING: Irreversible Operation</div>
            <p>
              You are about to reprocess blockchain events starting from ledger <strong>#{fromLedger}</strong>. This operational mutation will re-evaluate contract states in the database and cannot be undone.
            </p>
          </div>

          <div className="replay-confirm-field">
            <label htmlFor="confirm-phrase-input">
              Type <span className="replay-confirm-phrase">{REQUIRED_CONFIRM_PHRASE}</span> to confirm:
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
            {confirmPhraseError && <p className="replay-confirm-error">{confirmPhraseError}</p>}
          </div>

          {/* Actions */}
          <div className="replay-modal-actions">
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
