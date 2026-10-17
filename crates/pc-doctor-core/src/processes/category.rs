pub use crate::models::ProcessCategory;

/// Known core system binaries
const SYSTEM_BINARIES: &[&str] = &[
    "system",
    "system idle process",
    "registry",
    "smss",
    "csrss",
    "wininit",
    "services",
    "lsass",
    "winlogon",
    "dwm",
    "fontdrvhost",
    "explorer",
];

/// Known web browser executables
const BROWSER_BINARIES: &[&str] = &[
    "chrome",
    "msedge",
    "msedgewebview2",
    "firefox",
    "brave",
    "opera",
    "opera_gx",
    "vivaldi",
    "arc",
    "safari",
    "tor",
    "chromium",
    "waterfox",
    "librewolf",
];

/// Known background services and host daemons
const BACKGROUND_SERVICES: &[&str] = &[
    "svchost",
    "spoolsv",
    "taskhostw",
    "sihost",
    "runtimebroker",
    "searchindexer",
    "searchhost",
    "startmenuexperiencehost",
    "securityhealthservice",
    "msmpeng",
    "nissrv",
    "trustedinstaller",
    "wuauserv",
    "conhost",
    "wmiprvse",
    "dllhost",
    "smartscreen",
    "audiodg",
    "lsiso",
    "dashost",
    "sedsvc",
    "ctfmon",
];

/// Normalizes process executable name by lowercasing and trimming '.exe'
pub fn normalize_proc_name(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    if let Some(stripped) = lower.strip_suffix(".exe") {
        stripped.to_string()
    } else {
        lower
    }
}

/// Categorizes a process into System, BackgroundService, BrowserEngine, or Application.
pub fn categorize_process(pid: u32, name: &str, exe_path: Option<&str>) -> ProcessCategory {
    // 1. PID <= 4 is always core Windows System
    if pid <= 4 {
        return ProcessCategory::System;
    }

    let norm_name = normalize_proc_name(name);

    // 2. Check core System binaries
    if SYSTEM_BINARIES.contains(&norm_name.as_str()) {
        return ProcessCategory::System;
    }

    // 3. Check Browser Engines
    if BROWSER_BINARIES.contains(&norm_name.as_str())
        || norm_name.contains("browser")
        || norm_name.contains("webview")
    {
        return ProcessCategory::BrowserEngine;
    }

    if let Some(path) = exe_path {
        let path_lower = path.to_lowercase();
        if path_lower.contains("\\google\\chrome\\")
            || path_lower.contains("\\microsoft\\edge\\")
            || path_lower.contains("\\mozilla firefox\\")
            || path_lower.contains("\\bravesoftware\\")
            || path_lower.contains("\\opera\\")
            || path_lower.contains("\\vivaldi\\")
        {
            return ProcessCategory::BrowserEngine;
        }
    }

    // 4. Check Background Services
    if BACKGROUND_SERVICES.contains(&norm_name.as_str())
        || norm_name.ends_with("service")
        || norm_name.ends_with("svc")
        || norm_name.ends_with("daemon")
        || norm_name.ends_with("agent")
        || norm_name.ends_with("helper")
        || norm_name.ends_with("host")
    {
        return ProcessCategory::BackgroundService;
    }

    if let Some(path) = exe_path {
        let path_lower = path.to_lowercase();
        if (path_lower.contains("\\windows\\system32\\")
            || path_lower.contains("\\windows\\syswow64\\")
            || path_lower.contains("\\windows\\servicing\\"))
            && !SYSTEM_BINARIES.contains(&norm_name.as_str())
        {
            return ProcessCategory::BackgroundService;
        }
    }

    // 5. Default to Application
    ProcessCategory::Application
}
