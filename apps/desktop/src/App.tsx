import { useState, useEffect } from 'react';
import { I18nProvider, useI18n, type Language } from './i18n';
import { useMonitoring } from './hooks/useMonitoring';
import { Companion } from './components/companion/Companion';
import { Dashboard } from './components/dashboard/Dashboard';
import { ChatPanel } from './components/chat/ChatPanel';
import { ProcessPanel } from './components/processes/ProcessPanel';
import { StorageCleanerPanel } from './components/storage/StorageCleanerPanel';
import { SettingsPanel } from './components/settings/SettingsPanel';
import type { SystemSnapshot } from './types';
import './styles/theme.css';
import './App.css';

type NavPage = 'dashboard' | 'chat' | 'processes' | 'security' | 'storage' | 'settings';

function AppInner() {
  const { t, lang, setLang } = useI18n();
  const [page, setPage] = useState<NavPage>('dashboard');
  const [pollingMs, setPollingMs] = useState<number>(() => {
    const saved = localStorage.getItem('pc_doctor_polling');
    return saved ? parseInt(saved, 10) : 2000;
  });

  const { snapshot, processes, loading, error, mood, refetch } = useMonitoring(pollingMs);

  // Default to light theme matching kernova.click website reference screenshot
  useEffect(() => {
    const saved = localStorage.getItem('pc_doctor_theme') || 'light';
    document.documentElement.setAttribute('data-theme', saved);
  }, []);

  const toggleTheme = () => {
    const current = document.documentElement.getAttribute('data-theme') || 'light';
    const next = current === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    localStorage.setItem('pc_doctor_theme', next);
  };

  const toggleLang = () => {
    const nextLang: Language = lang === 'vi' ? 'en' : 'vi';
    setLang(nextLang);
  };

  const currentTheme = document.documentElement.getAttribute('data-theme') || 'light';

  return (
    <div className="app-shell">
      {/* Top Header Bar matching website navbar */}
      <header className="app-header">
        <div className="header-brand">
          <svg className="brand-stethoscope-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M4.8 2.3A.3.3 0 1 0 5 2H4a2 2 0 0 0-2 2v5a6 6 0 0 0 6 6v0a6 6 0 0 0 6-6V4a2 2 0 0 0-2-2h-1a.2.2 0 1 0 .3.3"/>
            <path d="M8 15v1a6 6 0 0 0 6 6v0a6 6 0 0 0 6-6v-4"/>
            <circle cx="20" cy="10" r="2"/>
          </svg>
          <span className="brand-title">TBK Core.</span>
        </div>

        {/* Website-aligned Navigation items */}
        <nav className="header-nav" aria-label="Main Navigation">
          <button
            className={`nav-link ${page === 'dashboard' ? 'active' : ''}`}
            onClick={() => setPage('dashboard')}
          >
            {t.nav_dashboard}
          </button>
          <button
            className={`nav-link ${page === 'chat' ? 'active' : ''}`}
            onClick={() => setPage('chat')}
          >
            {t.nav_chat}
          </button>
          <button
            className={`nav-link ${page === 'processes' ? 'active' : ''}`}
            onClick={() => setPage('processes')}
          >
            {t.nav_processes}
          </button>
          <button
            className={`nav-link ${page === 'security' ? 'active' : ''}`}
            onClick={() => setPage('security')}
          >
            {t.nav_security}
          </button>
          <button
            className={`nav-link ${page === 'storage' ? 'active' : ''}`}
            onClick={() => setPage('storage')}
          >
            {t.nav_storage}
          </button>
          <button
            className={`nav-link ${page === 'settings' ? 'active' : ''}`}
            onClick={() => setPage('settings')}
          >
            {t.nav_settings}
          </button>
        </nav>

        {/* Header Right Actions: Lang switch, Theme toggle, Status Pill */}
        <div className="header-actions">
          <button className="lang-toggle-btn" onClick={toggleLang} title="Switch language">
            {lang === 'vi' ? 'VI / EN' : 'EN / VI'}
          </button>
          <button className="theme-toggle-btn" onClick={toggleTheme} title="Toggle Dark/Light">
            {currentTheme === 'dark' ? '☀️' : '🌙'}
          </button>
          <div className="live-status-pill">
            <span className="live-status-dot" />
            <span className="live-status-text">
              {loading ? t.status_reading : t.status_optimal}
            </span>
          </div>
        </div>
      </header>

      {/* Main Body with content and right-side Companion Widget */}
      <div className="app-body">
        <main className="main-viewport">
          {page === 'dashboard' && (
            <Dashboard snapshot={snapshot} loading={loading} error={error} onNavigate={(p) => setPage(p as NavPage)} />
          )}
          {page === 'chat' && (
            <ChatPanel snapshot={snapshot} />
          )}
          {page === 'processes' && (
            <ProcessPanel processes={processes} loading={loading} refetch={refetch} />
          )}
          {page === 'security' && (
            <SecurityWatchSection snapshot={snapshot} />
          )}
          {page === 'storage' && (
            <StorageCleanerPanel snapshot={snapshot} />
          )}
          {page === 'settings' && (
            <SettingsPanel pollingMs={pollingMs} setPollingMs={setPollingMs} />
          )}
        </main>

        {/* Right side companion widget dock */}
        <aside className="companion-dock">
          <div className="dock-card">
            <Companion mood={mood} label={t[`mood_${mood}` as keyof typeof t] as string} />
            <div className="dock-telemetry">
              {snapshot && (
                <>
                  <div className="dock-stat">
                    <span className="dock-stat-label">CPU:</span>
                    <span className="dock-stat-val">{snapshot.cpu.global_usage_percent.toFixed(0)}%</span>
                  </div>
                  <div className="dock-stat">
                    <span className="dock-stat-label">RAM:</span>
                    <span className="dock-stat-val">{snapshot.memory.usage_percent.toFixed(0)}%</span>
                  </div>
                </>
              )}
            </div>
            <button className="dock-action-btn" onClick={() => setPage('chat')}>
              💬 {t.nav_chat}
            </button>
          </div>
        </aside>
      </div>
    </div>
  );
}

// In-app Security Watch component
function SecurityWatchSection({ snapshot }: { snapshot: SystemSnapshot | null }) {
  const { t } = useI18n();
  const highCpuProcs = snapshot?.findings.filter((f) => f.rule_id === 'PERF_UNUSUAL_CPU_PROCESS') || [];

  return (
    <div className="security-section card">
      <h1 className="page-title">{t.sec_title}</h1>
      <div className="security-banner">
        <span className="sec-icon">🛡️</span>
        <div>
          <h3>{t.sec_status}: {t.sec_safe}</h3>
          <p>{t.sec_verified_note}</p>
        </div>
      </div>

      <div className="sec-grid">
        <div className="sec-item card">
          <div className="sec-item-head">
            <span className="sec-check">✅</span>
            <strong>{t.sec_firewall}</strong>
          </div>
          <p>{t.sec_firewall_desc}</p>
        </div>

        <div className="sec-item card">
          <div className="sec-item-head">
            <span className="sec-check">✅</span>
            <strong>{t.sec_antivirus}</strong>
          </div>
          <p>{t.sec_check_clean}</p>
        </div>
      </div>

      {highCpuProcs.length > 0 && (
        <div className="sec-alert-box">
          <h4>⚠️ {t.sec_alert_procs}</h4>
          <ul>
            {highCpuProcs.map((p, idx: number) => (
              <li key={idx}><strong>{p.title}</strong>: {p.description}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

export default function App() {
  return (
    <I18nProvider>
      <AppInner />
    </I18nProvider>
  );
}
