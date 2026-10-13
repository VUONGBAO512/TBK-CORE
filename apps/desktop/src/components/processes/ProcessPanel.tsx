import { useState, useMemo } from 'react';
import type { ProcessInfo } from '../../types';
import { useI18n } from '../../i18n';
import './ProcessPanel.css';

interface ProcessPanelProps {
  processes: ProcessInfo[];
  loading: boolean;
  refetch: () => void;
}

function fmtMB(bytes: number): string {
  return (bytes / 1_048_576).toFixed(1);
}

export function ProcessPanel({ processes, loading, refetch }: ProcessPanelProps) {
  const { t } = useI18n();
  const [search, setSearch] = useState('');
  const [sortBy, setSortBy] = useState<'cpu' | 'ram'>('cpu');

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return processes
      .filter(p => p.name.toLowerCase().includes(q) || String(p.pid).includes(q))
      .sort((a, b) => sortBy === 'cpu'
        ? b.cpu_percent - a.cpu_percent
        : b.memory_bytes - a.memory_bytes
      );
  }, [processes, search, sortBy]);

  return (
    <div className="process-panel">
      <div className="process-header-row">
        <h1 className="page-title">{t.process_title}</h1>
        <button className="refresh-btn" onClick={refetch} disabled={loading} aria-label="Refresh">
          {loading ? '⏳' : '🔄'}
        </button>
      </div>

      <div className="process-controls">
        <input
          className="search-input"
          type="search"
          placeholder={t.process_search}
          value={search}
          onChange={e => setSearch(e.target.value)}
          aria-label="Search processes"
        />
        <div className="sort-toggle" role="group" aria-label="Sort by">
          <button
            className={`sort-btn ${sortBy === 'cpu' ? 'active' : ''}`}
            onClick={() => setSortBy('cpu')}
          >
            CPU ↓
          </button>
          <button
            className={`sort-btn ${sortBy === 'ram' ? 'active' : ''}`}
            onClick={() => setSortBy('ram')}
          >
            RAM ↓
          </button>
        </div>
      </div>

      <p className="proc-count-label">
        {filtered.length} {t.proc_count}
      </p>

      <div className="process-table-wrap">
        <table className="process-table" aria-label="Process list">
          <thead>
            <tr>
              <th>{t.proc_pid}</th>
              <th>{t.proc_name}</th>
              <th>{t.proc_cpu}</th>
              <th>{t.proc_ram}</th>
              <th>{t.proc_status}</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map(p => (
              <tr key={p.pid} className={p.cpu_percent >= 80 ? 'row-hot' : ''}>
                <td className="pid-cell">{p.pid}</td>
                <td className="name-cell" title={p.name}>{p.name}</td>
                <td className="num-cell">
                  <span className={`cpu-badge ${p.cpu_percent >= 80 ? 'hot' : p.cpu_percent >= 40 ? 'warm' : ''}`}>
                    {p.cpu_percent.toFixed(1)}%
                  </span>
                </td>
                <td className="num-cell">{fmtMB(p.memory_bytes)} MB</td>
                <td className="status-cell">{p.status}</td>
              </tr>
            ))}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={5} className="empty-row">
                  {search ? `No processes matching "${search}"` : 'No process data'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
