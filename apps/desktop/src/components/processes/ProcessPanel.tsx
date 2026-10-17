import { useState, useMemo } from 'react';
import type { ProcessItem } from '../../types';
import { terminateProcess } from '../../api/ipc';
import { useI18n } from '../../i18n';
import './ProcessPanel.css';

interface ProcessPanelProps {
  processes: ProcessItem[];
  loading: boolean;
  refetch: () => void;
}

type CategoryTab = 'All' | 'Application' | 'BrowserEngine' | 'BackgroundService' | 'System';

function canonicalCategory(cat: string): 'Application' | 'BrowserEngine' | 'BackgroundService' | 'System' {
  const lower = cat.toLowerCase();
  if (lower.includes('browser')) return 'BrowserEngine';
  if (lower.includes('service') || lower.includes('background')) return 'BackgroundService';
  if (lower.includes('system')) return 'System';
  return 'Application';
}

function fmtMB(bytes: number): string {
  return (bytes / 1_048_576).toFixed(1);
}

export function ProcessPanel({ processes, loading, refetch }: ProcessPanelProps) {
  const { t } = useI18n();
  const [search, setSearch] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<CategoryTab>('All');
  const [sortBy, setSortBy] = useState<'cpu' | 'ram'>('cpu');

  // Process termination modal & execution state
  const [targetToTerminate, setTargetToTerminate] = useState<ProcessItem | null>(null);
  const [terminating, setTerminating] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);

  // Compute category counts
  const categoryCounts = useMemo(() => {
    const counts: Record<CategoryTab, number> = {
      All: processes.length,
      Application: 0,
      BrowserEngine: 0,
      BackgroundService: 0,
      System: 0,
    };
    for (const p of processes) {
      const cat = canonicalCategory(p.category);
      counts[cat]++;
    }
    return counts;
  }, [processes]);

  // Filtered and sorted processes
  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return processes
      .filter((p) => {
        // Category filter
        if (selectedCategory !== 'All' && canonicalCategory(p.category) !== selectedCategory) {
          return false;
        }
        // Search filter (name or PID)
        if (q && !p.name.toLowerCase().includes(q) && !String(p.pid).includes(q)) {
          return false;
        }
        return true;
      })
      .sort((a, b) =>
        sortBy === 'cpu'
          ? b.cpu_percent - a.cpu_percent || b.memory_bytes - a.memory_bytes
          : b.memory_bytes - a.memory_bytes || b.cpu_percent - a.cpu_percent
      );
  }, [processes, selectedCategory, search, sortBy]);

  const handleConfirmTerminate = async () => {
    if (!targetToTerminate) return;
    setTerminating(true);
    setActionError(null);
    setActionSuccess(null);

    try {
      const res = await terminateProcess(targetToTerminate.pid, targetToTerminate.name);
      if (res.success) {
        setActionSuccess(res.message);
        setTargetToTerminate(null);
        refetch();
      } else {
        setActionError(res.message);
      }
    } catch (err: unknown) {
      let msg = String(err);
      try {
        const parsed = JSON.parse(String(err));
        if (parsed.error_type === 'ProtectedSystemProcess' && parsed.details?.reason) {
          msg = `${t.proc_error_protected}: ${parsed.details.reason}`;
        }
      } catch {
        // Use raw message
      }
      setActionError(msg);
    } finally {
      setTerminating(false);
    }
  };

  const getCategoryLabel = (cat: 'Application' | 'BrowserEngine' | 'BackgroundService' | 'System') => {
    switch (cat) {
      case 'Application':
        return t.proc_cat_application;
      case 'BrowserEngine':
        return t.proc_cat_browser;
      case 'BackgroundService':
        return t.proc_cat_service;
      case 'System':
        return t.proc_cat_system;
    }
  };

  const getCategoryBadgeClass = (cat: 'Application' | 'BrowserEngine' | 'BackgroundService' | 'System') => {
    switch (cat) {
      case 'Application':
        return 'app';
      case 'BrowserEngine':
        return 'browser';
      case 'BackgroundService':
        return 'service';
      case 'System':
        return 'system';
    }
  };

  const categoryTabs: CategoryTab[] = ['All', 'Application', 'BrowserEngine', 'BackgroundService', 'System'];

  return (
    <div className="process-panel">
      {/* Top Header */}
      <div className="process-header-row">
        <h1 className="page-title">{t.process_title}</h1>
        <button
          className="refresh-btn"
          onClick={refetch}
          disabled={loading}
          aria-label={t.proc_refresh_btn}
        >
          {loading ? '⏳' : '🔄'} <span>{t.proc_refresh_btn}</span>
        </button>
      </div>

      {/* Category Filtering Tabs with Count Badges */}
      <div className="category-tabs-row" role="tablist">
        {categoryTabs.map((cat) => {
          const count = categoryCounts[cat];
          const label = cat === 'All' ? t.proc_cat_all : getCategoryLabel(cat);
          return (
            <button
              key={cat}
              className={`cat-tab-btn ${selectedCategory === cat ? 'active' : ''}`}
              onClick={() => setSelectedCategory(cat)}
              role="tab"
              aria-selected={selectedCategory === cat}
            >
              <span>{label}</span>
              <span className="cat-count-badge">{count}</span>
            </button>
          );
        })}
      </div>

      {/* Search and Sort Controls */}
      <div className="process-controls">
        <input
          className="search-input"
          type="search"
          placeholder={t.process_search}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
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

      {/* Action Notification Banners */}
      {actionError && (
        <div className="process-alert-banner error">
          <span>⚠️ {actionError}</span>
          <button className="alert-dismiss-btn" onClick={() => setActionError(null)}>✕</button>
        </div>
      )}
      {actionSuccess && (
        <div className="process-alert-banner success">
          <span>✅ {actionSuccess}</span>
          <button className="alert-dismiss-btn" onClick={() => setActionSuccess(null)}>✕</button>
        </div>
      )}

      {/* Process Count */}
      <p className="proc-count-label">
        {filtered.length} {t.proc_count}
      </p>

      {/* Process Table */}
      <div className="process-table-wrap">
        <table className="process-table" aria-label="Process list">
          <thead>
            <tr>
              <th>{t.proc_pid}</th>
              <th>{t.proc_name}</th>
              <th>{t.proc_category}</th>
              <th>{t.proc_cpu}</th>
              <th>{t.proc_ram}</th>
              <th>{t.proc_protection}</th>
              <th className="term-action-cell">{t.proc_action}</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((p) => {
              const cat = canonicalCategory(p.category);
              return (
                <tr key={p.pid} className={p.cpu_percent >= 80 ? 'row-hot' : ''}>
                  <td className="pid-cell">{p.pid}</td>
                  <td className="name-cell" title={p.name}>
                    {p.name}
                  </td>
                  <td>
                    <span className={`category-badge ${getCategoryBadgeClass(cat)}`}>
                      {getCategoryLabel(cat)}
                    </span>
                  </td>
                  <td className="num-cell">
                    <span
                      className={`cpu-badge ${
                        p.cpu_percent >= 80 ? 'hot' : p.cpu_percent >= 40 ? 'warm' : ''
                      }`}
                    >
                      {p.cpu_percent.toFixed(1)}%
                    </span>
                  </td>
                  <td className="num-cell">{fmtMB(p.memory_bytes)} MB</td>
                  <td>
                    {p.is_protected ? (
                      <span className="prot-badge protected" title={t.proc_protected_tooltip}>
                        🛡️ {t.proc_status_protected}
                      </span>
                    ) : (
                      <span className="prot-badge unprotected">
                        {t.proc_status_unprotected}
                      </span>
                    )}
                  </td>
                  <td className="term-action-cell">
                    {p.is_protected ? (
                      <button
                        className="term-btn disabled"
                        disabled
                        title={t.proc_protected_tooltip}
                      >
                        {t.proc_btn_terminate}
                      </button>
                    ) : (
                      <button
                        className="term-btn"
                        onClick={() => {
                          setActionError(null);
                          setTargetToTerminate(p);
                        }}
                      >
                        {t.proc_btn_terminate}
                      </button>
                    )}
                  </td>
                </tr>
              );
            })}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={7} className="empty-row">
                  {search ? `${t.proc_no_match} "${search}"` : t.proc_no_data}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Confirmation Modal */}
      {targetToTerminate && (
        <div className="modal-overlay" onClick={() => !terminating && setTargetToTerminate(null)}>
          <div className="process-modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="process-modal-head">
              <span style={{ fontSize: '24px' }}>⚠️</span>
              <h3 className="process-modal-title">{t.proc_confirm_title}</h3>
            </div>

            <p className="process-modal-desc">
              {t.proc_confirm_desc.replace('{name}', targetToTerminate.name).replace('{pid}', String(targetToTerminate.pid))}
            </p>

            <div className="process-target-preview">
              <div className="preview-row">
                <span>{t.proc_name}:</span>
                <strong>{targetToTerminate.name}</strong>
              </div>
              <div className="preview-row">
                <span>{t.proc_pid}:</span>
                <strong>{targetToTerminate.pid}</strong>
              </div>
              <div className="preview-row">
                <span>{t.proc_category}:</span>
                <span>{getCategoryLabel(canonicalCategory(targetToTerminate.category))}</span>
              </div>
              <div className="preview-row">
                <span>{t.proc_cpu}:</span>
                <span>{targetToTerminate.cpu_percent.toFixed(1)}%</span>
              </div>
              <div className="preview-row">
                <span>{t.proc_ram}:</span>
                <span>{fmtMB(targetToTerminate.memory_bytes)} MB</span>
              </div>
            </div>

            <div className="modal-actions">
              <button
                className="modal-btn-cancel"
                onClick={() => setTargetToTerminate(null)}
                disabled={terminating}
              >
                {t.proc_btn_cancel}
              </button>
              <button
                className="modal-btn-terminate"
                onClick={handleConfirmTerminate}
                disabled={terminating}
              >
                {terminating ? t.proc_terminating : t.proc_btn_confirm_terminate}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
