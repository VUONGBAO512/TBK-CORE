import { useI18n, type Language } from '../../i18n';
import './SettingsPanel.css';

interface SettingsPanelProps {
  pollingMs: number;
  setPollingMs: (ms: number) => void;
}

export function SettingsPanel({ pollingMs, setPollingMs }: SettingsPanelProps) {
  const { t, lang, setLang } = useI18n();

  const currentTheme = document.documentElement.getAttribute('data-theme') ?? 'dark';

  const setTheme = (theme: 'dark' | 'light') => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('pc_doctor_theme', theme);
  };

  const handlePolling = (ms: number) => {
    setPollingMs(ms);
    localStorage.setItem('pc_doctor_polling', String(ms));
  };

  return (
    <div className="settings-panel">
      <h1 className="page-title">{t.settings_title}</h1>

      {/* Language */}
      <div className="settings-card card">
        <label className="setting-label">{t.language_label}</label>
        <div className="option-group">
          {(['vi', 'en'] as Language[]).map(l => (
            <button
              key={l}
              className={`option-btn ${lang === l ? 'active' : ''}`}
              onClick={() => setLang(l)}
            >
              {l === 'vi' ? '🇻🇳 Tiếng Việt' : '🇬🇧 English'}
            </button>
          ))}
        </div>
      </div>

      {/* Theme */}
      <div className="settings-card card">
        <label className="setting-label">{t.theme_label}</label>
        <div className="option-group">
          <button
            className={`option-btn ${currentTheme === 'dark' ? 'active' : ''}`}
            onClick={() => setTheme('dark')}
          >
            🌙 {t.theme_dark}
          </button>
          <button
            className={`option-btn ${currentTheme === 'light' ? 'active' : ''}`}
            onClick={() => setTheme('light')}
          >
            ☀️ {t.theme_light}
          </button>
        </div>
      </div>

      {/* Polling interval */}
      <div className="settings-card card">
        <label className="setting-label">{t.refresh_rate}</label>
        <div className="option-group">
          {[
            [1000, t.refresh_1s],
            [2000, t.refresh_2s],
            [5000, t.refresh_5s],
          ].map(([ms, label]) => (
            <button
              key={ms}
              className={`option-btn ${pollingMs === ms ? 'active' : ''}`}
              onClick={() => handlePolling(ms as number)}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* About */}
      <div className="settings-card card about-card">
        <div className="about-header">
          <span className="about-logo">🩺</span>
          <div>
            <div className="about-name">{t.about_title}</div>
            <div className="about-version">{t.version} 0.1.0</div>
          </div>
        </div>
        <p className="about-desc">{t.about_desc}</p>
        <div className="about-pills">
          <span className="pill">Tauri 2</span>
          <span className="pill">Rust</span>
          <span className="pill">React 19</span>
          <span className="pill">sysinfo</span>
          <span className="pill">100% Offline</span>
        </div>
      </div>
    </div>
  );
}
