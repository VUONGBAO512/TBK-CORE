import { useState, useEffect, useCallback, useMemo } from 'react';
import type { SystemSnapshot, StorageScanReport, StorageCleanSummary } from '../../types';
import { scanCleanableStorage, cleanStorageTargets } from '../../api/ipc';
import { useI18n } from '../../i18n';
import './StorageCleanerPanel.css';

interface StorageCleanerPanelProps {
  snapshot: SystemSnapshot | null;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function getUsageStatus(pct: number): 'healthy' | 'warning' | 'critical' {
  if (pct >= 90) return 'critical';
  if (pct >= 75) return 'warning';
  return 'healthy';
}

export function StorageCleanerPanel({ snapshot }: StorageCleanerPanelProps) {
  const { t, lang } = useI18n();

  const [report, setReport] = useState<StorageScanReport | null>(null);
  const [scanning, setScanning] = useState<boolean>(false);
  const [scanError, setScanError] = useState<string | null>(null);

  const [selectedTargets, setSelectedTargets] = useState<Set<string>>(new Set());
  const [isDryRun, setIsDryRun] = useState<boolean>(false);

  const [cleaning, setCleaning] = useState<boolean>(false);
  const [cleanSummary, setCleanSummary] = useState<StorageCleanSummary | null>(null);
  const [cleanError, setCleanError] = useState<string | null>(null);
  const [showConfirmModal, setShowConfirmModal] = useState<boolean>(false);

  const handleScan = useCallback(async () => {
    setScanning(true);
    setScanError(null);
    try {
      const rep = await scanCleanableStorage();
      setReport(rep);
      // Automatically select all discovered targets
      const allIds = new Set(rep.targets.map((tgt) => tgt.id));
      setSelectedTargets(allIds);
    } catch (err: unknown) {
      console.error('Storage scan failed:', err);
      setScanError(String(err));
    } finally {
      setScanning(false);
    }
  }, []);

  // Run initial scan on component mount
  useEffect(() => {
    handleScan();
  }, [handleScan]);

  const toggleTarget = (id: string) => {
    setSelectedTargets((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleSelectAll = () => {
    if (!report) return;
    if (selectedTargets.size === report.targets.length) {
      setSelectedTargets(new Set());
    } else {
      setSelectedTargets(new Set(report.targets.map((tgt) => tgt.id)));
    }
  };

  const executeClean = async () => {
    if (selectedTargets.size === 0) return;
    setShowConfirmModal(false);
    setCleaning(true);
    setCleanError(null);

    try {
      const summary = await cleanStorageTargets({
        target_ids: Array.from(selectedTargets),
        dry_run: isDryRun,
      });
      setCleanSummary(summary);

      // Re-scan after live clean to update remaining target sizes
      if (!isDryRun) {
        const updatedRep = await scanCleanableStorage();
        setReport(updatedRep);
      }
    } catch (err: unknown) {
      console.error('Storage cleanup failed:', err);
      setCleanError(String(err));
    } finally {
      setCleaning(false);
    }
  };

  const totalSelectedBytes = useMemo(() => {
    if (!report) return 0;
    return report.targets
      .filter((t) => selectedTargets.has(t.id))
      .reduce((acc, t) => acc + t.total_bytes, 0);
  }, [report, selectedTargets]);

  const totalSelectedFiles = useMemo(() => {
    if (!report) return 0;
    return report.targets
      .filter((t) => selectedTargets.has(t.id))
      .reduce((acc, t) => acc + t.file_count, 0);
  }, [report, selectedTargets]);

  const isAllSelected = report ? report.targets.length > 0 && selectedTargets.size === report.targets.length : false;

  const disks = snapshot?.disks || [];

  return (
    <div className="storage-cleaner-panel">
      {/* Header Row */}
      <div className="storage-header-row">
        <h1 className="page-title">{t.storage_cleaner_title}</h1>
        <button
          className="storage-scan-btn"
          onClick={handleScan}
          disabled={scanning || cleaning}
          aria-label={t.storage_scan_btn}
        >
          {scanning ? (
            <>
              <span className="spinner-icon">⏳</span>
              <span>{t.storage_scanning}</span>
            </>
          ) : (
            <>
              <span>🔍</span>
              <span>{t.storage_scan_btn}</span>
            </>
          )}
        </button>
      </div>

      {/* Disk Summary Cards */}
      <div className="disks-summary-section">
        <h2 className="section-subtitle">{t.storage_disk_overview}</h2>
        <div className="disks-grid">
          {disks.map((d, idx) => {
            const status = getUsageStatus(d.usage_percent);
            return (
              <div key={idx} className="disk-card">
                <div className="disk-card-head">
                  <span className="disk-mount-badge">{d.mount_point}</span>
                  <span className="disk-label" title={d.name || d.mount_point}>
                    {d.name || t.storage_local_drive} ({d.filesystem})
                  </span>
                  <span className="disk-pct-badge">{d.usage_percent.toFixed(1)}%</span>
                </div>
                <div className="disk-progress-track">
                  <div
                    className={`disk-progress-fill ${status}`}
                    style={{ width: `${Math.min(d.usage_percent, 100)}%` }}
                  />
                </div>
                <div className="disk-metrics-row">
                  <span>{t.used}: {formatBytes(d.used_bytes)}</span>
                  <span>{t.available}: {formatBytes(d.available_bytes)}</span>
                  <span>{t.total}: {formatBytes(d.total_bytes)}</span>
                </div>
              </div>
            );
          })}
          {disks.length === 0 && (
            <div className="empty-storage-state">{t.storage_no_disks}</div>
          )}
        </div>
      </div>

      {/* Post-Clean Results Banner */}
      {cleanSummary && (
        <div className="results-banner">
          <div className="results-banner-head">
            <div className="results-banner-title">
              <span>{cleanSummary.dry_run ? '🔍' : '🧹'}</span>
              <span>{t.storage_results_title}</span>
              <span className={`results-mode-badge ${cleanSummary.dry_run ? 'dryrun' : ''}`}>
                {cleanSummary.dry_run ? t.storage_mode_dryrun : t.storage_mode_live}
              </span>
            </div>
            <button className="results-dismiss-btn" onClick={() => setCleanSummary(null)}>
              ✕ {t.storage_dismiss}
            </button>
          </div>

          <div className="results-stats-grid">
            <div className="results-stat-box">
              <span className="results-stat-label">
                {cleanSummary.dry_run ? t.storage_simulated_freed : t.storage_bytes_freed}
              </span>
              <span className="results-stat-val">{formatBytes(cleanSummary.bytes_freed)}</span>
            </div>
            <div className="results-stat-box">
              <span className="results-stat-label">{t.storage_files_deleted}</span>
              <span className="results-stat-val">{cleanSummary.files_deleted}</span>
            </div>
            <div className="results-stat-box">
              <span className="results-stat-label">{t.storage_files_skipped}</span>
              <span className="results-stat-val">{cleanSummary.files_skipped_locked}</span>
            </div>
          </div>

          {cleanSummary.errors.length > 0 && (
            <div className="results-errors-box">
              <strong>{t.storage_errors_title}:</strong>
              <ul>
                {cleanSummary.errors.map((err, i) => (
                  <li key={i}>{err}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* Scan Error Alert */}
      {scanError && (
        <div className="results-errors-box">
          ⚠️ {t.storage_scan_error}: {scanError}
        </div>
      )}

      {/* Clean Error Alert */}
      {cleanError && (
        <div className="results-errors-box">
          ⚠️ {t.storage_clean_error}: {cleanError}
        </div>
      )}

      {/* Reclaimable Storage Summary & Actions Bar */}
      {report && (
        <div className="reclaimable-summary-banner">
          <div className="reclaimable-info">
            <span className="reclaimable-heading">{t.storage_reclaimable_total}</span>
            <div className="reclaimable-value-row">
              <span className="reclaimable-size">{formatBytes(report.total_reclaimable_bytes)}</span>
              <span className="reclaimable-count">
                ({report.total_files} {t.storage_files})
              </span>
            </div>
          </div>

          <div className="storage-options-bar">
            <label className="dry-run-toggle-label" title={t.storage_dry_run_tooltip}>
              <input
                type="checkbox"
                className="dry-run-checkbox"
                checked={isDryRun}
                onChange={(e) => setIsDryRun(e.target.checked)}
              />
              <span>{t.storage_dry_run_toggle}</span>
            </label>

            <button
              className={`storage-clean-btn ${isDryRun ? 'dry-run' : ''}`}
              onClick={() => setShowConfirmModal(true)}
              disabled={selectedTargets.size === 0 || cleaning || scanning}
            >
              {cleaning
                ? t.storage_cleaning
                : isDryRun
                ? t.storage_btn_preview
                : t.storage_btn_clean}
            </button>
          </div>
        </div>
      )}

      {/* Targets List */}
      <div className="targets-section">
        <div className="targets-section-head">
          <h2 className="section-subtitle">{t.storage_targets_title}</h2>
          {report && report.targets.length > 0 && (
            <label className="select-all-label">
              <input
                type="checkbox"
                className="target-checkbox"
                checked={isAllSelected}
                onChange={handleSelectAll}
              />
              <span>{isAllSelected ? t.storage_deselect_all : t.storage_select_all}</span>
            </label>
          )}
        </div>

        {report && report.targets.length > 0 ? (
          <div className="targets-list">
            {report.targets.map((target) => {
              const isChecked = selectedTargets.has(target.id);
              const title = lang === 'vi' ? target.name_vi : target.name_en;
              const desc = lang === 'vi' ? target.description_vi : target.description_en;

              return (
                <div
                  key={target.id}
                  className={`target-card ${isChecked ? 'selected' : ''}`}
                  onClick={() => toggleTarget(target.id)}
                >
                  <div className="target-card-left">
                    <input
                      type="checkbox"
                      className="target-checkbox"
                      checked={isChecked}
                      onChange={() => {}} // Controlled by card onClick
                      onClick={(e) => e.stopPropagation()}
                    />
                    <div className="target-details">
                      <div className="target-name-row">
                        <span className="target-name">{title}</span>
                        {target.is_safe && (
                          <span className="safe-badge">
                            🛡️ {t.storage_badge_safe}
                          </span>
                        )}
                      </div>
                      <span className="target-desc">{desc}</span>
                      <span className="target-path">{target.root_path}</span>
                    </div>
                  </div>
                  <div className="target-card-right">
                    <span className="target-size">{formatBytes(target.total_bytes)}</span>
                    <span className="target-files">
                      {target.file_count} {t.storage_files}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          !scanning && (
            <div className="empty-storage-state">
              {t.storage_no_targets}
            </div>
          )
        )}
      </div>

      {/* Confirmation Modal */}
      {showConfirmModal && (
        <div className="modal-overlay" onClick={() => setShowConfirmModal(false)}>
          <div className="confirm-modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <span className="modal-icon">{isDryRun ? '🔍' : '⚠️'}</span>
              <h3 className="modal-title">
                {isDryRun ? t.storage_confirm_dryrun_title : t.storage_confirm_purge_title}
              </h3>
            </div>

            <p className="modal-desc">
              {isDryRun ? t.storage_confirm_dryrun_desc : t.storage_confirm_purge_desc}
            </p>

            <div className="modal-highlight-box">
              <div className="modal-highlight-row">
                <span>{t.storage_modal_selected_targets}:</span>
                <strong>{selectedTargets.size}</strong>
              </div>
              <div className="modal-highlight-row">
                <span>{t.storage_modal_selected_bytes}:</span>
                <strong>{formatBytes(totalSelectedBytes)}</strong>
              </div>
              <div className="modal-highlight-row">
                <span>{t.storage_modal_selected_files}:</span>
                <strong>{totalSelectedFiles}</strong>
              </div>
            </div>

            <div className="modal-actions">
              <button
                className="modal-btn-cancel"
                onClick={() => setShowConfirmModal(false)}
              >
                {t.storage_btn_cancel}
              </button>
              <button
                className={`modal-btn-confirm ${isDryRun ? 'dry-run' : ''}`}
                onClick={executeClean}
              >
                {isDryRun ? t.storage_btn_preview : t.storage_btn_confirm}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
