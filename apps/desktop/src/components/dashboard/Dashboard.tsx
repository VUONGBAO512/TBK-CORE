import type { SystemSnapshot, DiagnosticFinding, SeverityLevel } from '../../types';
import { useI18n } from '../../i18n';
import './Dashboard.css';

interface DashboardProps {
  snapshot: SystemSnapshot | null;
  loading: boolean;
  error: string | null;
  onNavigate?: (page: string) => void;
}

function fmtGB(bytes: number): string {
  return (bytes / 1_073_741_824).toFixed(1) + ' GB';
}
function fmtUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${h}h ${m}m`;
}
function progressClass(pct: number): string {
  if (pct >= 90) return 'critical';
  if (pct >= 75) return 'warning';
  return 'healthy';
}
function severityLabel(s: SeverityLevel, t: Record<string, string>): string {
  const map: Record<SeverityLevel, string> = {
    critical: t.severity_critical,
    high: t.severity_high,
    moderate: t.severity_moderate,
    low: t.severity_low,
    informational: t.severity_informational,
  };
  return map[s] ?? s;
}

function FindingCard({ f }: { f: DiagnosticFinding }) {
  const { t } = useI18n();
  return (
    <div className={`finding-card severity-${f.severity}`}>
      <div className="finding-header">
        <span className={`badge ${f.severity}`}>{severityLabel(f.severity, t as unknown as Record<string, string>)}</span>
        <span className="finding-title">{f.title}</span>
      </div>
      <p className="finding-desc">{f.description}</p>
      {f.evidence.length > 0 && (
        <div className="finding-section">
          <strong>{t.evidence}</strong>
          <ul>{f.evidence.map((e, i) => <li key={i}>{e}</li>)}</ul>
        </div>
      )}
      {f.next_steps.length > 0 && (
        <div className="finding-section">
          <strong>{t.recommendations}</strong>
          <ul>{f.next_steps.map((s, i) => <li key={i}>{s}</li>)}</ul>
        </div>
      )}
    </div>
  );
}

export function Dashboard({ snapshot, loading, error, onNavigate }: DashboardProps) {
  const { t } = useI18n();

  if (loading && !snapshot) {
    return (
      <div className="loading-state">
        <div className="spinner" />
        <span>{t.loading_metrics}</span>
      </div>
    );
  }
  if (error && !snapshot) {
    return <div className="error-state">⚠️ {error}</div>;
  }
  if (!snapshot) return null;

  const { cpu, memory, disks, os, temperatures, findings } = snapshot;
  const hasCritical = findings.some(f => f.severity === 'critical');
  const hasHigh = findings.some(f => f.severity === 'high');

  return (
    <div className="dashboard">
      {/* Hero Banner directly aligned with website screenshot */}
      <section className="website-hero-card card">
        <div className="hero-left">
          <div className="hero-pill-badge">
            <span className="hero-dot" />
            <span className="hero-pill-text">CHĂM SÓC SỨC KHỎE MÁY TÍNH</span>
          </div>

          <h1 className="hero-headline">{t.hero_title}</h1>
          <p className="hero-subtext">{t.hero_sub}</p>

          <div className="hero-cta-group">
            <button className="btn-brand-primary" onClick={() => onNavigate?.('chat')}>
              Khám phá tính năng ↗
            </button>
            <button className="btn-brand-outline" onClick={() => onNavigate?.('settings')}>
              Thông tin phiên bản
            </button>
          </div>

          <div className="hero-os-badge">
            <span className="windows-icon">🪟</span>
            <span>Đang hoạt động trên {os.name} ({os.os_version})</span>
          </div>
        </div>

        {/* Hero Right: Floating Simulation / Telemetry Orb card */}
        <div className="hero-right">
          <div className="floating-metric-card top">
            <div className="fmc-head">
              <span className="pulse-wave-icon">📈</span>
              <div>
                <span className="fmc-sub">Tình trạng đo lường</span>
                <strong className="fmc-main">{hasCritical ? 'Cần xử lý' : hasHigh ? 'Cần lưu ý' : 'Hệ thống ổn định'}</strong>
              </div>
              <span className={`fmc-status-dot ${hasCritical ? 'critical' : hasHigh ? 'warning' : 'healthy'}`} />
            </div>
          </div>

          {/* Real live CPU Widget directly mirroring screenshot mini card */}
          <div className="floating-metric-card bottom">
            <div className="fmc-row">
              <span className="cpu-chip-icon">🔲</span>
              <div className="fmc-cpu-meta">
                <span className="fmc-sub">{t.cpu_usage}</span>
                <span className="fmc-val">{cpu.global_usage_percent.toFixed(0)}%</span>
              </div>
              <div className="mini-sparkline" title="Biểu đồ tải">
                <svg width="80" height="28" viewBox="0 0 80 28" fill="none">
                  <path
                    d={`M2 24 L 15 ${24 - (cpu.global_usage_percent * 0.15)} L 35 ${20 - (cpu.global_usage_percent * 0.12)} L 55 ${22 - (cpu.global_usage_percent * 0.18)} L 78 ${24 - (cpu.global_usage_percent * 0.2)}`}
                    stroke="var(--accent-cyan)"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Metrics grid */}
      <div className="metrics-grid">
        {/* CPU Detailed Card */}
        <div className="card metric-card">
          <div className="metric-header">
            <span className="metric-icon">🖥️</span>
            <div>
              <div className="metric-title">{t.cpu_usage}</div>
              <div className="metric-sub">{cpu.model_name}</div>
            </div>
            <div className={`metric-value pct ${progressClass(cpu.global_usage_percent)}`}>
              {cpu.global_usage_percent.toFixed(1)}%
            </div>
          </div>
          <div className="progress-bar-track">
            <div
              className={`progress-bar-fill ${progressClass(cpu.global_usage_percent)}`}
              style={{ width: `${Math.min(cpu.global_usage_percent, 100)}%` }}
            />
          </div>
          <div className="metric-detail-row">
            <span>{t.cores}: {cpu.logical_cores} ({cpu.physical_cores}P)</span>
            <span>{t.mhz}: {cpu.cores[0]?.frequency_mhz ?? '—'} MHz</span>
          </div>
          {/* Per-core mini bars */}
          <div className="core-grid">
            {cpu.cores.slice(0, 16).map(c => (
              <div key={c.core_id} className="core-bar-wrap" title={`Core ${c.core_id}: ${c.usage_percent.toFixed(0)}%`}>
                <div
                  className={`core-bar ${progressClass(c.usage_percent)}`}
                  style={{ height: `${Math.max(2, c.usage_percent)}%` }}
                />
              </div>
            ))}
          </div>
        </div>

        {/* RAM Card */}
        <div className="card metric-card">
          <div className="metric-header">
            <span className="metric-icon">💾</span>
            <div>
              <div className="metric-title">{t.memory_usage}</div>
              <div className="metric-sub">{fmtGB(memory.used_bytes)} / {fmtGB(memory.total_bytes)}</div>
            </div>
            <div className={`metric-value pct ${progressClass(memory.usage_percent)}`}>
              {memory.usage_percent.toFixed(1)}%
            </div>
          </div>
          <div className="progress-bar-track">
            <div
              className={`progress-bar-fill ${progressClass(memory.usage_percent)}`}
              style={{ width: `${Math.min(memory.usage_percent, 100)}%` }}
            />
          </div>
          <div className="metric-detail-row">
            <span>{t.available}: {fmtGB(memory.available_bytes)}</span>
            {memory.swap_total_bytes > 0 && (
              <span>{t.swap}: {fmtGB(memory.swap_used_bytes)}/{fmtGB(memory.swap_total_bytes)}</span>
            )}
          </div>
        </div>

        {/* OS Info Card */}
        <div className="card metric-card">
          <div className="metric-header">
            <span className="metric-icon">🪟</span>
            <div>
              <div className="metric-title">{t.os_info}</div>
              <div className="metric-sub">{os.host_name}</div>
            </div>
          </div>
          <div className="os-detail">
            <div className="os-row"><span>{os.name}</span><span>{os.os_version}</span></div>
            <div className="os-row"><span>{t.uptime}</span><span>{fmtUptime(os.uptime_seconds)}</span></div>
            <div className="os-row"><span>Kernel</span><span>{os.kernel_version}</span></div>
          </div>
        </div>
      </div>

      {/* Disks */}
      <h2 className="section-title">{t.storage_usage}</h2>
      <div className="disks-grid">
        {disks.map((d, i) => (
          <div key={i} className="card disk-card">
            <div className="disk-header">
              <span className="disk-icon">{d.is_removable ? '💽' : '🗄️'}</span>
              <div>
                <div className="disk-mount">{d.mount_point}</div>
                <div className="disk-fs">{d.filesystem} · {d.name}</div>
              </div>
              <div className={`metric-value pct ${progressClass(d.usage_percent)}`}>
                {d.usage_percent.toFixed(1)}%
              </div>
            </div>
            <div className="progress-bar-track">
              <div
                className={`progress-bar-fill ${progressClass(d.usage_percent)}`}
                style={{ width: `${Math.min(d.usage_percent, 100)}%` }}
              />
            </div>
            <div className="metric-detail-row">
              <span>{t.used}: {fmtGB(d.used_bytes)}</span>
              <span>{t.available}: {fmtGB(d.available_bytes)}</span>
              <span>{t.total}: {fmtGB(d.total_bytes)}</span>
            </div>
          </div>
        ))}
      </div>

      {/* Temperatures */}
      <h2 className="section-title">{t.temperatures}</h2>
      <div className="card temp-section">
        {temperatures.filter(t2 => t2.celsius !== null).length === 0 ? (
          <p className="unavailable-note">ℹ️ {t.hardware_temps_unavailable}</p>
        ) : (
          <div className="temp-grid">
            {temperatures.filter(t2 => t2.celsius !== null).map((s, i) => (
              <div key={i} className="temp-chip">
                <span className="temp-label">{s.label}</span>
                <span className={`temp-val ${(s.celsius ?? 0) >= 80 ? 'critical' : (s.celsius ?? 0) >= 65 ? 'warning' : 'healthy'}`}>
                  {s.celsius?.toFixed(0)}°C
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Diagnostic findings */}
      <h2 className="section-title">{t.diagnostic_findings}</h2>
      {findings.length === 0 ? (
        <div className="card no-findings">
          <span>✅</span> {t.no_findings}
        </div>
      ) : (
        <div className="findings-list">
          {findings.map((f, i) => <FindingCard key={i} f={f} />)}
        </div>
      )}
    </div>
  );
}
