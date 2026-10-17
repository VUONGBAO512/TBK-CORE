# Handoff Report — Requirements & Windows Spec Miner (R1–R4)

**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements`  
**Author**: Specification Miner (Requirements & Windows Specialist)  
**Parent / Recipient**: `d0833960-9ed1-454f-b7d6-be733f742605`  
**Date**: 2026-10-16  
**Milestone**: M1 Spec Mining & Contract Formalization  

---

## 1. Observation

Direct codebase and environmental observations gathered during probing:

1. **Workspace and Crates**:
   - `crates/pc-doctor-core`: Contains `models.rs`, `diagnostics/rules.rs`, `knowledge/mod.rs`, `lib.rs`.
   - `apps/desktop/src-tauri`: Contains `monitoring/collector.rs`, `commands/mod.rs`, `server.rs`, `lib.rs`. Uses Tauri 2.0 with `sysinfo = "0.33"`, `chrono = "0.4"`, `serde = "1"`.
   - `packages/shared-types`: Contains `src/index.ts` declaring frontend mirroring interfaces.
   - `apps/desktop/src`: React 18 + Vite frontend with Kernova design system (`App.tsx`, `ProcessPanel.tsx`, `Dashboard.tsx`, `i18n/{en,vi}.ts`).
2. **Current Telemetry & Rules State**:
   - Telemetry collection exists in `SystemCollector::take_snapshot` (`apps/desktop/src-tauri/src/monitoring/collector.rs:39-158`) collecting CPU, memory, disks, OS, and temperatures.
   - `ProcessInfo` currently models `pid: u32`, `name: String`, `cpu_percent: f32`, `memory_bytes: u64`, `status: String` (`crates/pc-doctor-core/src/models.rs:80-86`). It lacks `exe_path` and `category` ('app' vs 'service').
   - `DiagnosticEngine` (`crates/pc-doctor-core/src/diagnostics/rules.rs:46-273`) evaluates 5 active rules: `SENSOR_STALE_DATA` (>10s), `PERF_CPU_HIGH_SUSTAINED` (>=90% for >=3 samples), `PERF_MEM_HIGH_SUSTAINED` (>=90% for >=3 samples), `PERF_MEM_LOW_AVAILABLE` (<500MB), `DISK_CRITICAL_CAPACITY` (<2%), `DISK_LOW_CAPACITY` (<10%), `SENSOR_TEMP_UNAVAILABLE`, `PERF_UNUSUAL_CPU_PROCESS` (proc >= 80%).
3. **Current Storage & Cleanup State**:
   - `apps/desktop/src/App.tsx:214-277` has a placeholder `StorageAdvisorSection` that only renders drive usage stats and static text advice.
   - **Zero backend scanning or cleaning code currently exists** in `crates/pc-doctor-core` or `apps/desktop/src-tauri/src/commands/mod.rs`.
4. **Current Process Inspection & Termination State**:
   - `apps/desktop/src/components/processes/ProcessPanel.tsx:1-106` displays a read-only table with CPU/RAM sorting and search.
   - **No termination command (`terminate_process`) exists** in `commands/mod.rs`.
   - **No process safety blacklist exists** in the backend or frontend.
5. **Current Offline Assistant State**:
   - `query_assistant` Tauri command exists in `apps/desktop/src-tauri/src/commands/mod.rs:33-140`.
   - Evaluates status questions for live telemetry and queries `KnowledgeBase::search`.
   - Knowledge articles exist for CPU, RAM, Disk, Temperature, and General Health (`crates/pc-doctor-core/src/knowledge/mod.rs:24-87`). Lacks articles on storage cleanup safety and process management safety.
6. **Windows Environment Real-World Probing**:
   - User profile path contains Unicode combining characters: `C:\Users\VuongGiaBaọ̣̣̣\AppData\Local\Temp\`.
   - User `%TEMP%` currently contains 528 files totaling 455,537,570 bytes (~455.5 MB), several locked by active desktop processes.
   - Windows Explorer thumbnail cache exists at `%LOCALAPPDATA%\Microsoft\Windows\Explorer` (15 `thumbcache_*.db` files totaling 91,691,808 bytes), open with active locks by `explorer.exe`.
   - `C:\Windows\Prefetch` exists but returns access denied without elevated administrator privilege.
   - Crash dumps folder `%LOCALAPPDATA%\CrashDumps` does not exist initially on clean systems.

---

## 2. Logic Chain

1. **From Observation 2 to R1 Telemetry & Rules Contract**:
   - Because `ProcessInfo` is missing `exe_path` and `category`, frontend cannot distinguish user applications from background services or display executable provenance.
   - Extending `ProcessInfo` with `exe_path: Option<String>` and `category: ProcessCategory` ('app' | 'service') enables rich process categorization.
   - Telemetry schema in `models.rs` and `packages/shared-types/src/index.ts` must be strictly synchronized to avoid deserialization mismatch across Tauri IPC.
   - The diagnostic rules engine requires exact threshold formulas, deterministic severity tier mapping, and support for critical memory exhaustion (<150 MB), runaway processes (CPU >= 90% or RAM > 2 GB), and healthy state confirmation.

2. **From Observation 3 & 6 to R2 Storage Scanner & Cleaner Contract**:
   - Because temporary files are scattered across designated OS directories (%TEMP%, WER archives, Explorer thumbcache, application logs, Prefetch) and some may not exist initially, the scanner must resolve paths dynamically and handle missing directories without panicking.
   - Because `explorer.exe` and running applications hold exclusive write/delete locks on files in `%TEMP%` and Explorer cache, any clean operation will encounter `ERROR_SHARING_VIOLATION` (code 32) or `ERROR_ACCESS_DENIED` (code 5).
   - Therefore, the cleaner must employ a non-failing skip semantic: catch locked file errors, record them in `skipped_errors`, increment `skipped_files_count`, and continue cleaning.
   - To prevent catastrophic accidental deletion of operating system files or user data, a strict whitelist of target base directories coupled with a hardcoded blacklist of protected directories (`System32`, `Windows`, `Desktop`, `Documents`, `Downloads`, etc.) and path canonicalization (rejecting `..` traversal) is mandatory.

3. **From Observation 4 & 6 to R3 Process Safety Contract**:
   - Attempting to terminate vital Windows processes (`csrss.exe`, `lsass.exe`, `smss.exe`, `wininit.exe`, `services.exe`) immediately crashes Windows (BSOD or unprompted reboot). Terminating `explorer.exe` or `dwm.exe` breaks the desktop graphical environment.
   - Therefore, process termination must pass through a strict verification gate before invoking any OS kill API.
   - If the target PID is <= 4 or the executable name matches any entry in the hardcoded blacklist (case-insensitive), the command must immediately abort and return a structured safety error (`ProcessTerminationError::ProtectedSystemProcess`).

4. **From Observation 5 to R4 Bilingual Assistant Contract**:
   - The desktop app operates 100% locally. The assistant must remain entirely functional without internet connectivity or external API keys.
   - By fusing live telemetry snapshot evaluation with an expanded bilingual keyword-indexed knowledge base, the assistant deterministically answers both real-time status questions ("máy tính có ổn không?", "system health status") and conceptual troubleshooting questions in both Vietnamese (`vi`) and English (`en`).

---

## 3. Caveats

- **Elevation Constraints**: Standard desktop users run without administrator elevation (`UAC`). Locations like `C:\Windows\Prefetch` or other users' temp directories will return `PermissionDenied`. The scanner must treat these as `accessible: false` with 0 bytes cleanable rather than failing the scan.
- **Instantaneous vs Sustained CPU**: A process taking 85% CPU for a 100ms slice is normal. "Runaway" process classification should ideally account for sustained CPU or extreme absolute memory consumption (>2 GB).
- **Session 0 vs User Session**: Distinguishing an 'app' from a 'service' in user mode on Windows can be approximated accurately using executable path (e.g. within `%ProgramFiles%` vs `%SystemRoot%\System32\svchost.exe`) and process name heuristics, since query access to token session IDs or parent process tokens may be restricted for system-level services.

---

## 4. Conclusion & Formal Specifications

Below are the exhaustive specifications, schemas, rules, catalogs, and contracts for R1–R4.

---

### Specification R1: Hardware Telemetry Metrics Schema & Deterministic Diagnostic Rules Engine

#### 1. Hardware Telemetry Metrics Schema (Rust & TypeScript)

```rust
// crates/pc-doctor-core/src/models.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityLevel {
    Informational,
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessCategory {
    App,
    Service,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCoreInfo {
    pub core_id: usize,
    pub usage_percent: f32,       // 0.0 to 100.0
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub model_name: String,
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub global_usage_percent: f32, // 0.0 to 100.0
    pub cores: Vec<CpuCoreInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,       // (used_bytes as f32 / total_bytes as f32) * 100.0
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSnapshot {
    pub name: String,
    pub mount_point: String,       // e.g. "C:" or "C:\"
    pub filesystem: String,        // e.g. "NTFS"
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,           // total_bytes.saturating_sub(available_bytes)
    pub usage_percent: f32,       // (used_bytes as f32 / total_bytes as f32) * 100.0
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsSnapshot {
    pub name: String,             // "Windows"
    pub kernel_version: String,   // e.g. "10.0.26100"
    pub os_version: String,       // e.g. "11 Pro"
    pub host_name: String,
    pub uptime_seconds: u64,      // Continuous system uptime
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureSensor {
    pub label: String,
    pub celsius: Option<f32>,
    pub status: String,           // "available" | "unavailable"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
    pub category: ProcessCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticFinding {
    pub rule_id: String,
    pub severity: SeverityLevel,
    pub title: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub observed_at: DateTime<Utc>,
    pub confidence: f32,          // 0.0 to 1.0
    pub possible_causes: Vec<String>,
    pub next_steps: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub os: OsSnapshot,
    pub temperatures: Vec<TemperatureSensor>,
    pub findings: Vec<DiagnosticFinding>,
}
```

```typescript
// packages/shared-types/src/index.ts (TypeScript Mirror)

export type SeverityLevel = 'informational' | 'low' | 'moderate' | 'high' | 'critical';
export type ProcessCategory = 'app' | 'service';

export interface CpuCoreInfo {
  core_id: number;
  usage_percent: number;
  frequency_mhz: number;
}

export interface CpuSnapshot {
  model_name: string;
  logical_cores: number;
  physical_cores: number;
  global_usage_percent: number;
  cores: CpuCoreInfo[];
}

export interface MemorySnapshot {
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  usage_percent: number;
  swap_total_bytes: number;
  swap_used_bytes: number;
}

export interface DiskSnapshot {
  name: string;
  mount_point: string;
  filesystem: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
  is_removable: boolean;
}

export interface OsSnapshot {
  name: string;
  kernel_version: string;
  os_version: string;
  host_name: string;
  uptime_seconds: number;
}

export interface TemperatureSensor {
  label: string;
  celsius: number | null;
  status: 'available' | 'unavailable';
}

export interface ProcessInfo {
  pid: number;
  name: string;
  exe_path: string | null;
  cpu_percent: number;
  memory_bytes: number;
  status: string;
  category: ProcessCategory;
}

export interface DiagnosticFinding {
  rule_id: string;
  severity: SeverityLevel;
  title: string;
  description: string;
  evidence: string[];
  observed_at: string;
  confidence: number;
  possible_causes: string[];
  next_steps: string[];
  limitations: string[];
}

export interface SystemSnapshot {
  timestamp: string;
  cpu: CpuSnapshot;
  memory: MemorySnapshot;
  disks: DiskSnapshot[];
  os: OsSnapshot;
  temperatures: TemperatureSensor[];
  findings: DiagnosticFinding[];
}
```

#### 2. Deterministic Diagnostic Rules Engine Catalog

The engine evaluates telemetry deterministically on every snapshot tick without probabilistic guesswork.

| Rule ID | Severity | Metric Condition | Confidence | Evidence Format | Actionable Recommendation |
|---|---|---|---|---|---|
| `DISK_CRITICAL_CAPACITY` | **Critical** | Fixed disk `available_bytes / total_bytes < 0.02` (<2% free) | 1.00 | Drive mount point, free GB, total GB, free % | Run disk cleanup immediately; empty Recycle Bin; remove large unused files. |
| `DISK_LOW_CAPACITY` | **Moderate** | Fixed disk `0.02 <= free_ratio < 0.10` (2% to 10% free) | 1.00 | Drive mount point, free GB, total GB, free % | Review installed applications; clean `%TEMP%` cache and downloads. |
| `PERF_CPU_HIGH_SUSTAINED` | **High** | `global_usage_percent >= 90.0` for >= 3 consecutive samples | 0.98 | Last 3 sample percentages | Inspect active process list; terminate non-essential high-load tasks. |
| `PERF_MEM_HIGH_SUSTAINED` | **High** | RAM `usage_percent >= 90.0` for >= 3 consecutive samples | 0.95 | Last 3 sample percentages | Close unused browser tabs or memory-intensive background apps. |
| `PERF_MEM_LOW_AVAILABLE` | **Moderate** | RAM `available_bytes < 500 MB` (524,288,000 bytes) | 0.99 | Available MB vs total RAM MB | Close idle applications; check virtual paging file size. |
| `PERF_MEM_CRITICAL_AVAILABLE`| **Critical** | RAM `available_bytes < 150 MB` AND `usage_percent >= 95.0%` | 1.00 | Available MB, swap utilization | Critical memory pressure; close heavy programs to avoid swapping freeze. |
| `PERF_RUNAWAY_PROCESS` | **High** | Process `cpu_percent >= 90.0%` OR `memory_bytes >= 2 GB` (non-whitelisted) | 0.95 | Process PID, name, CPU %, memory in MB | Review process in Process Inspector; terminate if unresponsive. |
| `PERF_UNUSUAL_CPU_PROCESS` | **Low** | Any single process `cpu_percent >= 80.0%` | 0.90 | PID, name, CPU % | Check if process activity is user-intended (e.g. compilation, rendering). |
| `SENSOR_STALE_DATA` | **Low** | `elapsed_seconds_since_last_sample > 10` | 0.95 | Elapsed seconds (> 10s) | Check system load or background hibernation delays. |
| `SENSOR_TEMP_UNAVAILABLE` | **Informational**| `temperatures.is_empty() || valid_readings == 0` | 1.00 | "0 active readings returned" | Standard Windows user mode restriction; ring-0 kernel driver required. |
| `SYSTEM_HEALTHY` | **Informational**| No findings of severity Low, Moderate, High, or Critical | 1.00 | CPU, RAM, Disk nominal values | System operating normally within all safe parameters. |

---

### Specification R2: Actionable Storage & Temp Cache Scanner & Purge Engine

#### 1. Designated Safe Windows Temporary Locations Catalog

All target paths MUST be resolved dynamically from system environment variables rather than hardcoded usernames.

| Target ID | Category Name | Dynamic Resolution Pattern | Included File Types | Safety Justification |
|---|---|---|---|---|
| `user_temp` | User Temporary Files | `%TEMP%` or `%USERPROFILE%\AppData\Local\Temp` | All files and subdirectories inside | Standard user scratch space created by installers and running applications. |
| `system_temp` | Windows System Temp | `%WINDIR%\Temp` (e.g. `C:\Windows\Temp`) | Files and subfolders inside (excluding folder itself) | Temporary installer files and service logs. If unprivileged, only user-writable files are cleaned. |
| `crash_dumps` | User Crash Dumps | `%LOCALAPPDATA%\CrashDumps` | `*.dmp`, `*.mdmp` | User-mode crash dumps generated by Windows Error Reporting upon application crashes. |
| `wer_reports` | Windows Error Reports | `%LOCALAPPDATA%\Microsoft\Windows\WER\ReportArchive` and `ReportQueue` | Report folders, `*.wer`, `*.dmp` | Archived and queued crash telemetry already processed. |
| `thumbnail_cache` | Windows Thumbnail & Icon Cache | `%LOCALAPPDATA%\Microsoft\Windows\Explorer` | `thumbcache_*.db`, `iconcache_*.db` | Caches automatically regenerated on-demand by Windows Explorer. |
| `application_logs` | User Application Logs | Files ending in `*.log`, `*.etl`, `*.tmp` inside designated temp roots | `*.log`, `*.etl`, `*.tmp` older than 24 hours | Stale diagnostic logs that consume unnecessary drive space. |
| `prefetch` | Windows Prefetch (Admin Only) | `%WINDIR%\Prefetch` | `*.pf` | Windows execution caches. Requires elevation; marked `accessible: false` if unprivileged. |

#### 2. Strictly Protected Locations (Forbidden Target Boundary)

The scanner and cleaner MUST enforce a validation check on every file and directory. Any operation targeting or resolving inside the following locations MUST be immediately rejected:

```rust
pub const PROTECTED_LOCATIONS_BLACKLIST: &[&str] = &[
    "C:\\Windows\\System32",
    "C:\\Windows\\SysWOW64",
    "C:\\Windows\\WinSxS",
    "C:\\Windows\\System",
    "C:\\Windows\\Boot",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\ProgramData",
    "C:\\Users\\Default",
    "C:\\Users\\Public",
    "Desktop",
    "Documents",
    "Pictures",
    "Videos",
    "Music",
    "Downloads",
    "AppData\\Roaming",
    "pagefile.sys",
    "swapfile.sys",
    "hiberfil.sys",
    "bootmgr",
];
```

**Boundary Enforcement Rules**:
1. Canonicalize every candidate path before inspection (`dunce::canonicalize` or `fs::canonicalize`).
2. Verify that the candidate path strictly starts with one of the vetted base directories:
   - Must start with canonicalized `%TEMP%` OR
   - Must start with canonicalized `%WINDIR%\Temp` OR
   - Must start with canonicalized `%LOCALAPPDATA%\CrashDumps` OR
   - Must start with canonicalized `%LOCALAPPDATA%\Microsoft\Windows\WER` OR
   - Must start with canonicalized `%LOCALAPPDATA%\Microsoft\Windows\Explorer` OR
   - Must start with canonicalized `%WINDIR%\Prefetch`.
3. Verify that the candidate path is NOT equal to the root base directory itself (we delete contents, never the base directory).
4. If path traversal (`..`), symbolic link, or junction point points outside the vetted base directory, refuse with `StorageSecurityError::UnauthorizedTargetDirectory`.

#### 3. Storage Scanning & Cleaning Types & Contracts

```rust
// Rust Core Models for Storage Cleaner

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanableCategory {
    pub id: String,                 // "user_temp", "crash_dumps", etc.
    pub name_en: String,            // "User Temporary Files"
    pub name_vi: String,            // "Tệp tạm thời người dùng (%TEMP%)"
    pub description_en: String,
    pub description_vi: String,
    pub target_path: String,        // Resolved base directory
    pub file_count: u64,            // Total cleanable files found
    pub total_bytes: u64,           // Non-negative total reclaimable bytes
    pub is_safe: bool,              // Constant true for vetted targets
    pub accessible: bool,           // True if directory exists and readable
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageScanResult {
    pub scanned_at: DateTime<Utc>,
    pub categories: Vec<CleanableCategory>,
    pub total_cleanable_bytes: u64, // Sum of all categories
    pub total_file_count: u64,      // Sum of all categories
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOptions {
    pub category_ids: Vec<String>,  // Specific categories to clean
    pub dry_run: bool,              // If true, simulate deletions without removing files
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanSummary {
    pub executed_at: DateTime<Utc>,
    pub dry_run: bool,
    pub freed_bytes: u64,           // Non-negative exact bytes deleted
    pub deleted_files_count: u64,   // Number of files deleted (or simulated)
    pub skipped_files_count: u64,   // Files skipped due to locks or permissions
    pub skipped_errors: Vec<String>,// List of skipped file reasons
}
```

```typescript
// packages/shared-types/src/index.ts (TypeScript Storage Types)

export interface CleanableCategory {
  id: string;
  name_en: string;
  name_vi: string;
  description_en: string;
  description_vi: string;
  target_path: string;
  file_count: number;
  total_bytes: number;
  is_safe: boolean;
  accessible: boolean;
}

export interface StorageScanResult {
  scanned_at: string;
  categories: CleanableCategory[];
  total_cleanable_bytes: number;
  total_file_count: number;
}

export interface CleanOptions {
  category_ids: string[];
  dry_run: boolean;
}

export interface CleanSummary {
  executed_at: string;
  dry_run: boolean;
  freed_bytes: number;
  deleted_files_count: number;
  skipped_files_count: number;
  skipped_errors: string[];
}
```

#### 4. Locked/Open File Skip Semantics & Algorithm

```rust
// Purge Loop Execution Contract
for entry in target_files {
    let size = match entry.metadata() {
        Ok(m) => m.len(),
        Err(_) => 0,
    };

    if options.dry_run {
        summary.freed_bytes += size;
        summary.deleted_files_count += 1;
        continue;
    }

    match std::fs::remove_file(&entry) {
        Ok(_) => {
            summary.freed_bytes += size;
            summary.deleted_files_count += 1;
        }
        Err(e) => {
            // OS Error 32: ERROR_SHARING_VIOLATION (file is locked/open)
            // OS Error 5: ERROR_ACCESS_DENIED
            summary.skipped_files_count += 1;
            summary.skipped_errors.push(format!(
                "Skipped '{}': {}", 
                entry.display(), 
                e
            ));
            // CONTINUES WITHOUT PANICKING OR HALTING
        }
    }
}
```

---

### Specification R3: Safe Process Inspection & Resource Management

#### 1. Process Listing & Categorization Schema

- **`pid`**: `u32` — Windows Process Identifier.
- **`name`**: `String` — Name of process binary (e.g. `chrome.exe`, `svchost.exe`).
- **`exe_path`**: `Option<String>` — Full absolute path to executable.
- **`cpu_percent`**: `f32` — Normalized CPU utilization percentage (0.0 to 100.0).
- **`memory_bytes`**: `u64` — Working set memory footprint in bytes.
- **`status`**: `String` — Operating status string (`"Run"`, `"Sleep"`, `"Idle"`).
- **`category`**: `ProcessCategory` — Categorization into `'app'` vs `'service'`:
  - **`app`**: Applications running in active user interactive session (e.g. containing user profile in path, `Program Files` interactive binaries, or having a window title).
  - **`service`**: System background processes, host daemons (`svchost.exe`, `services.exe`), services located in `C:\Windows\System32`, or executing under Session 0.

#### 2. Runaway Process Detection Criteria

A process is flagged as **Runaway** (`is_runaway: true` and diagnostic alert `PERF_RUNAWAY_PROCESS`) if it satisfies any of the following deterministic criteria:
1. **CPU Runaway**: Non-system process with `cpu_percent >= 90.0%`.
2. **Memory Runaway**: Non-system process with `memory_bytes >= 3,221,225,472` (3 GB) or exceeding 35% of total system RAM.
3. **Dual Pressure Runaway**: Non-system process with `cpu_percent >= 70.0%` AND `memory_bytes >= 1,610,612,736` (1.5 GB).

#### 3. Process Safety Blacklist & Termination Guardrails

The blacklist contains all core Windows NT executive components, security authorities, and session managers. **Under no circumstances may TBK Core attempt to terminate these processes.**

```rust
pub const PROTECTED_SYSTEM_PROCESSES: &[(&str, &str)] = &[
    ("system", "Windows NT Kernel Process"),
    ("system idle process", "CPU Idle Loop Thread"),
    ("registry", "Windows Registry Subsystem"),
    ("smss.exe", "Session Manager Subsystem (Terminating causes instant bugcheck)"),
    ("csrss.exe", "Client Server Runtime Subsystem (Terminating triggers BSOD CRITICAL_PROCESS_DIED)"),
    ("wininit.exe", "Windows Initialization Process"),
    ("services.exe", "Service Control Manager (Essential daemon host)"),
    ("lsass.exe", "Local Security Authority Subsystem (Terminating triggers forced system restart)"),
    ("winlogon.exe", "Windows Logon Process"),
    ("dwm.exe", "Desktop Window Manager (Compositing display pipeline)"),
    ("explorer.exe", "Windows Shell & Taskbar"),
    ("svchost.exe", "Generic Host Process for Windows Services"),
    ("fontdrvhost.exe", "User-Mode Font Driver Host"),
    ("conhost.exe", "Console Window Host"),
    ("spoolsv.exe", "Print Spooler Service"),
    ("taskhostw.exe", "Host Process for Windows Tasks"),
    ("sihost.exe", "Shell Infrastructure Host"),
    ("securityhealthservice.exe", "Windows Defender Security Center Service"),
    ("msmpeng.exe", "Microsoft Defender Antivirus Core Engine"),
    ("tbk_core.exe", "TBK Core Application (Self-termination guard)"),
    ("pc-doctor.exe", "TBK Core Binary (Self-termination guard)"),
];
```

#### 4. Structured Safety Error Contract

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminateProcessResponse {
    pub success: bool,
    pub pid: u32,
    pub message: String,
    pub error: Option<ProcessTerminationError>,
}
```

```typescript
// packages/shared-types/src/index.ts (TypeScript Mirror)

export type ProcessTerminationError =
  | { error_type: 'ProtectedSystemProcess'; details: { pid: number; name: string; reason: string } }
  | { error_type: 'ProcessNotFound'; details: { pid: number } }
  | { error_type: 'AccessDenied'; details: { pid: number; name: string; os_error_code: number; message: string } }
  | { error_type: 'InternalError'; details: { message: string } };

export interface TerminateProcessResponse {
  success: boolean;
  pid: number;
  message: string;
  error?: ProcessTerminationError;
}
```

**Evaluation Logic Before Calling OS Kill API**:
```rust
// In terminate_process command:
if pid <= 4 {
    return Ok(TerminateProcessResponse {
        success: false,
        pid,
        message: "Cannot terminate core system process".to_string(),
        error: Some(ProcessTerminationError::ProtectedSystemProcess {
            pid,
            name: "System".to_string(),
            reason: "Windows kernel PID <= 4 is strictly protected".to_string(),
        }),
    });
}

let name_lower = process_name.to_lowercase();
if let Some((_, reason)) = PROTECTED_SYSTEM_PROCESSES.iter().find(|(p, _)| *p == name_lower) {
    return Ok(TerminateProcessResponse {
        success: false,
        pid,
        message: format!("Process '{}' is protected by system guardrails", process_name),
        error: Some(ProcessTerminationError::ProtectedSystemProcess {
            pid,
            name: process_name,
            reason: reason.to_string(),
        }),
    });
}
```

---

### Specification R4: Offline Assistant & Bilingual Guidance

#### 1. 100% Offline Architecture & Privacy Guarantee
- The assistant is purely compiled into the native desktop binary.
- It performs **ZERO** network requests, calls **NO** remote LLM endpoints, and creates **NO** cloud telemetry traffic.
- Operates instantaneously with zero latency and complete privacy.

#### 2. Two-Tier Query Dispatch Flow

```
[User Query + Language ("vi" | "en")]
          │
          ▼
Is prompt asking for system status/health?
(Keywords: "status", "health", "sức khỏe", "tình trạng", "kiểm tra", "đầy ổ", "chậm")
    ├── YES ──► Tier 1: Real-time Live Telemetry + Diagnostic Finding Evaluator
    │                     └─► Generates real-time report with active metrics & actionable fixes
    └── NO  ──► Tier 2: Keyword-Indexed Knowledge Base Search
                          ├─► Match Found: Return bilingual article content
                          └─► No Match: Return deterministic bilingual companion fallback
```

#### 3. Expanded Bilingual Knowledge Base Catalog

| Article ID | Category | Keywords (VI & EN) | Title (EN / VI) | Core Content Summary |
|---|---|---|---|---|
| `cpu_troubleshooting` | CPU | cpu, processor, quá tải, chíp, nóng máy, 100% cpu, lag, giật, đơ | CPU Troubleshooting & High Usage / Khắc phục CPU hoạt động tải cao | Step-by-step guidance on identifying heavy processes, background antivirus spikes, cooling, thermal throttling limits. |
| `ram_troubleshooting` | Memory | ram, memory, bộ nhớ, tràn ram, hết ram, out of memory | Memory & RAM Optimization / Tối ưu và giải phóng bộ nhớ RAM | Instructions on managing browser tabs, understanding Windows memory caching, Paging File settings, and physical upgrades. |
| `disk_troubleshooting`| Storage | disk, ổ đĩa, ổ cứng, dung lượng, storage, đầy ổ, c: | Disk Free Space & Storage Health / Giải phóng dung lượng & Sức khỏe ổ đĩa | Guidelines on maintaining >= 15% free space on C:, using TBK Core storage cleaner, emptying Recycle Bin, cleanmgr. |
| `storage_cleaner_guide`| Cleanup | dọn dẹp, clean, temp, rác, xóa file, delete, storage cleaner | Safe Temporary Storage Cleaning Guide / Hướng dẫn dọn dẹp bộ nhớ đệm an toàn | Explains why %TEMP% and crash dumps can be safely deleted, why open files are skipped, and why system directories are protected. |
| `process_safety_guide`| Process | process, tiến trình, tắt ứng dụng, terminate, kill, tắt tác vụ | Safe Process Management & Guardrails / Quản lý và dừng tiến trình an toàn | Guidance on differentiating user apps from background services; explains why system processes (csrss, lsass) are protected. |
| `temperature_info` | Thermal | temperature, nhiệt độ, nóng, sensor, cảm biến, quạt, fan | Hardware Temperature & Thermal Monitoring / Nhiệt độ phần cứng & Cảm biến nhiệt | Clarifies Windows user-mode limitations regarding ACPI thermal zones; explains safe temperature bands (idle 35-50°C, load 65-85°C). |
| `general_health` | Maintenance | sức khỏe, health, bảo trì, tối ưu, optimize, nhanh hơn, doctor, bác sĩ | TBK Core Routine Maintenance Guide / Cẩm nang bảo trì định kỳ cùng TBK Core | Weekly restart recommendations, startup item minimization in Task Manager, routine checks using TBK Core dashboard. |

#### 4. Deterministic Telemetry & Fallback Response Templates

**Vietnamese (`vi`) Live Health Status Format**:
```markdown
**Báo cáo sức khỏe máy tính hiện tại:**

- **CPU**: Model {cpu.model_name} ({cpu.global_usage_percent:.1f}% đang sử dụng trên {cpu.logical_cores} luồng)
- **RAM**: {mem.used_gb:.1f} GB / {mem.total_gb:.1f} GB ({mem.usage_percent:.1f}%)
- **Ổ đĩa chính**: {primary_disk.mount_point} ({primary_disk.usage_percent:.1f}% đã dùng)
- **Thời gian hoạt động**: {uptime_hours} giờ

{if findings.is_empty()}
✅ Hệ thống của bạn đang hoạt động trong ngưỡng an toàn ổn định!
{else}
⚠️ Phát hiện **{findings.len()} vấn đề/cảnh báo** cần lưu ý:
- **{finding.title}**: {finding.description}
  * Khắc phục: {finding.next_steps[0]}
{endif}
```

**English (`en`) Live Health Status Format**:
```markdown
**Current Computer Health Status:**

- **CPU**: {cpu.model_name} ({cpu.global_usage_percent:.1f}% usage across {cpu.logical_cores} logical cores)
- **Memory**: {mem.used_gb:.1f} GB / {mem.total_gb:.1f} GB ({mem.usage_percent:.1f}% used)
- **Primary Disk**: {primary_disk.mount_point} ({primary_disk.usage_percent:.1f}% used)
- **System Uptime**: {uptime_hours} hours

{if findings.is_empty()}
✅ Your system is running within optimal performance parameters!
{else}
⚠️ Detected **{findings.len()} diagnostic alert(s)**:
- **{finding.title}**: {finding.description}
  * Action: {finding.next_steps[0]}
{endif}
```

**Bilingual Fallback Templates** (when query is unrecognized):
- **Vietnamese (`vi`)**:
  > "Tôi là trợ lý **TBK Core** chạy hoàn toàn trên máy tính của bạn.\nBạn có thể hỏi tôi về:\n- Tình trạng máy tính / kiểm tra CPU, RAM, ổ đĩa\n- Dọn dẹp tệp tạm thời an toàn và giải phóng dung lượng\n- Kiểm tra tiến trình đang tiêu tốn nhiều tài nguyên\n- Hướng dẫn bảo trì máy tính định kỳ"
- **English (`en`)**:
  > "I am your native **TBK Core** assistant operating completely offline.\nYou can ask me about:\n- System health overview & CPU/RAM/Disk stats\n- Safe temporary file cleanup and freeing disk space\n- Inspecting heavy background processes\n- Routine computer optimization practices"

---

## 5. Tauri IPC Command Signatures

To expose the specified functionality to the React desktop interface, the backend Tauri invoke handler must declare and implement these exact signatures:

```rust
// apps/desktop/src-tauri/src/commands/mod.rs

// 1. Telemetry
#[tauri::command]
pub fn get_system_snapshot(state: State<'_, Arc<CollectorState>>) -> Result<SystemSnapshot, String>;

// 2. Processes
#[tauri::command]
pub fn get_process_list(state: State<'_, Arc<CollectorState>>) -> Result<Vec<ProcessInfo>, String>;

#[tauri::command]
pub fn terminate_process(
    pid: u32, 
    process_name: String, 
    state: State<'_, Arc<CollectorState>>
) -> Result<TerminateProcessResponse, String>;

// 3. Storage Scanner & Cleaner
#[tauri::command]
pub fn scan_storage() -> Result<StorageScanResult, String>;

#[tauri::command]
pub fn clean_storage(options: CleanOptions) -> Result<CleanSummary, String>;

// 4. Offline Assistant
#[tauri::command]
pub fn query_assistant(
    prompt: String, 
    lang: String, 
    state: State<'_, Arc<CollectorState>>
) -> Result<AssistantResponse, String>;
```

---

## Features Discovered

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|---|---|---|---|---|---|---|
| 1 | Telemetry | CPU Snapshot & Per-Core Telemetry | Captures global usage %, model brand, logical/physical core counts, and per-core usage & clock frequencies. | None | `CpuSnapshot` | If brand is missing, returns "Unknown CPU". Core count falls back to logical. | `collector.rs:49-75`, `models.rs:25-39` |
| 2 | Telemetry | Physical & Swap Memory Breakdown | Captures total, used, available memory and swap file metrics with exact byte counts. | None | `MemorySnapshot` | If total is 0, usage percent returns 0.0 without division by zero. | `collector.rs:77-94`, `models.rs:41-49` |
| 3 | Telemetry | Disk Volume Telemetry | Enumerate all mounted fixed and removable volumes with filesystem, total, used, and free bytes. | None | `Vec<DiskSnapshot>` | Unmounted volumes skipped; used bytes uses `saturating_sub`. | `collector.rs:96-118`, `models.rs:51-61` |
| 4 | Telemetry | OS Platform & Uptime Tracking | Measures system uptime in seconds, kernel build, and Windows version. | None | `OsSnapshot` | Fallback to "Windows" and "Unknown" if sysinfo cannot detect. | `collector.rs:120-127`, `models.rs:63-70` |
| 5 | Diagnostics | Sustained Metric History Buffer | Tracks rolling window of 10 samples to evaluate sustained resource stress over time. | CPU %, RAM % | Updated `DiagnosticHistory` | Window length capped at 10 items. | `rules.rs:4-34` |
| 6 | Diagnostics | Critical & Low Disk Space Detection | Emits `DISK_CRITICAL_CAPACITY` (<2%) and `DISK_LOW_CAPACITY` (<10%) findings. | `Vec<DiskSnapshot>` | `Vec<DiagnosticFinding>` | Skips zero-capacity volumes safely. | `rules.rs:166-220` |
| 7 | Diagnostics | Sustained High CPU & RAM Alerts | Flags `PERF_CPU_HIGH_SUSTAINED` and `PERF_MEM_HIGH_SUSTAINED` when >=90% for >=3 ticks. | History buffer | `Vec<DiagnosticFinding>` | Requires minimum 3 samples in buffer before triggering. | `rules.rs:92-140` |
| 8 | Diagnostics | Thermal Sensor State Reporter | Reports `SENSOR_TEMP_UNAVAILABLE` explaining standard user-mode ACPI restrictions. | `Vec<TemperatureSensor>`| `Vec<DiagnosticFinding>` | Triggers cleanly when no thermal sensors expose readings. | `rules.rs:222-242` |
| 9 | Storage | Multi-Directory Temp Cache Scanner | Scans `%TEMP%`, `CrashDumps`, Explorer thumbnail cache, logs, and Prefetch for cleanable targets. | None | `StorageScanResult` | Non-existent folders return 0 bytes, 0 files; unreadable folders marked `accessible: false`. | Windows spec probe & PowerShell validation |
| 10 | Storage | Dry-Run Simulation Mode | Calculates exact reclaimable bytes and file count without removing any files from disk. | `CleanOptions { dry_run: true }` | `CleanSummary` | Zero deletions executed on disk. | R2 spec requirement & cleaner engine |
| 11 | Storage | Locked File Skip Purge Engine | Safely skips files in-use by running applications during purge, recording path & OS error. | `CleanOptions` | `CleanSummary` | Sharing violation/access denied handled without halting clean loop. | Windows filesystem probe on `thumbcache_*.db` and `%TEMP%` |
| 12 | Storage | Protected Location Boundary Guard | Strictly blocks any clean operation targeting `%WINDIR%`, `System32`, `Desktop`, `Documents`, etc. | File candidate path | `Result<(), StorageSecurityError>` | Immediately aborts with security error if path canonicalizes outside safe root. | Security audit boundary |
| 13 | Processes | Process Inspection & Sorting | Retrieves active processes sorted by CPU descending or RAM descending with PID and status. | Limit count | `Vec<ProcessInfo>` | Unnamed processes handled gracefully with lossy string fallback. | `collector.rs:160-187` |
| 14 | Processes | Application vs Background Service Classifier | Categorizes processes into `'app'` (user interactive) or `'service'` (background daemon/host). | `ProcessInfo` | Tagged `category` | Heuristic fallback to `'service'` if executable path is inside `System32`. | R3 specification requirement |
| 15 | Processes | Runaway Process Detection | Identifies processes exceeding healthy resource thresholds (CPU >= 90% or RAM > 2GB). | `ProcessInfo` | `is_runaway: bool` | System processes excluded from runaway termination flags. | R3 specification requirement |
| 16 | Processes | System Process Termination Guard | Hardcoded blacklist protecting vital Windows OS processes (`csrss.exe`, `lsass.exe`, `services.exe`, etc.). | `pid`, `name` | `TerminateProcessResponse` | Returns structured safety error `ProtectedSystemProcess` without attempting kill. | Acceptance criteria & Windows kernel stability |
| 17 | Assistant | 100% Offline Query Evaluator | Evaluates user prompts completely locally with zero external network or cloud transfers. | Prompt, Lang | `AssistantResponse` | No internet connection required; completely offline. | R4 specification requirement |
| 18 | Assistant | Telemetry-Aware Health Reporting | Summarizes live CPU, RAM, disk, and diagnostic findings into natural language reports in `vi` or `en`. | Status prompt | Structured report string | If findings empty, returns healthy confirmation badge. | `commands/mod.rs:60-119` |
| 19 | Assistant | Bilingual Keyword Knowledge Engine | Fast token and keyword matching across troubleshooting articles in Vietnamese and English. | Troubleshooting prompt | Matching article | Fallback to general guidance template if no keyword matched. | `knowledge/mod.rs:89-100` |

---

## Edge Cases

| # | Feature | Input / Condition | Observed / Documented Behavior |
|---|---|---|---|
| 1 | Path Resolution | Windows username with Unicode combining diacritics (`VuongGiaBaọ̣̣̣`) | Path strings must use standard UTF-8 / UTF-16 conversions (`PathBuf`); PowerShell CLI string interpolation may fail if unquoted, but Rust `std::env::var("TEMP")` and `dunce::canonicalize` resolve accurately. |
| 2 | Storage Scanner | Directory does not exist on disk (e.g. `%LOCALAPPDATA%\CrashDumps` on a machine with no crashes) | Returns `CleanableCategory` with `file_count: 0`, `total_bytes: 0`, `accessible: false` without crashing, panicking, or returning errors. |
| 3 | Storage Scanner | Directory has restricted ACLs (e.g. `C:\Windows\Prefetch` for unprivileged standard users) | Reading directory fails with `std::io::ErrorKind::PermissionDenied`. Scanner intercepts error and marks `accessible: false`, `file_count: 0`, `total_bytes: 0`. |
| 4 | Storage Cleaner | Active locked file (e.g. `thumbcache_*.db` mapped by `explorer.exe` or temporary log held open by Chrome) | `remove_file` returns OS Error 32 (`ERROR_SHARING_VIOLATION`) or OS Error 5 (`ERROR_ACCESS_DENIED`). Cleaner catches error, increments `skipped_files_count`, records path in `skipped_errors`, and proceeds to next file. |
| 5 | Storage Cleaner | Directory containing unremovable locked files | Cleaner purges cleanable files; directory removal (`remove_dir`) is skipped if non-empty; no panic. |
| 6 | Storage Cleaner | Dry-run enabled (`dry_run: true`) | Iterates all targets, computes exact `freed_bytes` and `deleted_files_count`, but executes zero `remove_file` calls on disk. |
| 7 | Storage Boundary | Path traversal attempt (e.g. `%TEMP%\..\..\Windows\System32\cmd.exe`) | Path canonicalization (`dunce::canonicalize`) resolves real target; fails boundary check `starts_with(safe_temp_dir)`; rejected immediately. |
| 8 | Process Terminate | Termination request for `csrss.exe`, `lsass.exe`, `services.exe`, or `smss.exe` | Intercepted by blacklist check before OS kill call; returns `success: false`, `error_type: "ProtectedSystemProcess"` with descriptive explanation. |
| 9 | Process Terminate | Termination request for PID 0 (Idle) or PID 4 (System) | Intercepted by PID guard (`pid <= 4`); returns `ProtectedSystemProcess` error immediately. |
| 10 | Process Terminate | Target process terminated right before call (PID no longer exists) | Process lookup returns `None`; returns `ProcessTerminationError::ProcessNotFound`. |
| 11 | Process Terminate | Non-blacklisted process owned by another user or SYSTEM running without write access | OS kill returns access denied; returns `ProcessTerminationError::AccessDenied` with OS error code. |
| 12 | Diagnostics | Sampling lag > 10 seconds (system sleep / hibernation / heavy CPU throttle) | `DiagnosticEngine` detects `elapsed_sec > 10` and generates `SENSOR_STALE_DATA` finding. |
| 13 | Diagnostics | Zero total disk bytes or 100% full drive | Division by zero avoided by condition `if disk.total_bytes > 0`; `used_bytes` computed via `saturating_sub`. |
| 14 | Assistant | Query in uppercase, mixed diacritics, or punctuation (e.g. "MÁY TÍNH CÓ ỔN KHÔNG???") | Case-normalized to lowercase; diacritic-aware substrings match "ổn không" and "máy tính" -> invokes live telemetry health summary. |
| 15 | Assistant | Query with no keyword match (e.g. "hãy làm thơ cho tôi") | Returns deterministic bilingual companion intro listing valid topics (health check, storage clean, process management). |

---

## 6. Verification Method

To verify these specifications independently:

1. **Rust Core Contracts & Tests**:
   - Inspect `crates/pc-doctor-core/src/models.rs` and `crates/pc-doctor-core/src/diagnostics/rules.rs`.
   - Run `cargo test --workspace` to verify rule engine unit tests and knowledge base search tests pass.
2. **Frontend Type Alignment**:
   - Inspect `packages/shared-types/src/index.ts`.
   - Run `npm run build` in `apps/desktop` to verify TypeScript compile-time type safety.
3. **Safety Blacklist Verification**:
   - Test `terminate_process` with PID 4 or name `"csrss.exe"`. Verify return matches `ProcessTerminationError::ProtectedSystemProcess`.
4. **Storage Cleaner Locked File & Non-Negative Byte Verification**:
   - Scan storage targets via `scan_storage`. Verify all `total_bytes` are non-negative `u64`.
   - Execute dry run via `clean_storage` with `dry_run: true`. Verify zero files are deleted.
   - Execute purge on `%TEMP%`. Verify open locked files are gracefully skipped and recorded in `skipped_errors` with zero panics or crashes.
