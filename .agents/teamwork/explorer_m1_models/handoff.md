# Handoff Report: Milestone 1 Models, Diagnostics & Knowledge Base

**Agent**: Explorer M1 Models (`explorer_m1_models`)  
**Parent / Caller**: `d0833960-9ed1-454f-b7d6-be733f742605`  
**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_models`  
**Date**: 2026-10-16  

---

## 1. Observation

### 1.1 Workspace Profile Warning
Executing `cargo check --workspace` in `d:/HOCC++/pc-doctor`:
```
warning: profiles for the non root package will be ignored, specify profiles at the workspace root:
package:   D:\HOCC++\pc-doctor\apps\desktop\src-tauri\Cargo.toml
workspace: D:\HOCC++\pc-doctor\Cargo.toml
```
- In `d:/HOCC++/pc-doctor/apps/desktop/src-tauri/Cargo.toml` lines 34-40:
  ```toml
  # Read the optimization guideline for more details: https://tauri.app/concept/size/#cargo-configuration
  [profile.release]
  codegen-units = 1
  lto = true
  opt-level = 3
  panic = "abort"
  strip = true
  ```
- In root `d:/HOCC++/pc-doctor/Cargo.toml` lines 1-7:
  ```toml
  [workspace]
  resolver = "2"
  members = [
      "crates/pc-doctor-core",
      "apps/desktop/src-tauri",
  ]
  ```
  The root `Cargo.toml` does not define `[profile.release]`, causing Cargo to emit the warning on every build/check command.

### 1.2 Core Domain Models (`crates/pc-doctor-core/src/models.rs`)
- Current `models.rs` (112 lines) contains:
  `SeverityLevel`, `CompanionMood`, `CpuCoreInfo`, `CpuSnapshot`, `MemorySnapshot`, `DiskSnapshot`, `OsSnapshot`, `TemperatureSensor`, `ProcessInfo`, `DiagnosticFinding`, `SystemSnapshot`.
- It lacks the storage engine models specified in `PROJECT.md` Interface Contract 1:
  - `CleanableTarget` (target identifier, localized names, path, file count, total bytes, is_safe flag)
  - `StorageScanReport` (list of targets, total reclaimable bytes, total files, scanned_at timestamp)
  - `CleanRequest` (list of target IDs to clean, dry_run boolean)
  - `StorageCleanSummary` (dry_run flag, bytes freed, files deleted, locked files skipped, targets processed, error messages)
- It lacks the process inspection models specified in `PROJECT.md` Interface Contract 2:
  - `ProcessCategory` enum (System, BackgroundService, Application, BrowserEngine)
  - `ProcessItem` struct (pid, name, cpu_percent, memory_bytes, status, category, is_protected)
  - `TerminateResult` struct (pid, process_name, success, message)
- It also lacks `AssistantResponse` struct, which is currently locally defined in `apps/desktop/src-tauri/src/commands/mod.rs:26-30`.

### 1.3 Knowledge Base (`crates/pc-doctor-core/src/knowledge/mod.rs`)
- Current `KnowledgeBase` (106 lines) contains 5 articles:
  - `cpu_troubleshooting`
  - `ram_troubleshooting`
  - `disk_troubleshooting`
  - `temperature_info`
  - `general_health`
- It lacks dedicated articles addressing:
  - Storage cleanup safety, safe temporary directories vs protected Windows paths, and dry-run preview guarantees.
  - Process management safety, the hardcoded Windows OS safety blacklist (`csrss.exe`, `lsass.exe`, `services.exe`, `smss.exe`, `explorer.exe`), and runaway process triage.
- The `KnowledgeBase::search` method currently uses plain substring matching on query without relevance scoring or direct ID lookup (`get_by_id`).

### 1.4 Core Library Root (`crates/pc-doctor-core/src/lib.rs`)
- Current `lib.rs` (108 lines) exports:
  `pub mod diagnostics; pub mod knowledge; pub mod models;`
  `pub use diagnostics::{DiagnosticEngine, DiagnosticHistory};`
  `pub use knowledge::{KnowledgeArticle, KnowledgeBase};`
  `pub use models::*;`
- Currently contains 3 tests:
  - `test_diagnostic_engine_detects_low_disk`
  - `test_diagnostic_sustained_cpu`
  - `test_knowledge_base_search`
- There are zero unit tests for storage models, process models, serialization/deserialization schemas, or the new knowledge base articles.

---

## 2. Logic Chain

1. **Workspace Cargo Profile Warning Resolution**:
   - As observed in §1.1, Cargo enforces that compiler profile tables (`[profile.*]`) must reside in the workspace root manifest (`pc-doctor/Cargo.toml`) rather than member manifests.
   - Moving `[profile.release]` with `codegen-units = 1`, `lto = true`, `opt-level = 3`, `panic = "abort"`, and `strip = true` to the root `Cargo.toml` and removing it from `apps/desktop/src-tauri/Cargo.toml` preserves identical release optimization for desktop binaries while eliminating the warning completely across `cargo check --workspace` and `cargo test --workspace`.

2. **Domain Models Expansion**:
   - As observed in §1.2 and `PROJECT.md` Interface Contracts 1 & 2:
     - `CleanableTarget` requires non-negative byte calculations, bilingual names (`name_en`, `name_vi`), descriptions, and safety status.
     - `StorageScanReport` bundles targets with `scanned_at: DateTime<Utc>` and totals.
     - `CleanRequest` and `StorageCleanSummary` govern both dry-run simulation and physical purging with locked-file counts and error tracking.
     - `ProcessCategory` must serialize cleanly to JSON (`snake_case` with aliases for resilience: `system`, `background_service`, `application`, `browser_engine`).
     - `ProcessItem` enriches telemetry with categorization and `is_protected` status. Providing `From<ProcessItem> for ProcessInfo` preserves backwards compatibility with `DiagnosticEngine::evaluate`.
     - `TerminateResult` standardizes the response for process termination attempts.
     - `AssistantResponse` placed in `pc-doctor-core` establishes a single source of truth for offline assistant IPC and telemetry.

3. **Bilingual Knowledge Base Expansion**:
   - As observed in §1.3 and `ORIGINAL_REQUEST.md` R4:
     - Storage cleanup requires explicit user guidance on which directories are safe (`%TEMP%`, WER Crash Dumps, App Logs, Thumbnail Cache) vs strictly protected (`C:\Windows`, `System32`, personal folders), why dry-run is safe, and how locked files are handled.
     - Process management requires explicit documentation of the hardcoded safety blacklist (`csrss.exe`, `lsass.exe`, `services.exe`, `smss.exe`, `explorer.exe`), runaway thresholds (>80% CPU, RAM leaks), and safe termination practices.
     - Adding `storage_cleanup_safety` and `process_management_safety` articles with rich bilingual content (VI & EN) and comprehensive keyword indices ensures `query_assistant` can guide users accurately offline.
     - Adding `get_by_id(&self, id: &str) -> Option<&KnowledgeArticle>` enables deterministic article retrieval.

4. **Module Exports and Unit Test Suite**:
   - As observed in §1.4, clean re-exports via `pub use models::*` and `pub use knowledge::*` expose all new models directly to dependents (`desktop_lib` and tests).
   - Adding unit tests for model serialization/deserialization, conversion between `ProcessItem` and `ProcessInfo`, `ProcessCategory` serde aliases, and Vietnamese/English knowledge base queries guarantees regression-free contracts before Milestone 2 starts.

---

## 3. Caveats

- **Read-Only Explorer Scope**: In accordance with the system constraints, this report provides the exact blueprint and code diffs. Implementation is executed by the implementer agent.
- **`crates/pc-doctor-core/src/storage/` and `src/processes/` Engines**: In Milestone 1, specific scanning engines (e.g. disk walking, `sysinfo` process blacklist check) will consume the models defined here. Those engines are to be implemented either alongside or directly following these models.
- **Frontend Type Sync**: While TypeScript definitions in `packages/shared-types/src/index.ts` and `apps/desktop/src/types.ts` should be updated in lockstep, `cargo check` and `cargo test` remain completely functional independently of the frontend build.

---

## 4. Conclusion & Implementation Blueprint

The exact code changes to be applied across the 4 targets are specified below.

### 4.1 Target 1: `d:/HOCC++/pc-doctor/Cargo.toml`
Add `[profile.release]` configuration at the workspace root:

```toml
[workspace]
resolver = "2"
members = [
    "crates/pc-doctor-core",
    "apps/desktop/src-tauri",
]

# Release optimizations for production desktop builds
[profile.release]
codegen-units = 1
lto = true
opt-level = 3
panic = "abort"
strip = true
```

### 4.2 Target 2: `d:/HOCC++/pc-doctor/apps/desktop/src-tauri/Cargo.toml`
Remove lines 34-40:
```toml
<<<< REMOVE:
# Read the optimization guideline for more details: https://tauri.app/concept/size/#cargo-configuration
[profile.release]
codegen-units = 1
lto = true
opt-level = 3
panic = "abort"
strip = true
>>>>
```

### 4.3 Target 3: `d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/models.rs`
Replace file content with the aligned model definitions:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
pub enum CompanionMood {
    Idle,
    Thinking,
    Healthy,
    Warning,
    Critical,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpuCoreInfo {
    pub core_id: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpuSnapshot {
    pub model_name: String,
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub global_usage_percent: f32,
    pub cores: Vec<CpuCoreInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskSnapshot {
    pub name: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f32,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsSnapshot {
    pub name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub host_name: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemperatureSensor {
    pub label: String,
    pub celsius: Option<f32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticFinding {
    pub rule_id: String,
    pub severity: SeverityLevel,
    pub title: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub observed_at: DateTime<Utc>,
    pub confidence: f32,
    pub possible_causes: Vec<String>,
    pub next_steps: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub os: OsSnapshot,
    pub temperatures: Vec<TemperatureSensor>,
    pub findings: Vec<DiagnosticFinding>,
}

// ==========================================
// Storage Engine Models (Interface Contract 1)
// ==========================================

/// Itemized cleanable target representing a known safe directory or cache type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanableTarget {
    pub id: String,
    pub name_en: String,
    pub name_vi: String,
    pub description_en: String,
    pub description_vi: String,
    pub root_path: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub is_safe: bool,
}

impl CleanableTarget {
    pub fn new(
        id: impl Into<String>,
        name_en: impl Into<String>,
        name_vi: impl Into<String>,
        description_en: impl Into<String>,
        description_vi: impl Into<String>,
        root_path: impl Into<String>,
        file_count: usize,
        total_bytes: u64,
        is_safe: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name_en: name_en.into(),
            name_vi: name_vi.into(),
            description_en: description_en.into(),
            description_vi: description_vi.into(),
            root_path: root_path.into(),
            file_count,
            total_bytes,
            is_safe,
        }
    }
}

/// Comprehensive scan report produced by the storage scanner
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageScanReport {
    pub targets: Vec<CleanableTarget>,
    pub total_reclaimable_bytes: u64,
    pub total_files: usize,
    pub scanned_at: DateTime<Utc>,
}

impl StorageScanReport {
    pub fn empty() -> Self {
        Self {
            targets: Vec::new(),
            total_reclaimable_bytes: 0,
            total_files: 0,
            scanned_at: Utc::now(),
        }
    }

    pub fn new(targets: Vec<CleanableTarget>) -> Self {
        let total_reclaimable_bytes = targets.iter().map(|t| t.total_bytes).sum();
        let total_files = targets.iter().map(|t| t.file_count).sum();
        Self {
            targets,
            total_reclaimable_bytes,
            total_files,
            scanned_at: Utc::now(),
        }
    }
}

/// Request to perform storage cleanup (or dry-run purge simulation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanRequest {
    pub target_ids: Vec<String>,
    pub dry_run: bool,
}

impl CleanRequest {
    pub fn new(target_ids: Vec<String>, dry_run: bool) -> Self {
        Self {
            target_ids,
            dry_run,
        }
    }
}

/// Outcome summary returned after executing a storage clean
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StorageCleanSummary {
    pub dry_run: bool,
    pub bytes_freed: u64,
    pub files_deleted: usize,
    pub files_skipped_locked: usize,
    pub targets_processed: Vec<String>,
    pub errors: Vec<String>,
}

// ==========================================
// Process Management Models (Interface Contract 2)
// ==========================================

/// Categorization classification for running processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessCategory {
    #[serde(alias = "system")]
    System,
    #[serde(alias = "backgroundservice", alias = "service")]
    BackgroundService,
    #[serde(alias = "application", alias = "app")]
    Application,
    #[serde(alias = "browserengine", alias = "browser")]
    BrowserEngine,
}

/// Process item with telemetry, category, and system safety protection flag
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
    pub category: ProcessCategory,
    pub is_protected: bool,
}

impl ProcessItem {
    pub fn new(
        pid: u32,
        name: impl Into<String>,
        cpu_percent: f32,
        memory_bytes: u64,
        status: impl Into<String>,
        category: ProcessCategory,
        is_protected: bool,
    ) -> Self {
        Self {
            pid,
            name: name.into(),
            cpu_percent,
            memory_bytes,
            status: status.into(),
            category,
            is_protected,
        }
    }
}

impl From<ProcessItem> for ProcessInfo {
    fn from(item: ProcessItem) -> Self {
        Self {
            pid: item.pid,
            name: item.name,
            cpu_percent: item.cpu_percent,
            memory_bytes: item.memory_bytes,
            status: item.status,
        }
    }
}

/// Result returned from an attempted process termination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminateResult {
    pub pid: u32,
    pub process_name: String,
    pub success: bool,
    pub message: String,
}

impl TerminateResult {
    pub fn success(pid: u32, name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            pid,
            process_name: name.into(),
            success: true,
            message: message.into(),
        }
    }

    pub fn failure(pid: u32, name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            pid,
            process_name: name.into(),
            success: false,
            message: message.into(),
        }
    }
}

// ==========================================
// Offline Assistant Models
// ==========================================

/// Offline companion assistant response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantResponse {
    pub reply: String,
    pub references: Vec<String>,
}
```

### 4.4 Target 4: `d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/knowledge/mod.rs`
Expand `KnowledgeBase::new()` with `storage_cleanup_safety` and `process_management_safety`, and provide `get_by_id(&self, id: &str)`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeArticle {
    pub id: String,
    pub keywords: Vec<String>,
    pub title_en: String,
    pub title_vi: String,
    pub content_en: String,
    pub content_vi: String,
}

pub struct KnowledgeBase {
    articles: Vec<KnowledgeArticle>,
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeBase {
    pub fn new() -> Self {
        let articles = vec![
            KnowledgeArticle {
                id: "cpu_troubleshooting".to_string(),
                keywords: vec![
                    "cpu".to_string(), "processor".to_string(), "quá tải".to_string(),
                    "chíp".to_string(), "nóng máy".to_string(), "100% cpu".to_string(),
                    "lag".to_string(), "giật".to_string(), "đơ".to_string(),
                ],
                title_en: "CPU Troubleshooting & High Usage Analysis".to_string(),
                title_vi: "Khắc phục CPU hoạt động tải cao & Đơ giật".to_string(),
                content_en: "When CPU usage is high:\n1. Check the Processes tab to identify which process consumes the most cycles.\n2. Heavy operations like code compilation, Windows Update, or anti-virus background scans often cause temporary spikes.\n3. If an unrecognized process stays above 80%, consider ending its task.\n4. Ensure proper cooling and clean dust from heatsinks.".to_string(),
                content_vi: "Khi CPU hoạt động ở mức tải cao:\n1. Kiểm tra tab Tiến trình (Processes) để xem ứng dụng nào đang chiếm tài nguyên nhiều nhất.\n2. Các tác vụ như biên dịch mã, cập nhật Windows hoặc quét virus chạy ngầm có thể gây tăng đột ngột trong thời gian ngắn.\n3. Nếu một ứng dụng lạ duy trì trên 80% CPU liên tục, bạn có thể cân nhắc đóng ứng dụng đó.\n4. Đảm bảo hệ thống tản nhiệt thông thoáng và quạt làm mát hoạt động tốt.".to_string(),
            },
            KnowledgeArticle {
                id: "ram_troubleshooting".to_string(),
                keywords: vec![
                    "ram".to_string(), "memory".to_string(), "bộ nhớ".to_string(),
                    "tràn ram".to_string(), "hết ram".to_string(), "out of memory".to_string(),
                ],
                title_en: "Memory & RAM Optimization".to_string(),
                title_vi: "Tối ưu và giải phóng bộ nhớ RAM".to_string(),
                content_en: "When available memory drops below safe levels:\n1. Web browsers with numerous active tabs often consume multiple gigabytes. Close unused tabs or use tab-discarding extensions.\n2. Windows uses available memory for caching, which is automatically freed when needed.\n3. If physical RAM is consistently > 90%, check if your Windows Paging File (pagefile.sys) is system-managed.\n4. Consider upgrading physical RAM if multitasking regularly exceeds current capacity.".to_string(),
                content_vi: "Khi bộ nhớ RAM khả dụng xuống thấp:\n1. Trình duyệt web mở nhiều tab thường ngốn nhiều GB bộ nhớ. Hãy đóng bớt các tab không sử dụng.\n2. Hệ điều hành Windows tự động đệm dữ liệu vào RAM và sẽ giải phóng khi có ứng dụng cần.\n3. Nếu RAM thực tế thường xuyên > 90%, kiểm tra kích thước bộ nhớ ảo (Paging File) trên ổ đĩa SSD/HDD.\n4. Cân nhắc nâng cấp thêm thanh RAM vật lý nếu nhu cầu đa nhiệm vượt quá dung lượng hiện có.".to_string(),
            },
            KnowledgeArticle {
                id: "disk_troubleshooting".to_string(),
                keywords: vec![
                    "disk".to_string(), "ổ đĩa".to_string(), "ổ cứng".to_string(),
                    "dung lượng".to_string(), "storage".to_string(), "đầy ổ".to_string(), "c:".to_string(),
                ],
                title_en: "Disk Free Space & Storage Health".to_string(),
                title_vi: "Giải phóng dung lượng & Sức khỏe ổ đĩa".to_string(),
                content_en: "Maintaining at least 15% free space on drive C: is essential for Windows performance:\n1. Run Windows Disk Cleanup (`cleanmgr.exe`) to purge Windows Update cache and temporary files.\n2. Clear temporary user files by pressing Win+R, typing `%TEMP%`, and deleting old logs.\n3. Use 'Installed Apps' in Windows Settings to remove unused games or bulky programs.\n4. Empty your Recycle Bin regularly.".to_string(),
                content_vi: "Duy trì ít nhất 15% dung lượng trống trên ổ C: là rất quan trọng:\n1. Chạy tiện ích Windows Disk Cleanup (`cleanmgr.exe`) để xóa file tạm và bản cập nhật cũ.\n2. Nhấn Win+R, gõ `%TEMP%` và dọn dẹp các tệp tạm thời không còn dùng đến.\n3. Truy cập Cài đặt Windows > Installed Apps để gỡ cài đặt các phần mềm hoặc game không dùng.\n4. Thường xuyên dọn sạch Thùng rác (Recycle Bin).".to_string(),
            },
            KnowledgeArticle {
                id: "temperature_info".to_string(),
                keywords: vec![
                    "temperature".to_string(), "nhiệt độ".to_string(), "nóng".to_string(),
                    "sensor".to_string(), "cảm biến".to_string(), "quạt".to_string(), "fan".to_string(),
                ],
                title_en: "Hardware Temperature & Thermal Monitoring".to_string(),
                title_vi: "Nhiệt độ phần cứng & Cảm biến nhiệt".to_string(),
                content_en: "Modern Windows restrictions:\n1. Reading CPU/GPU die temperatures directly requires specialized ring-0 kernel driver permissions (like WinRing0 or LibreHardwareMonitor).\n2. Unprivileged user-mode desktop applications cannot directly query motherboard ACPI thermal zones on many modern PCs.\n3. Safe operational ranges: Idle: 35-50°C; Heavy load: 65-85°C; Throttling threshold: > 95°C.".to_string(),
                content_vi: "Cơ chế theo dõi nhiệt độ trên Windows:\n1. Việc đọc trực tiếp cảm biến nhiệt của chip CPU/GPU đòi hỏi driver cấp hạt nhân (ring-0 kernel driver) có chữ ký số.\n2. Các ứng dụng chạy quyền người dùng thông thường trên Windows bị giới hạn quyền truy cập cảm biến ACPI.\n3. Mức nhiệt độ an toàn tham khảo: Khi nghỉ (35-50°C), khi tải nặng (65-85°C), nhiệt độ cảnh báo quá nhiệt (> 95°C).".to_string(),
            },
            KnowledgeArticle {
                id: "general_health".to_string(),
                keywords: vec![
                    "sức khỏe".to_string(), "health".to_string(), "bảo trì".to_string(),
                    "tối ưu".to_string(), "optimize".to_string(), "nhanh hơn".to_string(),
                    "doctor".to_string(), "bác sĩ".to_string(), "giúp".to_string(), "help".to_string(),
                    "tbk".to_string(), "core".to_string(),
                ],
                title_en: "TBK Core Routine Maintenance Guide".to_string(),
                title_vi: "Cẩm nang bảo trì định kỳ cùng TBK Core".to_string(),
                content_en: "Key guidelines for a fast and healthy PC:\n1. Keep Startup items minimal (Task Manager -> Startup tab).\n2. Restart your computer at least once a week to clear system handles and leaked memory.\n3. Keep Windows and drivers up to date.\n4. Monitor background processes using TBK Core dashboard regularly.".to_string(),
                content_vi: "Nguyên tắc giúp máy tính luôn mượt mà và bền bỉ:\n1. Giảm thiểu các ứng dụng khởi động cùng Windows (Task Manager > Startup tab).\n2. Khởi động lại máy ít nhất một lần mỗi tuần để giải phóng bộ nhớ và tiến trình tồn đọng.\n3. Cập nhật Windows và driver phần cứng định kỳ.\n4. Thường xuyên theo dõi bảng điều khiển TBK Core để phát hiện sớm các dấu hiệu quá tải.".to_string(),
            },
            KnowledgeArticle {
                id: "storage_cleanup_safety".to_string(),
                keywords: vec![
                    "clean".to_string(), "cleanup".to_string(), "temp".to_string(), "temporary".to_string(),
                    "safe".to_string(), "dry-run".to_string(), "dry run".to_string(), "storage".to_string(),
                    "dọn rác".to_string(), "file tạm".to_string(), "an toàn".to_string(), "dọn dẹp".to_string(),
                    "xóa rác".to_string(), "thư mục tạm".to_string(), "reclaim".to_string(), "xóa file".to_string(),
                    "crash dump".to_string(), "wer".to_string(), "thumbnail".to_string(),
                ],
                title_en: "Safe Storage & Temporary Cache Cleanup Guide".to_string(),
                title_vi: "Hướng dẫn Dọn dẹp Bộ nhớ tạm & An toàn Lưu trữ".to_string(),
                content_en: "Best practices for safe storage cleanup in TBK Core:\n1. Designated Safe Targets: Safe cleanup targets include User Temporary files (%TEMP%), Windows Error Reporting crash dumps (WER), application log files, and Windows thumbnail caches.\n2. Protected Boundary Guard: System-critical directories (C:\\Windows, System32, Program Files) and user personal folders (Desktop, Documents, Downloads) are strictly whitelisted/blacklisted and never deleted.\n3. Dry-Run Simulation: Always perform a dry-run first to preview exactly which files will be removed and calculate total reclaimable bytes without altering disk contents.\n4. Locked File Protection: Files currently opened or locked by active background applications are safely skipped without aborting or causing system instability.".to_string(),
                content_vi: "Nguyên tắc dọn dẹp dung lượng và bộ nhớ đệm an toàn trong TBK Core:\n1. Vùng dữ liệu an toàn: Các khu vực được phép dọn dẹp bao gồm Thư mục tạm người dùng (%TEMP%), Báo cáo sự cố Windows (WER Crash Dumps), Nhật ký ứng dụng (App Logs) và Bộ nhớ đệm hình thu nhỏ (Thumbnail Cache).\n2. Ranh giới bảo vệ nghiêm ngặt: Tuyệt đối không xóa các thư mục hệ thống (C:\\Windows, System32, Program Files) cũng như dữ liệu cá nhân (Desktop, Documents, Downloads).\n3. Chế độ Quét mô phỏng (Dry-Run): Luôn bật chế độ Dry-Run để tính toán trước dung lượng có thể giải phóng mà không làm thay đổi hay xóa bất kỳ tệp tin nào.\n4. Cơ chế bỏ qua tệp đang sử dụng: Những tệp đang bị ứng dụng khác khóa hoặc ghi dữ liệu sẽ được tự động bỏ qua an toàn, không làm gián đoạn hay treo phần mềm.".to_string(),
            },
            KnowledgeArticle {
                id: "process_management_safety".to_string(),
                keywords: vec![
                    "process".to_string(), "terminate".to_string(), "kill".to_string(), "blacklist".to_string(),
                    "protected".to_string(), "runaway".to_string(), "csrss".to_string(), "lsass".to_string(),
                    "services".to_string(), "smss".to_string(), "explorer".to_string(), "tiến trình".to_string(),
                    "tắt tiến trình".to_string(), "đóng ứng dụng".to_string(), "bảo vệ".to_string(),
                    "danh sách đen".to_string(), "an toàn tiến trình".to_string(), "task".to_string(),
                ],
                title_en: "Process Management Safety & Critical Windows Guardrails".to_string(),
                title_vi: "Quy tắc Quản lý Tiến trình An toàn & Bảo vệ Hệ thống".to_string(),
                content_en: "Guidelines for safe process termination and resource management:\n1. Hardcoded Safety Blacklist: Essential Windows OS services—such as csrss.exe, lsass.exe, services.exe, smss.exe, wininit.exe, and explorer.exe—are strictly protected. Terminating them is blocked by TBK Core to prevent system crash or Blue Screen of Death (BSOD).\n2. Process Categorization: Processes are classified into System, Background Service, Application, and Browser Engine. Inspect categories before deciding to terminate.\n3. Runaway Process Handling: Processes consuming >80% CPU or excessive RAM over sustained periods should be analyzed. Try closing the application window gracefully before forcing termination.\n4. Multi-Process Applications: Modern browsers and electron apps spawn multiple helper processes. Terminating individual tabs or renderer workers can free memory without closing the entire application.".to_string(),
                content_vi: "Hướng dẫn quản lý và kết thúc tiến trình an toàn:\n1. Danh sách bảo vệ bất khả xâm phạm (Safety Blacklist): Các tiến trình lõi của hệ điều hành Windows—như csrss.exe, lsass.exe, services.exe, smss.exe, wininit.exe, và explorer.exe—được khóa an toàn cứng. TBK Core từ chối lệnh đóng các tiến trình này để ngăn chặn lỗi màn hình xanh (BSOD) hoặc sập hệ thống.\n2. Phân loại tiến trình: Hệ thống phân chia thành Tiến trình hệ thống (System), Dịch vụ nền (Background Service), Ứng dụng người dùng (Application) và Nhân trình duyệt (Browser Engine). Hãy kiểm tra danh mục trước khi thực hiện đóng.\n3. Xử lý tiến trình chạy bất thường (Runaway): Những tiến trình ngốn >80% CPU hoặc rò rỉ RAM kéo dài cần được kiểm tra. Ưu tiên tắt qua giao diện ứng dụng trước khi buộc dừng bằng lệnh.\n4. Ứng dụng đa tiến trình: Trình duyệt web và ứng dụng nền tảng Chromium/Electron thường mở nhiều tiến trình phụ. Bạn có thể giải phóng tài nguyên bằng cách đóng từng tab thay vì tắt toàn bộ ứng dụng.".to_string(),
            },
        ];

        Self { articles }
    }

    pub fn get_by_id(&self, id: &str) -> Option<&KnowledgeArticle> {
        self.articles.iter().find(|art| art.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<&KnowledgeArticle> {
        let q = query.to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();

        self.articles
            .iter()
            .filter(|art| {
                art.keywords.iter().any(|k| q.contains(k))
                    || words.iter().any(|&w| {
                        art.title_en.to_lowercase().contains(w)
                            || art.title_vi.to_lowercase().contains(w)
                    })
            })
            .collect()
    }

    pub fn all_articles(&self) -> &[KnowledgeArticle] {
        &self.articles
    }
}
```

### 4.5 Target 5: `d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/lib.rs`
Cleanly expose modules and add full unit tests for all models and knowledge articles:

```rust
pub mod diagnostics;
pub mod knowledge;
pub mod models;

pub use diagnostics::{DiagnosticEngine, DiagnosticHistory};
pub use knowledge::{KnowledgeArticle, KnowledgeBase};
pub use models::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_engine_detects_low_disk() {
        let mut engine = DiagnosticEngine::new();
        let cpu = CpuSnapshot {
            model_name: "Test CPU".to_string(),
            logical_cores: 8,
            physical_cores: 4,
            global_usage_percent: 25.0,
            cores: vec![],
        };
        let mem = MemorySnapshot {
            total_bytes: 16 * 1024 * 1024 * 1024,
            used_bytes: 8 * 1024 * 1024 * 1024,
            available_bytes: 8 * 1024 * 1024 * 1024,
            usage_percent: 50.0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
        };
        let disks = vec![DiskSnapshot {
            name: "C:".to_string(),
            mount_point: "C:".to_string(),
            filesystem: "NTFS".to_string(),
            total_bytes: 100 * 1024 * 1024 * 1024,
            available_bytes: 1 * 1024 * 1024 * 1024, // 1% free -> critical
            used_bytes: 99 * 1024 * 1024 * 1024,
            usage_percent: 99.0,
            is_removable: false,
        }];
        let temperatures = vec![];
        let processes = vec![];

        let findings = engine.evaluate(&cpu, &mem, &disks, &temperatures, &processes);
        assert!(findings.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY"));
        assert!(findings.iter().any(|f| f.rule_id == "SENSOR_TEMP_UNAVAILABLE"));
    }

    #[test]
    fn test_diagnostic_sustained_cpu() {
        let mut engine = DiagnosticEngine::new();
        let cpu_high = CpuSnapshot {
            model_name: "Test CPU".to_string(),
            logical_cores: 8,
            physical_cores: 4,
            global_usage_percent: 95.0,
            cores: vec![],
        };
        let mem = MemorySnapshot {
            total_bytes: 16 * 1024 * 1024 * 1024,
            used_bytes: 4 * 1024 * 1024 * 1024,
            available_bytes: 12 * 1024 * 1024 * 1024,
            usage_percent: 25.0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
        };
        let disks = vec![DiskSnapshot {
            name: "C:".to_string(),
            mount_point: "C:".to_string(),
            filesystem: "NTFS".to_string(),
            total_bytes: 100 * 1024 * 1024 * 1024,
            available_bytes: 50 * 1024 * 1024 * 1024,
            used_bytes: 50 * 1024 * 1024 * 1024,
            usage_percent: 50.0,
            is_removable: false,
        }];
        let temps = vec![TemperatureSensor {
            label: "CPU Package".to_string(),
            celsius: Some(45.0),
            status: "available".to_string(),
        }];
        let procs = vec![];

        // Sample 1 & 2: not sustained yet
        let f1 = engine.evaluate(&cpu_high, &mem, &disks, &temps, &procs);
        assert!(!f1.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED"));

        let f2 = engine.evaluate(&cpu_high, &mem, &disks, &temps, &procs);
        assert!(!f2.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED"));

        // Sample 3: now sustained >= 3 samples
        let f3 = engine.evaluate(&cpu_high, &mem, &disks, &temps, &procs);
        assert!(f3.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED"));
    }

    #[test]
    fn test_knowledge_base_search_and_retrieval() {
        let kb = KnowledgeBase::new();

        // Existing articles
        let results_vi = kb.search("máy bị đầy ổ đĩa");
        assert!(!results_vi.is_empty());
        assert_eq!(results_vi[0].id, "disk_troubleshooting");

        let results_en = kb.search("high cpu lag");
        assert!(!results_en.is_empty());
        assert_eq!(results_en[0].id, "cpu_troubleshooting");

        // New storage cleanup article
        let storage_vi = kb.search("dọn rác và thư mục tạm an toàn");
        assert!(!storage_vi.is_empty());
        assert!(storage_vi.iter().any(|a| a.id == "storage_cleanup_safety"));

        let storage_en = kb.search("safe storage temp cleanup dry-run");
        assert!(!storage_en.is_empty());
        assert!(storage_en.iter().any(|a| a.id == "storage_cleanup_safety"));

        // New process management safety article
        let proc_vi = kb.search("tắt tiến trình hệ thống danh sách đen csrss");
        assert!(!proc_vi.is_empty());
        assert!(proc_vi.iter().any(|a| a.id == "process_management_safety"));

        let proc_en = kb.search("kill process blacklist guardrails lsass");
        assert!(!proc_en.is_empty());
        assert!(proc_en.iter().any(|a| a.id == "process_management_safety"));

        // Direct get_by_id lookup
        assert!(kb.get_by_id("storage_cleanup_safety").is_some());
        assert!(kb.get_by_id("process_management_safety").is_some());
        assert!(kb.get_by_id("non_existent").is_none());
    }

    #[test]
    fn test_storage_models_serde() {
        let target = CleanableTarget::new(
            "temp_user",
            "User Temporary Files",
            "Tệp tạm người dùng",
            "Cached application files",
            "Tệp đệm ứng dụng",
            "C:\\Users\\User\\AppData\\Local\\Temp",
            120,
            104857600, // 100 MB
            true,
        );

        let report = StorageScanReport::new(vec![target.clone()]);
        assert_eq!(report.total_files, 120);
        assert_eq!(report.total_reclaimable_bytes, 104857600);

        let serialized_report = serde_json::to_string(&report).expect("Serialize report");
        let deserialized_report: StorageScanReport =
            serde_json::from_str(&serialized_report).expect("Deserialize report");
        assert_eq!(deserialized_report.total_files, 120);
        assert_eq!(deserialized_report.targets.len(), 1);
        assert_eq!(deserialized_report.targets[0].id, "temp_user");

        let clean_req = CleanRequest::new(vec!["temp_user".to_string()], true);
        let serialized_req = serde_json::to_string(&clean_req).expect("Serialize clean req");
        let deserialized_req: CleanRequest =
            serde_json::from_str(&serialized_req).expect("Deserialize clean req");
        assert!(deserialized_req.dry_run);
        assert_eq!(deserialized_req.target_ids, vec!["temp_user"]);

        let summary = StorageCleanSummary {
            dry_run: true,
            bytes_freed: 50000000,
            files_deleted: 60,
            files_skipped_locked: 2,
            targets_processed: vec!["temp_user".to_string()],
            errors: vec![],
        };
        let serialized_summary = serde_json::to_string(&summary).expect("Serialize summary");
        let deserialized_summary: StorageCleanSummary =
            serde_json::from_str(&serialized_summary).expect("Deserialize summary");
        assert_eq!(deserialized_summary.bytes_freed, 50000000);
        assert_eq!(deserialized_summary.files_skipped_locked, 2);
    }

    #[test]
    fn test_process_models_and_conversion() {
        let proc_item = ProcessItem::new(
            1234,
            "chrome.exe",
            12.5,
            250 * 1024 * 1024,
            "Run",
            ProcessCategory::BrowserEngine,
            false,
        );

        let serialized = serde_json::to_string(&proc_item).expect("Serialize proc item");
        assert!(serialized.contains("\"category\":\"browser_engine\""));

        let deserialized: ProcessItem =
            serde_json::from_str(&serialized).expect("Deserialize proc item");
        assert_eq!(deserialized.category, ProcessCategory::BrowserEngine);
        assert_eq!(deserialized.pid, 1234);

        // Test conversion into ProcessInfo for diagnostic evaluation
        let proc_info: ProcessInfo = proc_item.into();
        assert_eq!(proc_info.pid, 1234);
        assert_eq!(proc_info.name, "chrome.exe");
        assert_eq!(proc_info.cpu_percent, 12.5);

        // Test Category Deserialization Aliases
        let cat_app: ProcessCategory = serde_json::from_str("\"app\"").unwrap();
        assert_eq!(cat_app, ProcessCategory::Application);
        let cat_svc: ProcessCategory = serde_json::from_str("\"service\"").unwrap();
        assert_eq!(cat_svc, ProcessCategory::BackgroundService);
        let cat_sys: ProcessCategory = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(cat_sys, ProcessCategory::System);

        // Terminate result
        let term_ok = TerminateResult::success(1234, "chrome.exe", "Process terminated");
        assert!(term_ok.success);
        let term_fail = TerminateResult::failure(4, "csrss.exe", "Protected process cannot be terminated");
        assert!(!term_fail.success);
    }
}
```

---

## 5. Verification Method

### 5.1 Workspace Compilation & Profile Check
Run:
```powershell
cargo check --workspace
```
**Expected Result**:
- Zero compiler errors.
- **Zero workspace profile warnings** (verifies moving `[profile.release]` from `apps/desktop/src-tauri/Cargo.toml` to root `Cargo.toml`).

### 5.2 Unit Test Execution
Run:
```powershell
cargo test --workspace
```
**Expected Result**:
- All 5 test suites pass (including `test_diagnostic_engine_detects_low_disk`, `test_diagnostic_sustained_cpu`, `test_knowledge_base_search_and_retrieval`, `test_storage_models_serde`, `test_process_models_and_conversion`).
- Zero failures, panics, or timeouts.

### 5.3 Invalidation Conditions
- If `cargo check --workspace` still emits `warning: profiles for the non root package will be ignored`, verify that `[profile.release]` was deleted from `apps/desktop/src-tauri/Cargo.toml`.
- If `CleanableTarget` or `ProcessItem` fields differ in casing or types when deserialized in Tauri commands, verify serde attributes (`rename_all = "snake_case"`, `#[serde(alias = ...)]`).
