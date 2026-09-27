'use client';

import { useEffect, useState } from 'react';
import { api } from '@/lib/api/admin-client';
import './audit.css';

type Log = { id?: string | number; actor?: string; action?: string; resource_type?: string; timestamp?: string; status?: string };

export default function AuditLogPage() {
  const [filters, setFilters] = useState({ actor: '', action: '', from: '', to: '' });
  const [page, setPage] = useState(0);
  const [logs, setLogs] = useState<Log[]>([]);
  const [error, setError] = useState('');
  const limit = 50;

  useEffect(() => {
    let active = true;
    const query = { ...filters, from: filters.from ? new Date(filters.from).toISOString() : undefined, to: filters.to ? new Date(filters.to).toISOString() : undefined, limit, offset: page * limit };
    api.getAuditLogs(query)
      .then(data => { if (active) setLogs(Array.isArray(data) ? data as Log[] : ((data as { logs?: Log[] }).logs ?? [])); })
      .catch(() => active && setError('Unable to load audit logs.'));
    return () => { active = false; };
  }, [filters, page]);

  const update = (key: keyof typeof filters, value: string) => { setPage(0); setFilters(f => ({ ...f, [key]: value })); };
  return (
    <div className="audit-log-page">
      <div className="admin-page-header">
        <div>
          <h1 className="admin-page-title">Audit log</h1>
          <p className="admin-page-desc">Administrative actions taken across the platform, with actor/action/date filtering.</p>
        </div>
      </div>

      <form onSubmit={e => e.preventDefault()} className="admin-card audit-log-filters">
        {(['actor', 'action'] as const).map(key => (
          <label key={key} className="audit-log-filters__field">
            {key}
            <input value={filters[key]} onChange={e => update(key, e.target.value)} />
          </label>
        ))}
        <label className="audit-log-filters__field">
          From
          <input type="datetime-local" value={filters.from} onChange={e => update('from', e.target.value)} />
        </label>
        <label className="audit-log-filters__field">
          To
          <input type="datetime-local" value={filters.to} onChange={e => update('to', e.target.value)} />
        </label>
      </form>

      {error && (
        <p role="alert" className="audit-log-error">
          {error}
        </p>
      )}

      <div className="admin-table-container">
        <table className="admin-table">
          <caption className="audit-log-caption">Administrative actions</caption>
          <thead>
            <tr>
              <th>Time</th>
              <th>Actor</th>
              <th>Action</th>
              <th>Resource</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {logs.map((log, i) => (
              <tr key={log.id ?? i}>
                <td>{log.timestamp ? new Date(log.timestamp).toLocaleString() : '—'}</td>
                <td>{log.actor ?? '—'}</td>
                <td>{log.action ?? '—'}</td>
                <td>{log.resource_type ?? '—'}</td>
                <td>{log.status ?? '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <nav aria-label="Audit log pages" className="audit-log-pagination">
        <button type="button" disabled={page === 0} onClick={() => setPage(p => p - 1)}>
          Previous
        </button>
        <span>Page {page + 1}</span>
        <button type="button" disabled={logs.length < limit} onClick={() => setPage(p => p + 1)}>
          Next
        </button>
      </nav>
    </div>
  );
}
