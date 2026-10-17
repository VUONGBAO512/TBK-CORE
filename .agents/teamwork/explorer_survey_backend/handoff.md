# Backend Architectural Survey & Implementation Plan for PC Doctor (TBK Core)

**Report Date**: 2026-10-16  
**Author**: Backend Architecture Explorer  
**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_survey_backend`  
**Target Codebase**: `d:/HOCC++/pc-doctor`  
**Reference Document**: `d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md`

---

## 1. Observation

### 1.1 Workspace Structure & Crate Boundaries
The Rust workspace is declared at `d:/HOCC++/pc-doctor/Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = [
    "crates/pc-doctor-core",
    "apps/desktop/src-tauri",
]
```

- **`crates/pc-doctor-core`**:
  - `Cargo.toml`:
    - Name: `pc-doctor-core` (v0.1.0, edition 2021)
    - Dependencies: `serde` ("1", derive), `serde_json` ("1"), `chrono` ("0.4", serde), `uuid` ("1", v4, serde).
    - Status: Pure domain logic crate without I/O or platform dependencies. Decoupled from Windows APIs and `sysinfo`.
  - Source Tree:
    - `src/lib.rs`: Exports `diagnostics`, `knowledge`, `models`, and runs 3 unit tests.
    - `src/models.rs`: Defines data contracts (`CpuSnapshot`, `MemorySnapshot`, `DiskSnapshot`, `OsSnapshot`, `TemperatureSensor`, `ProcessInfo`, `DiagnosticFinding`, `SystemSnapshot`, `SeverityLevel`, `CompanionMood`).
    - `src/diagnostics/rules.rs`: Implements `DiagnosticEngine` and `DiagnosticHistory`.
    - `src/knowledge/mod.rs`: Implements `KnowledgeBase` and `KnowledgeArticle` with 5 bilingual articles.

- **`apps/desktop/src-tauri`**:
  - `Cargo.toml`:
    - Package: `desktop` (v0.1.0, edition 2024), binary `TBK_Core`, library `desktop_lib`.
    - Dependencies:
      - `tauri = "2"`
      - `tauri-plugin-opener = "2"`
      - `serde = "1"`, `serde_json = "1"`
      - `sysinfo = "0.33"`
      - `chrono = "0.4"`
      - `tokio = "1"` (features = ["full"])
      - `axum = "0.8"`
      - `tower-http = "0.6"` (features = ["cors"])
      - `pc-doctor-core = { path = "../../../crates/pc-doctor-core" }`
    - Contains lines 35-40:
      ```toml
      [profile.release]
      codegen-units = 1
      lto = true
      opt-level = 3
      panic = "abort"
      strip = true
      ```
  - Source Tree:
    - `src/main.rs`: Calls `desktop_lib::run()`.
    - `src/lib.rs`: Sets up `SystemCollector`, wraps in `Arc<CollectorState>`, starts background pairing server, registers Tauri commands: `get_system_snapshot`, `get_process_list`, `query_assistant`.
    - `src/monitoring/collector.rs`: Interacts with `sysinfo::System`, `sysinfo::Disks`, and `sysinfo::Components`.
    - `src/server.rs`: Runs local HTTP server at `127.0.0.1:45120` (`/api/v1/ping`, `/api/v1/snapshot`) for loopback dashboard pairing.
    - `src/commands/mod.rs`: Handles IPC requests from the React frontend.

- **Frontend & Shared Types**:
  - `packages/shared-types/src/index.ts`: TypeScript type definitions mirroring `pc-doctor-core::models`.
  - `apps/desktop/src/hooks/useMonitoring.ts`: Polls `get_system_snapshot` and `get_process_list`.
  - `apps/desktop/src/components/processes/ProcessPanel.tsx`: Displays process table sorted by CPU/RAM.

---

### 1.2 Verification of Existing Build & Test State

1. **`cargo check --workspace`**:
   - Exit code: `0`
   - Warning observed verbatim:
     ```
     warning: profiles for the non root package will be ignored, specify profiles at the workspace root:
     package:   D:\HOCC++\pc-doctor\apps\desktop\src-tauri\Cargo.toml
     workspace: D:\HOCC++\pc-doctor\Cargo.toml
     ```
   - Rationale: Cargo requires `[profile.*]` sections to be placed in the workspace root `Cargo.toml`.

2. **`cargo test --workspace`**:
   - Exit code: `0`
   - Output:
     ```
     running 3 tests
     test tests::test_diagnostic_sustained_cpu ... ok
     test tests::test_diagnostic_engine_detects_low_disk ... ok
     test tests::test_knowledge_base_search ... ok

     test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
   - Note: 0 unit tests currently exist in `apps/desktop/src-tauri`.

3. **`npm run build` inside `apps/desktop`**:
   - Exit code: `0`
   - Output:
     ```
     dist/index.html                   0.50 kB │ gzip:  0.34 kB
     dist/assets/index-Bq9vP0ui.css   27.03 kB │ gzip:  5.53 kB
     dist/assets/index-BVcdlEQU.js   255.70 kB │ gzip: 78.80 kB
     ✓ built in 151ms
     ```

---

### 1.3 Implementation Status by Requirement

| Requirement | Current Status | Implemented Components | Deficiencies & Gaps |
| :--- | :--- | :--- | :--- |
| **R1. Deterministic System Diagnostics & Health Telemetry** | **85% Complete** | `SystemCollector` gathers CPU (global, per-core, frequency), RAM, Disks, OS, Uptime, top 20 processes. `DiagnosticEngine` evaluates 8 rules into 5 severity tiers (`Critical`, `High`, `Moderate`, `Low`, `Informational`). | Diagnostic findings only generated in English. Missing swap exhaustion rule, per-core runaway rule, and memory leak trend tracking. |
| **R2. Actionable Storage & Temp Cache Scanner** | **0% Complete (Backend)** | Frontend has static advice in `StorageAdvisorSection` (`App.tsx:214-277`) showing disk space. | **Completely absent from backend**. No scanning functions, no reclaimable byte calculations, no dry-run engine, no safe purge command, no locked-file error handling. |
| **R3. Safe Process Inspection & Resource Management** | **30% Complete** | `get_process_list` queries top 30 processes sorted by CPU and memory. Frontend has search and sort. | **No process termination command**. **No hardcoded Windows OS process blacklist**. **No process categorization** (`System`, `BackgroundService`, `Application`, `BrowserEngine`). |
| **R4. Offline Assistant & Bilingual Guidance** | **75% Complete** | `KnowledgeBase` has 5 articles in English & Vietnamese. `query_assistant` returns health summaries & KB answers. | Knowledge base matching is basic substring lookup. Assistant does not integrate storage scanner or process management context. |

---

## 2. Logic Chain

### 2.1 Crate Boundary Separation (Pure Domain vs Platform Adapter)
- **Observation**: `pc-doctor-core` currently depends only on `serde`, `serde_json`, `chrono`, and `uuid`. It compiles in under 1 second and executes in-memory unit tests in milliseconds.
- **Deduction**: Keeping `pc-doctor-core` decoupled from system-level calls (or placing I/O logic behind clean traits/pure standard library routines) allows the entire business logic (diagnostic evaluation, storage category definitions, safety whitelist validation, process blacklist, and bilingual reasoning) to be tested deterministically in CI without requiring administrator privileges or active Windows background services.
- **Conclusion**:
  1. Business rules, safety blacklists, target path generators, and data models belong in `crates/pc-doctor-core`.
  2. Platform execution (reading Windows `sysinfo`, executing file deletion, terminating processes via PID, and Tauri IPC command registration) belongs in `apps/desktop/src-tauri`.

---

### 2.2 Storage Scanner & Cleaner Safety Mechanics (R2)
- **Observation**: The acceptance criteria demand:
  - Scanning safe Windows temporary directories (`%TEMP%`, user crash dumps, application logs, thumbnail cache, prefetch).
  - Computing exact reclaimable byte sizes with itemized breakdown.
  - Safe purge execution with dry-run/preview support.
  - Handling locked/in-use files safely without panicking.
  - Protecting files outside safe locations.
- **Deduction**:
  1. *Directory Whitelist Containment*: An attacker or malformed request could pass arbitrary target paths. Therefore, the backend must maintain an immutable whitelist of allowed root directories (`std::env::temp_dir()`, `%LOCALAPPDATA%\CrashDumps`, `%LOCALAPPDATA%\Microsoft\Windows\WER`, `%LOCALAPPDATA%\Microsoft\Windows\Explorer`, and `%WINDIR%\Temp`). Every path scheduled for scanning or cleaning must be canonicalized and verified to start strictly with an approved safe root.
  2. *Locked File Handling*: Windows raises error code 32 (`ERROR_SHARING_VIOLATION`) or `io::ErrorKind::PermissionDenied` when deleting files actively held by running applications (e.g. active `.log` or `.tmp` files). The cleaner must catch `Err(e)` on each individual file removal, record the file in a `skipped_locked` counter, and proceed to the next file without aborting the batch or panicking.
  3. *Dry-Run Mode*: When `dry_run == true`, the engine traverses directories, tallies `file_count` and `size_bytes`, but executes zero `fs::remove_file` or `fs::remove_dir_all` calls.

---

### 2.3 Process Management & Safety Guardrails (R3)
- **Observation**: The acceptance criteria require a hardcoded safety blacklist protecting vital Windows OS processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`) and unit tests verifying termination rejection.
- **Deduction**:
  1. *Blacklist Invariant*: The blacklist must be an immutable, case-insensitive set of Windows critical process names, plus PID 0 (`System Idle Process`), PID 4 (`System`), and the current running application's own PID (`std::process::id()`).
  2. *Rejection Guardrail*: The function `ProcessManager::terminate_process(pid)` must query the process name from `sysinfo::System`. If the process name matches the blacklist or PID matches protected system PIDs, it immediately returns `Err(ProcessError::ProtectedProcess(name))`. The OS kill function is never reached.
  3. *Categorization*: Processes can be deterministically categorized by inspecting executable path, parent PID, or process name patterns into:
     - `System`: Matches core Windows system processes.
     - `BackgroundService`: Located under `System32` or running without active UI windows.
     - `BrowserEngine`: Known browser processes (`chrome.exe`, `msedge.exe`, `firefox.exe`, `brave.exe`).
     - `Application`: User desktop applications.

---

### 2.4 Profile Warning Resolution
- **Observation**: `apps/desktop/src-tauri/Cargo.toml` contains `[profile.release]` which triggers Cargo warning in workspace mode.
- **Deduction**: Moving the `[profile.release]` configuration to root `d:/HOCC++/pc-doctor/Cargo.toml` eliminates the warning while ensuring workspace-wide release optimizations (LTO, strip, abort on panic).

---

## 3. Caveats

1. **Windows Elevation Limitations**:
   - Certain directories like `C:\Windows\Temp` or `C:\Windows\Prefetch` may require elevated Administrator rights. If the user runs TBK Core as a standard user, attempts to read or delete in these system folders will return `PermissionDenied`. The scanner and cleaner must treat permission errors gracefully as skipped targets rather than hard failures.
2. **Thermal Sensors**:
   - `collector.rs:130-141` already documents that Windows user-mode applications cannot query ACPI/motherboard temperature zones without kernel-level drivers. This is correctly surfaced as `SENSOR_TEMP_UNAVAILABLE` (Severity: Informational) and is an intentional OS limitation.
3. **Open File Handles in Temp Directories**:
   - Active user sessions will frequently have open locks on files in `%TEMP%` (e.g. browser caches, runtime logs). A clean run will almost never report 100% of temp files deleted; reporting `files_skipped_locked` alongside `bytes_freed` is normal and expected.

---

## 4. Conclusion & Proposed Backend Architecture

To satisfy R1, R2, R3, and R4 cleanly while adhering to the **Deep Module** design principle (small interface, rich behavior hidden behind seams, comprehensive testability), we propose the following backend module structure.

### 4.1 Crate Architecture Overview

```
d:/HOCC++/pc-doctor/
├── Cargo.toml                              [Root Workspace: profile.release defined here]
├── crates/
│   └── pc-doctor-core/
│       ├── Cargo.toml                      [Add thiserror = "2"]
│       └── src/
│           ├── lib.rs                      [Exports diagnostics, storage, processes, knowledge, models]
│           ├── models.rs                   [Comprehensive domain types for all 4 requirements]
│           ├── diagnostics/
│           │   ├── mod.rs
│           │   └── rules.rs                [Deterministic diagnostic rules & rolling history]
│           ├── storage/
│           │   ├── mod.rs                  [Storage domain module root]
│           │   ├── targets.rs              [Cleanable target definitions, safe roots whitelist]
│           │   └── engine.rs               [Recursive scanning, byte calculation, safe deletion with locked-file skip]
│           ├── processes/
│           │   ├── mod.rs                  [Process management module root]
│           │   ├── blacklist.rs            [Hardcoded OS blacklist & self-protection guardrails]
│           │   └── classifier.rs           [Process categorization (System, Service, App, Browser)]
│           └── knowledge/
│               ├── mod.rs                  [Bilingual knowledge base articles]
│               └── assistant.rs            [Offline reasoning engine combining live telemetry + KB]
│
└── apps/desktop/src-tauri/
    ├── Cargo.toml                          [desktop crate: tauri, sysinfo, tokio, axum]
    └── src/
        ├── lib.rs                          [Tauri setup, State registration, command handlers]
        ├── main.rs                         [App entry point]
        ├── server.rs                       [Axum pairing server at 127.0.0.1:45120]
        ├── monitoring/
        │   ├── mod.rs
        │   └── collector.rs                [SystemCollector wrapping sysinfo + DiagnosticEngine]
        └── commands/
            ├── mod.rs                      [Tauri command exports]
            ├── system.rs                   [get_system_snapshot]
            ├── storage.rs                  [scan_cleanable_storage, clean_storage_targets]
            ├── process.rs                  [get_process_list, terminate_process]
            └── assistant.rs                [query_assistant]
```

---

### 4.2 Data Models & Deep Interfaces

#### A. Storage Scanner & Cleaner (`pc-doctor-core::storage`)
```rust
// In pc-doctor-core::models or pc_doctor_core::storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanableTarget {
    pub id: String,                    // e.g. "user_temp", "system_temp", "crash_dumps", "wer_reports", "thumbnails"
    pub name_en: String,
    pub name_vi: String,
    pub description_en: String,
    pub description_vi: String,
    pub root_path: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub is_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageScanReport {
    pub targets: Vec<CleanableTarget>,
    pub total_reclaimable_bytes: u64,
    pub total_files: usize,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanRequest {
    pub target_ids: Vec<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCleanSummary {
    pub dry_run: bool,
    pub bytes_freed: u64,
    pub files_deleted: usize,
    pub files_skipped_locked: usize,
    pub targets_processed: Vec<String>,
    pub errors: Vec<String>,
}

pub struct StorageEngine;
impl StorageEngine {
    pub fn scan_targets() -> StorageScanReport;
    pub fn execute_cleanup(request: CleanRequest) -> StorageCleanSummary;
    pub fn is_path_safe_to_clean(path: &Path) -> bool;
}
```

*Depth & Safeguards in `StorageEngine`:*
- `is_path_safe_to_clean(path)` canonicalizes the path and verifies it resides within designated safe folders:
  - `%TEMP%` / `%LOCALAPPDATA%\Temp`
  - `C:\Windows\Temp`
  - `%LOCALAPPDATA%\CrashDumps`
  - `%LOCALAPPDATA%\Microsoft\Windows\WER`
  - `%LOCALAPPDATA%\Microsoft\Windows\Explorer` (for `thumbcache_*.db`)
- Traversal checks for symlink loops and restricts deletion strictly to files and empty child directories.
- In `execute_cleanup`, if `remove_file` fails due to lock or permission, it does NOT abort: it logs the path, increments `files_skipped_locked`, and continues.

#### B. Safe Process Inspection & Termination (`pc-doctor-core::processes`)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessCategory {
    System,
    BackgroundService,
    Application,
    BrowserEngine,
}

pub struct ProcessGuard;
impl ProcessGuard {
    pub const PROTECTED_NAMES: &'static [&'static str] = &[
        "csrss.exe",
        "lsass.exe",
        "services.exe",
        "explorer.exe",
        "smss.exe",
        "wininit.exe",
        "winlogon.exe",
        "system",
        "system idle process",
        "svchost.exe",
        "runtimebroker.exe",
        "spoolsv.exe",
        "conhost.exe",
        "fontdrvhost.exe",
        "dwm.exe",
        "sihost.exe",
        "taskhostw.exe",
    ];

    pub fn is_protected(process_name: &str, pid: u32, self_pid: u32) -> bool {
        if pid == 0 || pid == 4 || pid == self_pid {
            return true;
        }
        let lower = process_name.to_lowercase();
        Self::PROTECTED_NAMES.iter().any(|&p| lower == p)
    }

    pub fn classify(name: &str, pid: u32) -> ProcessCategory {
        let lower = name.to_lowercase();
        if Self::is_protected(name, pid, std::process::id()) || lower == "svchost.exe" {
            ProcessCategory::System
        } else if lower.contains("chrome") || lower.contains("edge") || lower.contains("firefox") || lower.contains("brave") {
            ProcessCategory::BrowserEngine
        } else if lower.ends_with("service.exe") || lower.contains("helper") || lower.contains("agent") {
            ProcessCategory::BackgroundService
        } else {
            ProcessCategory::Application
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
    pub category: ProcessCategory,
    pub is_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminateResult {
    pub pid: u32,
    pub process_name: String,
    pub success: bool,
    pub message: String,
}
```

*Depth & Safeguards in `ProcessManager`:*
- When `terminate_process(pid)` is called via Tauri:
  1. Look up process in `sysinfo::System`.
  2. Call `ProcessGuard::is_protected(proc.name(), pid, std::process::id())`.
  3. If true, immediately return `Err(format!("Process '{}' (PID {}) is protected by Windows system safety guardrails and cannot be terminated.", proc.name(), pid))`.
  4. If false, execute `sysinfo::Process::kill()` or `kill_with(Signal::Kill)`.
  5. Return `Ok(TerminateResult { pid, process_name, success: true, message: ... })`.

#### C. Tauri Command Surface (`apps/desktop/src-tauri/src/commands/`)
The IPC surface between React and Rust is kept small and predictable:

| Tauri Command | Input Arguments | Output Type | Description |
| :--- | :--- | :--- | :--- |
| `get_system_snapshot` | None | `Result<SystemSnapshot, String>` | Live CPU, RAM, Disk, OS, finding evaluation (R1) |
| `get_process_list` | None | `Result<Vec<ProcessItem>, String>` | List of processes with CPU, RAM, category, protection flag (R1, R3) |
| `terminate_process` | `pid: u32` | `Result<TerminateResult, String>` | Safe termination with OS blacklist validation (R3) |
| `scan_cleanable_storage` | None | `Result<StorageScanReport, String>` | Scans safe temp directories and returns reclaimable sizes (R2) |
| `clean_storage_targets` | `request: CleanRequest` | `Result<StorageCleanSummary, String>` | Purges selected targets with dry-run support & locked file skip (R2) |
| `query_assistant` | `prompt: String, lang: String` | `Result<AssistantResponse, String>` | Context-aware bilingual assistant recommendations (R4) |

---

## 5. Verification Method

### 5.1 Project Commands
Execute the following commands in order from `d:/HOCC++/pc-doctor`:

```bash
# 1. Verify Rust workspace compiles with zero errors and zero warnings
cargo check --workspace

# 2. Run all unit and integration tests across all workspace crates
cargo test --workspace

# 3. Verify React/TypeScript bundle builds with zero type errors
cd apps/desktop
npm run build
```

### 5.2 Specific Test Cases to Implement for Acceptance Criteria

1. **Storage Scanner & Cleaner Tests** (`crates/pc-doctor-core/src/storage/mod.rs`):
   - `test_scan_reclaimable_returns_non_negative_sizes`: Scans mock temp folder, asserts bytes >= 0 and file count >= 0.
   - `test_storage_cleaner_dry_run`: Executes with `dry_run = true`, asserts `bytes_freed > 0` but files remain on disk.
   - `test_storage_cleaner_locked_file_handling`: Creates an open, locked file in a mock temp directory, executes clean, asserts `files_skipped_locked == 1`, normal files deleted, no panic.
   - `test_path_safety_boundary_rejection`: Attempts to clean `C:\Windows\System32` or `../`, asserts `is_path_safe_to_clean` returns `false` and execution is rejected.

2. **Process Safety Blacklist Tests** (`crates/pc-doctor-core/src/processes/mod.rs`):
   - `test_protected_process_blacklist`: Asserts `is_protected` returns `true` for `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `wininit.exe`, PID 0, PID 4, and self PID.
   - `test_unprotected_process`: Asserts `is_protected` returns `false` for `notepad.exe`, `calculator.exe`, `random_app.exe`.
   - `test_process_categorization`: Asserts `classify` categorizes `svchost.exe` as `System`, `chrome.exe` as `BrowserEngine`, and user apps as `Application`.

3. **Assistant & Diagnostic Tests** (`crates/pc-doctor-core/src/knowledge/mod.rs`):
   - `test_bilingual_knowledge_retrieval`: Asserts search for Vietnamese and English queries return correct articles with full translations.

### 5.3 Invalidation Conditions
This survey and proposal will be invalidated if:
1. `cargo check --workspace` fails to resolve dependencies or introduces new unhandled compilation errors.
2. The hardcoded blacklist fails to cover any of the required protected system processes specified in R3 (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`).
3. Storage cleaning attempts to operate outside the designated Windows safe temporary directories.
