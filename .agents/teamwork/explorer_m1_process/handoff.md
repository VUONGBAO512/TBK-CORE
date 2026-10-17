# Handoff Report — Milestone 1: Process Guard & Safety Blacklist (`pc-doctor-core::processes`)

**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_process`  
**Author**: Explorer for Milestone 1 Process Guard & Blacklist  
**Parent / Caller**: `d0833960-9ed1-454f-b7d6-be733f742605`  
**Date**: 2026-10-16  
**Status**: COMPLETE (Hard Handoff)  

---

## 1. Observation

Direct codebase inspection and runtime verification observations:

1. **Workspace and Target Crates**:
   - `crates/pc-doctor-core`: Contains pure domain models, diagnostic rules (`rules.rs`), and bilingual knowledge base (`knowledge/mod.rs`). Currently lacks `crates/pc-doctor-core/src/processes/`.
   - `crates/pc-doctor-core/Cargo.toml:7-12`:
     ```toml
     [dependencies]
     serde = { version = "1", features = ["derive"] }
     serde_json = "1"
     chrono = { version = "0.4", features = ["serde"] }
     uuid = { version = "1", features = ["v4", "serde"] }
     ```
     `pc-doctor-core` has zero platform or OS C-bindings. It does not depend directly on `sysinfo`, which allows all process classification, runaway rules, blacklist enforcement, and structured safety errors to be 100% pure Rust unit-testable across all platforms.
   - `crates/pc-doctor-core/src/models.rs:80-86`:
     ```rust
     pub struct ProcessInfo {
         pub pid: u32,
         pub name: String,
         pub cpu_percent: f32,
         pub memory_bytes: u64,
         pub status: String,
     }
     ```
     Currently lacks `category: ProcessCategory` and `is_protected: bool`.

2. **Project Specification & Contract (`PROJECT.md:89-114`)**:
   - Mandates interface contract:
     ```rust
     pub enum ProcessCategory {
         System,
         BackgroundService,
         Application,
         BrowserEngine,
     }

     pub struct ProcessItem {
         pub pid: u32,
         pub name: String,
         pub cpu_percent: f32,
         pub memory_bytes: u64,
         pub status: String,
         pub category: ProcessCategory,
         pub is_protected: bool,
     }

     pub struct TerminateResult {
         pub pid: u32,
         pub process_name: String,
         pub success: bool,
         pub message: String,
     }
     ```

3. **Current Process Inspection in Desktop App (`apps/desktop/src-tauri/src/monitoring/collector.rs:160-187`)**:
   - Collects `ProcessInfo` without categorization or protection flags.
   - Sorts descending by CPU percentage, with memory fallback.
   - `apps/desktop/src-tauri/src/commands/mod.rs` currently lacks a `terminate_process` IPC command.

4. **Desktop UI Consumption (`apps/desktop/src/components/processes/ProcessPanel.tsx:1-106`)**:
   - Renders a process inspection table with CPU and RAM sorting.
   - `PROJECT.md:18` prescribes expanding `ProcessPanel.tsx` with category tabs, protected status badges, and termination modal, consuming `ProcessCategory` and `is_protected`.

5. **Tool Commands and Results**:
   - `cargo check --workspace`: Succeeded with code 0.
   - `cargo test -p pc-doctor-core`: Succeeded with 3 passed, 0 failed.

---

## 2. Logic Chain

1. **Decoupled Architecture (Observation 1 -> Domain Isolation)**:
   - Terminating Windows processes is an OS-level operation (`sysinfo::Process::kill()` or Win32 `TerminateProcess`), but deciding *whether* a process is allowed to be terminated is pure business domain safety logic.
   - By isolating categorization, runaway detection, and blacklist validation into `pc-doctor-core::processes`, the safety gate operates deterministically without I/O or platform side effects. The execution layer (`desktop_lib`) is required to consult `ProcessGuard::validate_termination` before invoking any destructive system call.

2. **Categorization Taxonomy (Observation 2 & Prompt Requirement 1)**:
   - System users require clarity when inspecting running processes. The four mandated categories provide intuitive segmentation:
     - **`System`**: Core Windows kernel, session managers, display subsystems (`PID <= 4`, `csrss`, `smss`, `wininit`, `dwm`, `explorer`, `services`, `lsass`).
     - **`BrowserEngine`**: Web browser rendering pipelines and host executables (`chrome`, `msedge`, `msedgewebview2`, `firefox`, `brave`, `opera`, `vivaldi`, `arc`, `tor`).
     - **`BackgroundService`**: System and user daemons, service hosts, and background tasks (`svchost`, `spoolsv`, `taskhostw`, `sihost`, `SearchIndexer`, `RuntimeBroker`, `SecurityHealthService`, `msmpeng`, `conhost`, `*service`, `*daemon`, `*host`).
     - **`Application`**: Interactive user-mode desktop applications (VS Code `code.exe`, `notepad`, `discord`, `spotify`, games, tools).
   - Normalization must strip `.exe` extensions and lowercase inputs to prevent bypasses or misclassification due to casing differences.

3. **Runaway Process Detection Criteria (Prompt Requirement 2)**:
   - Single metric spikes:
     - `cpu_percent >= 90.0%`: A single process monopolizing CPU resources.
     - `memory_bytes > 2 * 1024 * 1024 * 1024` (2 GB): A single process holding excessive working memory.
   - Dual metric stress:
     - `cpu_percent >= 70.0%` AND `memory_bytes >= 1_610_612_736` (1.5 GB): Combined compute and memory pressure indicating an aggressive runaway or memory leak under heavy load.
   - Returning a structured `RunawayKind` enum alongside boolean `is_runaway` allows the UI and diagnostic engines to display informative badges (e.g. "Runaway (CPU 94%)", "Runaway (RAM 3.2 GB)", "Runaway (Dual Stress)").

4. **Hardcoded Windows OS Safety Blacklist (Prompt Requirement 3 & 4)**:
   - Attempting to terminate vital Windows processes causes instant system instability:
     - Terminating `csrss.exe`, `smss.exe`, `wininit.exe`, or `services.exe` immediately crashes Windows with a Blue Screen of Death (`CRITICAL_PROCESS_DIED` or `KERNEL_DATA_INPAGE_ERROR`).
     - Terminating `lsass.exe` causes Windows to pop an un-dismissible countdown dialog forcing a reboot.
     - Terminating `dwm.exe` or `explorer.exe` breaks graphical window compositing and crashes the taskbar/shell.
     - Terminating `PID <= 4` targets the System Idle Process or Windows NT Kernel thread.
     - Terminating `self PID` causes PC Doctor to crash itself mid-operation.
   - Therefore, `ProcessGuard::validate_termination` MUST block these 11 mandatory targets and return `Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason })` containing a detailed, user-intelligible explanation.

5. **Testability & Robustness (Prompt Requirement 5)**:
   - By allowing `ProcessGuard::with_self_pid(test_pid)`, unit tests can inject mock self-PIDs and verify self-termination prevention deterministically without altering the host process.
   - Comprehensive test cases must verify boundary conditions (e.g. CPU 89.9% vs 90.0%, RAM 2GB vs 2GB+1 byte, uppercase vs lowercase process names, presence or absence of `.exe`).

---

## 3. Caveats

1. **Process Masquerading**:
   - A malicious binary on a compromised machine could name itself `explorer.exe` or `notepad.exe`. In this user-mode diagnostics tool, safety blacklist enforcement prioritizes preventing catastrophic OS crashes; therefore, protecting `explorer.exe` regardless of file hash is the safe default. Future milestones could add cryptographic code-signature validation if elevated privileges are enabled.
2. **Dynamic PID Reallocation**:
   - Windows reallocates PIDs after process termination. The safety verification gate takes both `pid` and `name` to ensure the process identity matches before authorizing termination.
3. **OS-Level Permissions (Access Denied)**:
   - Validating that an application is not protected does not guarantee the OS will allow its termination (e.g., if running under another user account without UAC elevation). If the OS returns `ERROR_ACCESS_DENIED`, the platform layer converts this into `ProcessTerminationError::AccessDenied`.

---

## 4. Conclusion & Implementation Blueprint

### Module File Layout
The `pc-doctor-core::processes` module will reside in `crates/pc-doctor-core/src/processes/` with the following clean architecture:

```
crates/pc-doctor-core/src/processes/
├── mod.rs        # Public API exports, convenience functions, and integration tests
├── category.rs   # ProcessCategory enum and categorization heuristics
├── runaway.rs    # Runaway detection thresholds, RunawayKind, and detection logic
├── guard.rs      # ProcessGuard, hardcoded Windows OS blacklist, and validation logic
└── error.rs      # ProcessTerminationError enum, Display, and Error implementations
```

---

### Detailed Code Blueprint

#### 1. `crates/pc-doctor-core/src/processes/error.rs`
```rust
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error_type", content = "details")]
pub enum ProcessTerminationError {
    ProtectedSystemProcess {
        pid: u32,
        name: String,
        reason: String,
    },
    ProcessNotFound {
        pid: u32,
    },
    AccessDenied {
        pid: u32,
        name: String,
        os_error_code: u32,
        message: String,
    },
    InternalError {
        message: String,
    },
}

impl fmt::Display for ProcessTerminationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectedSystemProcess { pid, name, reason } => {
                write!(
                    f,
                    "Process '{}' (PID {}) is protected and cannot be terminated: {}",
                    name, pid, reason
                )
            }
            Self::ProcessNotFound { pid } => {
                write!(f, "Process with PID {} was not found", pid)
            }
            Self::AccessDenied {
                pid,
                name,
                os_error_code,
                message,
            } => {
                write!(
                    f,
                    "Access denied terminating process '{}' (PID {}, OS code {}): {}",
                    name, pid, os_error_code, message
                )
            }
            Self::InternalError { message } => {
                write!(f, "Internal error during process termination: {}", message)
            }
        }
    }
}

impl std::error::Error for ProcessTerminationError {}
```

---

#### 2. `crates/pc-doctor-core/src/processes/category.rs`
```rust
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessCategory {
    System,
    BackgroundService,
    Application,
    BrowserEngine,
}

impl fmt::Display for ProcessCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => write!(f, "System"),
            Self::BackgroundService => write!(f, "Background Service"),
            Self::Application => write!(f, "Application"),
            Self::BrowserEngine => write!(f, "Browser Engine"),
        }
    }
}

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
```

---

#### 3. `crates/pc-doctor-core/src/processes/runaway.rs`
```rust
use serde::{Deserialize, Serialize};

pub const DEFAULT_RUNAWAY_CPU_PERCENT: f32 = 90.0;
pub const DEFAULT_RUNAWAY_MEMORY_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GB (2,147,483,648 bytes)
pub const DEFAULT_DUAL_CPU_PERCENT: f32 = 70.0;
pub const DEFAULT_DUAL_MEMORY_BYTES: u64 = 1_610_612_736; // 1.5 GB (1,610,612,736 bytes)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunawayThresholds {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub dual_cpu_percent: f32,
    pub dual_memory_bytes: u64,
}

impl Default for RunawayThresholds {
    fn default() -> Self {
        Self {
            cpu_percent: DEFAULT_RUNAWAY_CPU_PERCENT,
            memory_bytes: DEFAULT_RUNAWAY_MEMORY_BYTES,
            dual_cpu_percent: DEFAULT_DUAL_CPU_PERCENT,
            dual_memory_bytes: DEFAULT_DUAL_MEMORY_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunawayKind {
    Cpu { cpu_percent: f32 },
    Memory { memory_bytes: u64 },
    DualPressure { cpu_percent: f32, memory_bytes: u64 },
}

/// Returns true if a process exceeds healthy resource thresholds.
pub fn is_runaway(cpu_percent: f32, memory_bytes: u64) -> bool {
    is_runaway_with_thresholds(cpu_percent, memory_bytes, &RunawayThresholds::default())
}

pub fn is_runaway_with_thresholds(
    cpu_percent: f32,
    memory_bytes: u64,
    thresholds: &RunawayThresholds,
) -> bool {
    detect_runaway_with_thresholds(cpu_percent, memory_bytes, thresholds).is_some()
}

/// Detects the runaway kind if thresholds are exceeded, or None if healthy.
pub fn detect_runaway(cpu_percent: f32, memory_bytes: u64) -> Option<RunawayKind> {
    detect_runaway_with_thresholds(cpu_percent, memory_bytes, &RunawayThresholds::default())
}

pub fn detect_runaway_with_thresholds(
    cpu_percent: f32,
    memory_bytes: u64,
    thresholds: &RunawayThresholds,
) -> Option<RunawayKind> {
    // 1. Single CPU threshold (CPU >= 90%)
    if cpu_percent >= thresholds.cpu_percent {
        return Some(RunawayKind::Cpu { cpu_percent });
    }

    // 2. Single RAM threshold (RAM > 2GB)
    if memory_bytes > thresholds.memory_bytes {
        return Some(RunawayKind::Memory { memory_bytes });
    }

    // 3. Dual Pressure threshold (CPU >= 70% AND RAM >= 1.5GB)
    if cpu_percent >= thresholds.dual_cpu_percent && memory_bytes >= thresholds.dual_memory_bytes {
        return Some(RunawayKind::DualPressure {
            cpu_percent,
            memory_bytes,
        });
    }

    None
}
```

---

#### 4. `crates/pc-doctor-core/src/processes/guard.rs`
```rust
use super::category::normalize_proc_name;
use super::error::ProcessTerminationError;

/// Hardcoded Windows OS Process Safety Blacklist with structured danger explanations
pub const WINDOWS_PROTECTED_PROCESSES: &[(&str, &str)] = &[
    (
        "csrss.exe",
        "Client Server Runtime Subsystem (csrss.exe) is vital for Windows operation; terminating it triggers an immediate Blue Screen of Death (CRITICAL_PROCESS_DIED).",
    ),
    (
        "lsass.exe",
        "Local Security Authority Subsystem Service (lsass.exe) enforces Windows security policies; terminating it triggers an immediate system restart.",
    ),
    (
        "services.exe",
        "Service Control Manager (services.exe) manages all system background services; terminating it crashes the operating system.",
    ),
    (
        "explorer.exe",
        "Windows Explorer (explorer.exe) provides the desktop GUI, taskbar, and file manager; terminating it crashes the graphical user session.",
    ),
    (
        "smss.exe",
        "Session Manager Subsystem (smss.exe) is the first user-mode process started during boot; terminating it causes an instant kernel panic.",
    ),
    (
        "wininit.exe",
        "Windows Initialization Process (wininit.exe) launches core subsystems; terminating it triggers an immediate system crash.",
    ),
    (
        "winlogon.exe",
        "Windows Logon Process (winlogon.exe) manages user logon and secure attention sequences; terminating it locks the user out or causes a BSOD.",
    ),
    (
        "svchost.exe",
        "Service Host (svchost.exe) hosts critical Windows system services and networking components; terminating it disrupts essential operating system functionality.",
    ),
    (
        "dwm.exe",
        "Desktop Window Manager (dwm.exe) manages display compositing and window rendering; terminating it disrupts desktop display.",
    ),
    (
        "system",
        "Windows NT Kernel Process executing core system threads; cannot be terminated.",
    ),
    (
        "system idle process",
        "CPU Idle Loop Thread measuring processor idle cycles; cannot be terminated.",
    ),
    (
        "registry",
        "Windows Registry Subsystem holding in-memory registry hives; cannot be terminated.",
    ),
    (
        "fontdrvhost.exe",
        "User-Mode Font Driver Host providing isolated font rendering.",
    ),
    (
        "conhost.exe",
        "Console Window Host providing terminal UI support.",
    ),
    (
        "taskhostw.exe",
        "Host Process for Windows Tasks executing background system tasks.",
    ),
    (
        "sihost.exe",
        "Shell Infrastructure Host coordinating Windows shell components.",
    ),
    (
        "securityhealthservice.exe",
        "Windows Defender Security Center Service monitoring system protection.",
    ),
    (
        "msmpeng.exe",
        "Microsoft Defender Antivirus Core scanning engine.",
    ),
    (
        "tbk_core.exe",
        "PC Doctor / TBK Core desktop application guard preventing self-termination.",
    ),
    (
        "pc-doctor.exe",
        "PC Doctor application executable guard preventing self-termination.",
    ),
];

#[derive(Debug, Clone)]
pub struct ProcessGuard {
    self_pid: u32,
}

impl Default for ProcessGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessGuard {
    pub fn new() -> Self {
        Self {
            self_pid: std::process::id(),
        }
    }

    /// Construct a guard with an explicit self_pid (vital for deterministic unit testing)
    pub fn with_self_pid(self_pid: u32) -> Self {
        Self { self_pid }
    }

    pub fn self_pid(&self) -> u32 {
        self.self_pid
    }

    /// Validates whether a process can be safely terminated.
    /// Returns Ok(()) if safe to terminate, or Err(ProcessTerminationError) if protected.
    pub fn validate_termination(
        &self,
        pid: u32,
        name: &str,
    ) -> Result<(), ProcessTerminationError> {
        // 1. PID <= 4 check (PID 0 Idle, PID 4 System Kernel)
        if pid <= 4 {
            let proc_name = if name.is_empty() {
                if pid == 0 {
                    "System Idle Process"
                } else {
                    "System"
                }
            } else {
                name
            };
            return Err(ProcessTerminationError::ProtectedSystemProcess {
                pid,
                name: proc_name.to_string(),
                reason: format!(
                    "Process with PID {} is a core Windows NT kernel process and cannot be terminated.",
                    pid
                ),
            });
        }

        // 2. Self PID check
        if pid == self.self_pid {
            return Err(ProcessTerminationError::ProtectedSystemProcess {
                pid,
                name: name.to_string(),
                reason: "PC Doctor cannot terminate its own application process.".to_string(),
            });
        }

        // 3. Blacklist process name check (case-insensitive, normalized)
        let normalized = normalize_proc_name(name);
        for &(protected_name, reason) in WINDOWS_PROTECTED_PROCESSES {
            if normalized == normalize_proc_name(protected_name) {
                return Err(ProcessTerminationError::ProtectedSystemProcess {
                    pid,
                    name: name.to_string(),
                    reason: reason.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Returns true if the process is protected from termination.
    pub fn is_protected(&self, pid: u32, name: &str) -> bool {
        self.validate_termination(pid, name).is_err()
    }

    /// Returns the protection reason if protected, or None if unprotected.
    pub fn protection_reason(&self, pid: u32, name: &str) -> Option<String> {
        match self.validate_termination(pid, name) {
            Err(ProcessTerminationError::ProtectedSystemProcess { reason, .. }) => Some(reason),
            _ => None,
        }
    }
}
```

---

#### 5. `crates/pc-doctor-core/src/processes/mod.rs`
```rust
pub mod category;
pub mod error;
pub mod guard;
pub mod runaway;

pub use category::{categorize_process, normalize_proc_name, ProcessCategory};
pub use error::ProcessTerminationError;
pub use guard::{ProcessGuard, WINDOWS_PROTECTED_PROCESSES};
pub use runaway::{
    detect_runaway, detect_runaway_with_thresholds, is_runaway, is_runaway_with_thresholds,
    RunawayKind, RunawayThresholds, DEFAULT_DUAL_CPU_PERCENT, DEFAULT_DUAL_MEMORY_BYTES,
    DEFAULT_RUNAWAY_CPU_PERCENT, DEFAULT_RUNAWAY_MEMORY_BYTES,
};
```

---

### Unit Test Suite Specifications

Below is the complete, self-contained unit test suite that implementers must integrate into `crates/pc-doctor-core/src/processes/mod.rs` (under `#[cfg(test)] mod tests`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // 1. Blacklist & Safety Guard Tests
    // ==========================================

    #[test]
    fn test_guard_blocks_pid_zero() {
        let guard = ProcessGuard::with_self_pid(9999);
        let res = guard.validate_termination(0, "System Idle Process");
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, reason, .. }) => {
                assert_eq!(pid, 0);
                assert!(!reason.is_empty());
            }
            _ => panic!("Expected ProtectedSystemProcess error for PID 0"),
        }
    }

    #[test]
    fn test_guard_blocks_pid_four_system() {
        let guard = ProcessGuard::with_self_pid(9999);
        let res = guard.validate_termination(4, "System");
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, .. }) => {
                assert_eq!(pid, 4);
            }
            _ => panic!("Expected ProtectedSystemProcess error for PID 4"),
        }
    }

    #[test]
    fn test_guard_blocks_all_pids_less_than_or_equal_to_four() {
        let guard = ProcessGuard::with_self_pid(9999);
        for pid in 0..=4 {
            let res = guard.validate_termination(pid, "test_proc");
            assert!(
                matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })),
                "PID {} must be protected",
                pid
            );
            assert!(guard.is_protected(pid, "test_proc"));
        }
    }

    #[test]
    fn test_guard_blocks_self_pid() {
        let self_pid = 12345;
        let guard = ProcessGuard::with_self_pid(self_pid);
        let res = guard.validate_termination(self_pid, "custom_runner.exe");
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, reason, .. }) => {
                assert_eq!(pid, self_pid);
                assert!(reason.contains("own application process"));
            }
            _ => panic!("Expected ProtectedSystemProcess error for self PID"),
        }
        assert!(guard.is_protected(self_pid, "custom_runner.exe"));
    }

    #[test]
    fn test_guard_blocks_mandatory_windows_blacklist() {
        let guard = ProcessGuard::with_self_pid(9999);
        let mandatory_names = [
            "csrss.exe",
            "lsass.exe",
            "services.exe",
            "explorer.exe",
            "smss.exe",
            "wininit.exe",
            "winlogon.exe",
            "svchost.exe",
            "dwm.exe",
        ];

        for (i, name) in mandatory_names.iter().enumerate() {
            let test_pid = 1000 + i as u32;
            let res = guard.validate_termination(test_pid, name);
            match res {
                Err(ProcessTerminationError::ProtectedSystemProcess { pid, name: proc_name, reason }) => {
                    assert_eq!(pid, test_pid);
                    assert_eq!(proc_name, *name);
                    assert!(!reason.is_empty(), "Reason for {} must not be empty", name);
                }
                _ => panic!("Process {} must be protected by blacklist", name),
            }
            assert!(guard.is_protected(test_pid, name), "is_protected must be true for {}", name);
        }
    }

    #[test]
    fn test_guard_blocks_blacklist_case_insensitive_and_without_extension() {
        let guard = ProcessGuard::with_self_pid(9999);
        let variations = [
            "CSRSS.EXE",
            "csrss",
            "Explorer.exe",
            "EXPLORER",
            "Dwm.Exe",
            "DWM",
            "SvChOsT.eXe",
            "svchost",
            "LSASS.EXE",
            "Services.exe",
        ];

        for name in variations {
            let res = guard.validate_termination(8000, name);
            assert!(
                matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })),
                "Variation '{}' must be protected",
                name
            );
        }
    }

    #[test]
    fn test_guard_approves_unprotected_applications() {
        let guard = ProcessGuard::with_self_pid(9999);
        let safe_apps = [
            "notepad.exe",
            "chrome.exe",
            "code.exe",
            "spotify.exe",
            "discord.exe",
            "steam.exe",
            "game.exe",
            "photoshop.exe",
        ];

        for (i, name) in safe_apps.iter().enumerate() {
            let test_pid = 2000 + i as u32;
            let res = guard.validate_termination(test_pid, name);
            assert!(res.is_ok(), "Unprotected process '{}' should be approved", name);
            assert!(!guard.is_protected(test_pid, name), "is_protected should be false for '{}'", name);
            assert!(guard.protection_reason(test_pid, name).is_none());
        }
    }

    // ==========================================
    // 2. Process Categorization Tests
    // ==========================================

    #[test]
    fn test_categorize_system_processes() {
        assert_eq!(categorize_process(0, "System Idle Process", None), ProcessCategory::System);
        assert_eq!(categorize_process(4, "System", None), ProcessCategory::System);
        assert_eq!(categorize_process(100, "csrss.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(101, "CSRSS", None), ProcessCategory::System);
        assert_eq!(categorize_process(102, "smss.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(103, "wininit.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(104, "winlogon.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(105, "dwm.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(106, "explorer.exe", None), ProcessCategory::System);
    }

    #[test]
    fn test_categorize_browser_engines() {
        assert_eq!(categorize_process(200, "chrome.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(201, "CHROME.EXE", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(202, "msedge.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(203, "msedgewebview2.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(204, "firefox.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(205, "brave.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(206, "opera.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(207, "vivaldi.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(208, "arc.exe", None), ProcessCategory::BrowserEngine);

        // Path-based browser recognition
        assert_eq!(
            categorize_process(
                209,
                "browser_worker",
                Some(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
            ),
            ProcessCategory::BrowserEngine
        );
    }

    #[test]
    fn test_categorize_background_services() {
        assert_eq!(categorize_process(300, "svchost.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(301, "spoolsv.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(302, "taskhostw.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(303, "sihost.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(304, "RuntimeBroker.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(305, "SearchIndexer.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(306, "SecurityHealthService.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(307, "conhost.exe", None), ProcessCategory::BackgroundService);

        // Suffix-based service recognition
        assert_eq!(categorize_process(308, "custom_agent.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(309, "my_backup_daemon", None), ProcessCategory::BackgroundService);
    }

    #[test]
    fn test_categorize_applications() {
        assert_eq!(categorize_process(400, "code.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(401, "notepad.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(402, "discord.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(403, "spotify.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(404, "steam.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(405, "slack.exe", None), ProcessCategory::Application);
    }

    // ==========================================
    // 3. Runaway Process Detection Tests
    // ==========================================

    #[test]
    fn test_runaway_healthy_process() {
        let cpu = 15.0;
        let mem = 250 * 1024 * 1024; // 250 MB
        assert!(!is_runaway(cpu, mem));
        assert_eq!(detect_runaway(cpu, mem), None);
    }

    #[test]
    fn test_runaway_cpu_boundary() {
        let mem = 100 * 1024 * 1024;
        // Below 90% is healthy
        assert!(!is_runaway(89.9, mem));
        assert_eq!(detect_runaway(89.9, mem), None);

        // >= 90% is runaway CPU
        assert!(is_runaway(90.0, mem));
        assert_eq!(detect_runaway(90.0, mem), Some(RunawayKind::Cpu { cpu_percent: 90.0 }));

        assert!(is_runaway(99.5, mem));
        assert_eq!(detect_runaway(99.5, mem), Some(RunawayKind::Cpu { cpu_percent: 99.5 }));
    }

    #[test]
    fn test_runaway_memory_boundary() {
        let cpu = 5.0;
        let two_gb = 2 * 1024 * 1024 * 1024; // 2,147,483,648 bytes

        // Exactly 2 GB is not > 2 GB
        assert!(!is_runaway(cpu, two_gb));
        assert_eq!(detect_runaway(cpu, two_gb), None);

        // > 2 GB is runaway Memory
        let two_gb_plus_one = two_gb + 1;
        assert!(is_runaway(cpu, two_gb_plus_one));
        assert_eq!(
            detect_runaway(cpu, two_gb_plus_one),
            Some(RunawayKind::Memory { memory_bytes: two_gb_plus_one })
        );

        let three_gb = 3 * 1024 * 1024 * 1024;
        assert!(is_runaway(cpu, three_gb));
    }

    #[test]
    fn test_runaway_dual_pressure_detection() {
        let dual_mem = 1_610_612_736; // 1.5 GB
        let dual_cpu = 70.0;

        // Both conditions met: Dual Pressure Runaway
        assert!(is_runaway(dual_cpu, dual_mem));
        assert_eq!(
            detect_runaway(dual_cpu, dual_mem),
            Some(RunawayKind::DualPressure {
                cpu_percent: dual_cpu,
                memory_bytes: dual_mem,
            })
        );

        // High CPU (75%) and High RAM (1.8 GB) -> Dual Pressure
        assert!(is_runaway(75.0, 1_800_000_000));
        assert_eq!(
            detect_runaway(75.0, 1_800_000_000),
            Some(RunawayKind::DualPressure {
                cpu_percent: 75.0,
                memory_bytes: 1_800_000_000,
            })
        );

        // Only CPU high (75%), RAM low (500 MB) -> Not runaway
        assert!(!is_runaway(75.0, 500 * 1024 * 1024));
        assert_eq!(detect_runaway(75.0, 500 * 1024 * 1024), None);

        // Only RAM high (1.8 GB), CPU low (30%) -> Not runaway (below 2GB single threshold)
        assert!(!is_runaway(30.0, 1_800_000_000));
        assert_eq!(detect_runaway(30.0, 1_800_000_000), None);
    }

    // ==========================================
    // 4. Serde & Serialization Tests
    // ==========================================

    #[test]
    fn test_error_json_serialization() {
        let err = ProcessTerminationError::ProtectedSystemProcess {
            pid: 4,
            name: "System".to_string(),
            reason: "Windows kernel".to_string(),
        };

        let json = serde_json::to_string(&err).expect("Serialization failed");
        assert!(json.contains(r#""error_type":"ProtectedSystemProcess""#));
        assert!(json.contains(r#""pid":4"#));
        assert!(json.contains(r#""name":"System""#));

        let deserialized: ProcessTerminationError =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(err, deserialized);
    }

    #[test]
    fn test_category_json_serialization() {
        let cat = ProcessCategory::BackgroundService;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, r#""background_service""#);

        let cat_browser = ProcessCategory::BrowserEngine;
        assert_eq!(serde_json::to_string(&cat_browser).unwrap(), r#""browser_engine""#);
    }
}
```

---

## 5. Verification Method

To independently verify the implementation blueprint:

1. **Static Analysis & Compilation Check**:
   - Verify that `crates/pc-doctor-core/src/lib.rs` declares `pub mod processes;` and exposes `ProcessGuard`, `ProcessCategory`, `ProcessTerminationError`, `is_runaway`, etc.
   - Run:
     ```powershell
     cargo check --workspace
     ```
     Expected result: Compilation succeeds with 0 errors.

2. **Automated Unit Test Execution**:
   - Run the full test suite in `pc-doctor-core`:
     ```powershell
     cargo test -p pc-doctor-core -- processes
     ```
     Expected result: All 17 unit tests execute and pass with 0 failures:
     - 7 tests for `ProcessGuard` verifying PID <= 4, self PID, mandatory blacklist (csrss, lsass, services, explorer, smss, wininit, winlogon, svchost, dwm), case-insensitivity, and approval of unprotected apps.
     - 4 tests for `categorize_process` across System, BrowserEngine, BackgroundService, and Application.
     - 4 tests for `runaway` detection across healthy, CPU >= 90%, RAM > 2GB, and dual pressure (CPU >= 70% & RAM >= 1.5GB).
     - 2 tests for JSON serialization and TypeScript compatibility.

3. **Invalidation Conditions**:
   - Any failure where terminating `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `wininit.exe`, `winlogon.exe`, `svchost.exe`, `dwm.exe`, PID <= 4, or self PID returns `Ok(())` instead of `Err(ProcessTerminationError::ProtectedSystemProcess)`.
   - Any scenario where an unprotected application (e.g. `notepad.exe`) is rejected.
   - Any misclassification where a web browser (e.g. `chrome.exe`) is tagged as `Application` instead of `BrowserEngine`.
