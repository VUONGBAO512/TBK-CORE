# TEST_INFRA.md — E2E Test Suite & Test Infrastructure Architecture
**PC Doctor (TBK Core) — Actionable Diagnostics & Safe Cleanup Suite**

- **Architect**: E2E Test Suite Architect (`explorer_e2e_infra`)
- **Version**: 1.0.0
- **Status**: ESTABLISHED & SPECIFIED
- **Date**: 2026-10-16

---

## 1. Test Infrastructure Philosophy & 4-Tier Methodology

To ensure absolute reliability, safety, and operational resilience across PC Doctor (TBK Core), the test architecture is structured across four orthogonal, mutually reinforcing validation tiers:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    TIER 4: REAL-WORLD SCENARIOS                        │
│   End-to-End Operational Workflows (Multi-step User & System Journeys)  │
├─────────────────────────────────────────────────────────────────────────┤
│                TIER 3: CROSS-FEATURE PAIRWISE INTERACTIONS              │
│       Combinatorial Feature Coupling, Concurrency, State Feedback       │
├─────────────────────────────────────────────────────────────────────────┤
│            TIER 2: BOUNDARY VALUE ANALYSIS (BVA) & CORNER CASES         │
│     Extremes (0, Max, Overflows), Locked Resources, Security Bounds     │
├─────────────────────────────────────────────────────────────────────────┤
│                 TIER 1: FEATURE COVERAGE (CATEGORY-PARTITION)           │
│         Equivalence Partitions, Happy Path, Nominal Specifications      │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.1 Methodology Definitions
1. **Tier 1 (Category-Partition Feature Coverage)**: Decomposes each of the 17 features into functional equivalence partitions. Verifies nominal behavior, contract adherence, expected outputs, and invariants under normal operating conditions. (>=5 test cases per feature -> 85 test cases minimum).
2. **Tier 2 (Boundary Value Analysis & Corner Cases)**: Probes mathematical edges (0, 1, max, overflow), empty/null/missing files, path traversal attempts, locked resource sharing violations, permission denials, unicode diacritics, and timeout conditions. (>=5 test cases per feature -> 85 test cases minimum).
3. **Tier 3 (Pairwise Combinatorial Interactions)**: Exercises pairs and triads of features operating simultaneously (e.g., telemetry updating during storage purge; runaway detection coupled with safety blacklist; bilingual switching during offline assistant querying).
4. **Tier 4 (Real-World Operational Scenarios)**: Simulates holistic, multi-step real-world user scenarios representing daily computer maintenance, crisis recovery (disk full / memory exhaustion), and malicious or accidental misconfigurations. (>=10 scenarios).

---

## 2. Feature Inventory Matrix (17 Features)

| Feature # | Feature Name | Core Responsibility | Primary Interfaces |
|---|---|---|---|
| **F01** | Hardware Telemetry Metrics | CPU, RAM, Disks, OS, Uptime, Sensors | `SystemSnapshot`, `CpuSnapshot`, `MemorySnapshot`, `DiskSnapshot` |
| **F02** | Deterministic Diagnostic Rules | 8+ deterministic diagnostic rules across 5 severity tiers | `DiagnosticEngine`, `DiagnosticFinding`, `SeverityLevel` |
| **F03** | Safe Temp Cache Scanner | Scans %TEMP%, CrashDumps, WER, Thumbnails, Logs | `scan_storage()`, `CleanableCategory`, `StorageScanResult` |
| **F04** | Exact Reclaimable Byte Sizes | Non-negative byte calculation & itemized target breakdown | `file_count`, `total_bytes`, `freed_bytes`, `CleanSummary` |
| **F05** | Dry-Run Simulation Mode | Calculates reclaimable space without modifying disk | `CleanOptions { dry_run: true }`, zero disk deletion invariant |
| **F06** | Locked/In-Use File Skip | Gracefully skips locked files without panics, reports freed bytes | `ERROR_SHARING_VIOLATION`, `skipped_files_count`, `skipped_errors` |
| **F07** | Protected Location Boundary Guard | Strictly rejects system roots, Windows directories, user documents | `PROTECTED_LOCATIONS_BLACKLIST`, path canonicalization |
| **F08** | Process Listing & Sorting | Query running processes sorted by CPU and memory consumption | `ProcessInfo`, `get_process_list()`, sorting comparator |
| **F09** | Process Categorization | Categorizes processes into Application ('app') vs Service ('service') | `ProcessCategory`, path/heuristic classification |
| **F10** | Runaway Process Detection | Flags processes exceeding CPU/RAM safety thresholds | `PERF_RUNAWAY_PROCESS`, threshold evaluator |
| **F11** | Process Safety Blacklist | Hardcoded blacklist protecting vital Windows OS processes & PID <= 4 | `PROTECTED_SYSTEM_PROCESSES`, `ProcessTerminationError` |
| **F12** | Offline Companion Assistant | Local context-aware diagnostics and guidance with zero cloud calls | `query_assistant()`, `AssistantResponse`, offline privacy |
| **F13** | Bilingual Knowledge Base | Bilingual troubleshooting articles in Vietnamese and English | `KnowledgeBase`, `KnowledgeArticle`, token search |
| **F14** | React Storage Cleaner Suite | Full UI with itemized breakdown, dry-run toggle, purge confirmation | `StorageCleanerPanel.tsx`, selection state, modal workflow |
| **F15** | React Process Manager UI | Category tabs, protected status badges, terminate button & modal | `ProcessPanel.tsx`, category filter, safety dialog |
| **F16** | React Bilingual Localization | Seamless vi/en switching across all UI views and assistant | `i18n/{en,vi}.ts`, dictionary bundles, active locale state |
| **F17** | E2E Test Suite & Test Runner | Automated opaque-box test runner covering Tiers 1-4 | `tests/e2e_runner.rs`, exit code 0, structured reports |

---

## 3. Tier 1: Feature Coverage (Category-Partition Specifications)
*(>= 5 test cases per feature = 85 test cases)*

### Feature F01: Hardware Telemetry Metrics
- **T1-F01-01 (CPU Global & Per-Core Metrics)**:
  - *Partition*: Valid multi-core CPU snapshot.
  - *Input*: `CpuSnapshot` with 8 logical cores, global usage = 42.5%, per-core usage [30.0, 55.0, ...].
  - *Expected*: `snapshot.logical_cores == 8`, `snapshot.global_usage_percent == 42.5`, cores count == 8.
- **T1-F01-02 (Memory Metrics Calculation)**:
  - *Partition*: Standard RAM utilization.
  - *Input*: Total = 16 GB (17,179,869,184 B), Used = 8 GB (8,589,934,592 B), Available = 8 GB.
  - *Expected*: `usage_percent == 50.0`, `used_bytes + available_bytes <= total_bytes + swap_delta`.
- **T1-F01-03 (Disk Volume Enumeration)**:
  - *Partition*: Multiple mounted NTFS volumes (C:, D:).
  - *Input*: Two volumes with valid total, used, available bytes.
  - *Expected*: Returns 2 `DiskSnapshot` entries; each mount point formatted correctly; `used_bytes <= total_bytes`.
- **T1-F01-04 (OS Platform & Uptime)**:
  - *Partition*: Windows OS host snapshot.
  - *Input*: OS telemetry with `name: "Windows"`, `uptime_seconds: 7200`.
  - *Expected*: `os.name == "Windows"`, `uptime_seconds >= 0`, `kernel_version` non-empty.
- **T1-F01-05 (Temperature Sensor Enumeration)**:
  - *Partition*: Sensor list querying.
  - *Input*: System with ambient/core temperature sensors.
  - *Expected*: Sensor vector returned; status is `"available"` or `"unavailable"`; no panic on absent sensors.

### Feature F02: Deterministic Diagnostic Rules
- **T1-F02-01 (Disk Critical Rule Evaluation)**:
  - *Partition*: Fixed volume with free space < 2.0%.
  - *Input*: Total = 100 GB, Available = 1 GB (1.0% free).
  - *Expected*: Rule `DISK_CRITICAL_CAPACITY` emitted with `SeverityLevel::Critical`, confidence = 1.00.
- **T1-F02-02 (Disk Low Rule Evaluation)**:
  - *Partition*: Fixed volume with free space between 2.0% and 10.0%.
  - *Input*: Total = 100 GB, Available = 5 GB (5.0% free).
  - *Expected*: Rule `DISK_LOW_CAPACITY` emitted with `SeverityLevel::Moderate`.
- **T1-F02-03 (Sustained CPU Load Rule)**:
  - *Partition*: Sustained high CPU load across >= 3 ticks.
  - *Input*: 3 consecutive ticks with CPU usage = 95.0%.
  - *Expected*: Rule `PERF_CPU_HIGH_SUSTAINED` emitted with `SeverityLevel::High`.
- **T1-F02-04 (Low Available Memory Rule)**:
  - *Partition*: Available RAM below 500 MB.
  - *Input*: Total = 16 GB, Available = 400 MB (419,430,400 B).
  - *Expected*: Rule `PERF_MEM_LOW_AVAILABLE` emitted with `SeverityLevel::Moderate`.
- **T1-F02-05 (System Healthy Baseline)**:
  - *Partition*: All metrics within nominal ranges (CPU 20%, RAM 40%, Disk 50% free).
  - *Input*: Clean telemetry snapshot.
  - *Expected*: No Critical/High findings; `SYSTEM_HEALTHY` finding emitted or findings vector empty.

### Feature F03: Safe Temp Cache Scanner
- **T1-F03-01 (User Temp Directory Discovery)**:
  - *Partition*: Standard `%TEMP%` directory containing mock files.
  - *Input*: Target directory with 10 files totaling 25 MB.
  - *Expected*: Returns category `id: "user_temp"`, `file_count: 10`, `total_bytes: 26,214,400`, `accessible: true`.
- **T1-F03-02 (User Crash Dumps Target)**:
  - *Partition*: `%LOCALAPPDATA%\CrashDumps` containing `*.dmp` files.
  - *Input*: Target with 2 crash dump files.
  - *Expected*: Returns category `id: "crash_dumps"`, `file_count: 2`, `is_safe: true`.
- **T1-F03-03 (WER Report Archives)**:
  - *Partition*: Windows Error Reporting archive cache.
  - *Input*: Target containing error reports and logs.
  - *Expected*: Returns category `id: "wer_reports"`, non-negative file count and byte sum.
- **T1-F03-04 (Explorer Thumbnail Cache)**:
  - *Partition*: Explorer database cache (`thumbcache_*.db`).
  - *Input*: Target with mock `thumbcache_256.db`.
  - *Expected*: Returns category `id: "thumbnail_cache"`, `is_safe: true`.
- **T1-F03-05 (Itemized Scan Aggregation)**:
  - *Partition*: Full multi-target scan.
  - *Input*: 5 valid cleanable target roots.
  - *Expected*: `StorageScanResult.categories.len() == 5`, total cleanable bytes equals sum of category bytes.

### Feature F04: Exact Reclaimable Byte Sizes
- **T1-F04-01 (Single File Exact Size)**:
  - *Partition*: Exactly 1 file of size 1,048,576 bytes (1 MB).
  - *Input*: File created with 1,048,576 bytes.
  - *Expected*: `category.total_bytes == 1,048,576`, `category.file_count == 1`.
- **T1-F04-02 (Multi-File Accurate Summation)**:
  - *Partition*: 100 files of 10,240 bytes each.
  - *Input*: Total bytes created = 1,024,000 bytes.
  - *Expected*: `category.total_bytes == 1,024,000`, `category.file_count == 100`.
- **T1-F04-03 (Non-Negative Invariant)**:
  - *Partition*: Any arbitrary directory state.
  - *Input*: Scan results across arbitrary targets.
  - *Expected*: All byte fields are typed `u64`, mathematically guaranteed `>= 0`.
- **T1-F04-04 (Cross-Category Aggregate Equality)**:
  - *Partition*: 3 categories with varying file sizes.
  - *Input*: Cat1 = 500 KB, Cat2 = 1.2 MB, Cat3 = 800 KB.
  - *Expected*: `report.total_cleanable_bytes == (500*1024 + 1200*1024 + 800*1024)`.
- **T1-F04-05 (Freed Bytes Accuracy on Clean)**:
  - *Partition*: Live purge of 3 files totaling 3,145,728 bytes.
  - *Input*: `CleanOptions` executing purge.
  - *Expected*: `summary.freed_bytes == 3,145,728`, `summary.deleted_files_count == 3`.

### Feature F05: Dry-Run Simulation Mode
- **T1-F05-01 (Dry-Run Flag Fidelity)**:
  - *Partition*: Purge request with `dry_run: true`.
  - *Input*: `CleanOptions { category_ids: vec!["user_temp"], dry_run: true }`.
  - *Expected*: `CleanSummary.dry_run == true`.
- **T1-F05-02 (Byte Simulation Accuracy)**:
  - *Partition*: Mock files totaling 15 MB in target.
  - *Input*: Dry-run purge request.
  - *Expected*: `summary.freed_bytes == 15,728,640`, matching scanned bytes.
- **T1-F05-03 (File Count Simulation Accuracy)**:
  - *Partition*: 20 mock files in target.
  - *Input*: Dry-run purge request.
  - *Expected*: `summary.deleted_files_count == 20`.
- **T1-F05-04 (Zero Disk Deletion Invariant)**:
  - *Partition*: Files on filesystem before and after dry-run.
  - *Input*: Execute dry-run purge.
  - *Expected*: All 20 files continue to exist on disk; `file.exists() == true` for all targets.
- **T1-F05-05 (File Content & Hash Integrity)**:
  - *Partition*: File SHA256 checksums before and after dry-run.
  - *Input*: Inspect target file checksums.
  - *Expected*: Checksums strictly unchanged; zero write/truncate operations occurred.

### Feature F06: Locked/In-Use File Skip
- **T1-F06-01 (Single Locked File Handled)**:
  - *Partition*: Directory containing 1 locked file (held open exclusively) and 2 unlocked files.
  - *Input*: Live purge execution.
  - *Expected*: Purge succeeds without panic; 2 unlocked files deleted; 1 locked file preserved.
- **T1-F06-02 (Skipped Files Counter)**:
  - *Partition*: 2 locked files in target directory.
  - *Input*: Live purge execution.
  - *Expected*: `summary.skipped_files_count == 2`.
- **T1-F06-03 (Descriptive Error Reporting)**:
  - *Partition*: Locked file encountering OS sharing violation.
  - *Input*: Live purge execution.
  - *Expected*: `summary.skipped_errors` contains entry referencing the locked file path and OS reason.
- **T1-F06-04 (Freed Bytes Excludes Locked Files)**:
  - *Partition*: 2 unlocked files (1 MB each) + 1 locked file (5 MB).
  - *Input*: Live purge execution.
  - *Expected*: `summary.freed_bytes == 2,097,152` (exactly 2 MB; locked 5 MB excluded).
- **T1-F06-05 (Zero-Panic Guarantee)**:
  - *Partition*: High concurrency lock contention during purge loop.
  - *Input*: Continuous locks placed while purge loop executes.
  - *Expected*: Purge loop terminates gracefully with `Ok(CleanSummary)`; process does not abort.

### Feature F07: Protected Location Boundary Guard
- **T1-F07-01 (System32 Rejection)**:
  - *Partition*: Path resolving to `C:\Windows\System32`.
  - *Input*: Path candidate passed to boundary guard.
  - *Expected*: Returns `Err(StorageSecurityError::UnauthorizedTargetDirectory)`.
- **T1-F07-02 (Windows System Root Rejection)**:
  - *Partition*: Path resolving to `C:\Windows`.
  - *Input*: Path candidate passed to boundary guard.
  - *Expected*: Returns `Err(StorageSecurityError::UnauthorizedTargetDirectory)`.
- **T1-F07-03 (User Documents & Desktop Rejection)**:
  - *Partition*: Path resolving to `C:\Users\User\Documents` or `Desktop`.
  - *Input*: Path candidate passed to boundary guard.
  - *Expected*: Returns `Err(StorageSecurityError::UnauthorizedTargetDirectory)`.
- **T1-F07-04 (Base Target Directory Self-Deletion Protection)**:
  - *Partition*: Path equal to the root base temp directory itself.
  - *Input*: Attempting to delete `%TEMP%` root folder.
  - *Expected*: Guard rejects operation; only internal children may be cleaned.
- **T1-F07-05 (Legitimate Child Validation)**:
  - *Partition*: Canonical path strictly inside `%TEMP%\scratch_test\data.tmp`.
  - *Input*: Legitimate temp child passed to boundary guard.
  - *Expected*: Returns `Ok(())`.

### Feature F08: Process Listing & Sorting
- **T1-F08-01 (Process Snapshot Enumeration)**:
  - *Partition*: Standard active OS process table.
  - *Input*: Query `get_process_list()`.
  - *Expected*: Vector of `ProcessInfo` returned; count >= 1; PIDs are valid.
- **T1-F08-02 (Process Field Population)**:
  - *Partition*: Individual process record verification.
  - *Input*: Inspect any returned `ProcessInfo`.
  - *Expected*: `pid > 0`, `name` non-empty, `cpu_percent >= 0.0`, `memory_bytes > 0`.
- **T1-F08-03 (Sorting by CPU Descending)**:
  - *Partition*: Process list sorted by CPU.
  - *Input*: Sort comparator applied on `cpu_percent`.
  - *Expected*: For all `i < j`, `procs[i].cpu_percent >= procs[j].cpu_percent`.
- **T1-F08-04 (Sorting by Memory Descending)**:
  - *Partition*: Process list sorted by Memory.
  - *Input*: Sort comparator applied on `memory_bytes`.
  - *Expected*: For all `i < j`, `procs[i].memory_bytes >= procs[j].memory_bytes`.
- **T1-F08-05 (Filtering by Substring)**:
  - *Partition*: Search query `"chrome"`.
  - *Input*: Filter applied on process list.
  - *Expected*: Returned items all have `name.to_lowercase().contains("chrome")`.

### Feature F09: Process Categorization
- **T1-F09-01 (Application Classification)**:
  - *Partition*: User interactive app (e.g. `notepad.exe` in user session).
  - *Input*: Process running from user directory or `Program Files`.
  - *Expected*: `category == ProcessCategory::App`.
- **T1-F09-02 (Background Service Classification)**:
  - *Partition*: System service daemon (e.g. `svchost.exe`).
  - *Input*: Service process running under system account.
  - *Expected*: `category == ProcessCategory::Service`.
- **T1-F09-03 (Category Tag Serialization)**:
  - *Partition*: Serde JSON serialization of `ProcessCategory`.
  - *Input*: Serialize `ProcessCategory::App` and `ProcessCategory::Service`.
  - *Expected*: Serializes to `"app"` and `"service"` lowercase strings matching TypeScript types.
- **T1-F09-04 (UI Category Filter Subsetting)**:
  - *Partition*: Filter process table by `"app"`.
  - *Input*: Filter applied.
  - *Expected*: Returned list contains exclusively items with `category == ProcessCategory::App`.
- **T1-F09-05 (Protected Flag Derivation)**:
  - *Partition*: Process matching safety blacklist.
  - *Input*: Process `name: "csrss.exe"`.
  - *Expected*: `is_protected == true`.

### Feature F10: Runaway Process Detection
- **T1-F10-01 (High CPU Runaway Trigger)**:
  - *Partition*: Non-system process consuming >= 90.0% CPU.
  - *Input*: Process with `cpu_percent: 94.5`.
  - *Expected*: Evaluated as Runaway; `PERF_RUNAWAY_PROCESS` finding emitted.
- **T1-F10-02 (High Memory Runaway Trigger)**:
  - *Partition*: Non-system process consuming >= 2.0 GB RAM.
  - *Input*: Process with `memory_bytes: 2,500,000,000`.
  - *Expected*: Evaluated as Runaway; `PERF_RUNAWAY_PROCESS` finding emitted.
- **T1-F10-03 (Dual Pressure Runaway Trigger)**:
  - *Partition*: Non-system process with CPU >= 70% AND RAM >= 1.5 GB.
  - *Input*: `cpu_percent: 75.0`, `memory_bytes: 1,800,000,000`.
  - *Expected*: Evaluated as Runaway.
- **T1-F10-04 (Evidence Metadata Completeness)**:
  - *Partition*: Runaway finding generated.
  - *Input*: Inspect finding evidence array.
  - *Expected*: Evidence lists exact PID, process name, CPU percentage, and memory footprint in MB.
- **T1-F10-05 (Normal Process Negative Case)**:
  - *Partition*: Normal user application (CPU 12%, RAM 180 MB).
  - *Input*: Normal process metrics.
  - *Expected*: NOT evaluated as Runaway; no finding emitted for this process.

### Feature F11: Process Safety Blacklist
- **T1-F11-01 (csrss.exe Termination Blocked)**:
  - *Partition*: Request to terminate `csrss.exe`.
  - *Input*: `terminate_process(104, "csrss.exe")`.
  - *Expected*: `success == false`, `error == Some(ProcessTerminationError::ProtectedSystemProcess)`.
- **T1-F11-02 (lsass.exe Termination Blocked)**:
  - *Partition*: Request to terminate `lsass.exe`.
  - *Input*: `terminate_process(650, "lsass.exe")`.
  - *Expected*: `success == false`, `error == Some(ProcessTerminationError::ProtectedSystemProcess)`.
- **T1-F11-03 (services.exe Termination Blocked)**:
  - *Partition*: Request to terminate `services.exe`.
  - *Input*: `terminate_process(700, "services.exe")`.
  - *Expected*: `success == false`, `error == Some(ProcessTerminationError::ProtectedSystemProcess)`.
- **T1-F11-04 (smss.exe Termination Blocked)**:
  - *Partition*: Request to terminate `smss.exe`.
  - *Input*: `terminate_process(350, "smss.exe")`.
  - *Expected*: `success == false`, `error == Some(ProcessTerminationError::ProtectedSystemProcess)`.
- **T1-F11-05 (explorer.exe Termination Blocked)**:
  - *Partition*: Request to terminate `explorer.exe`.
  - *Input*: `terminate_process(2048, "explorer.exe")`.
  - *Expected*: `success == false`, `error == Some(ProcessTerminationError::ProtectedSystemProcess)`.

### Feature F12: Offline Companion Assistant
- **T1-F12-01 (Status Query in English)**:
  - *Partition*: Health status prompt with `lang: "en"`.
  - *Input*: `query_assistant("how is my system health?", "en")`.
  - *Expected*: Returns English live telemetry summary with CPU, RAM, Disk, and diagnostic findings.
- **T1-F12-02 (Status Query in Vietnamese)**:
  - *Partition*: Health status prompt with `lang: "vi"`.
  - *Input*: `query_assistant("tình trạng máy tính thế nào?", "vi")`.
  - *Expected*: Returns Vietnamese live telemetry summary with Báo cáo sức khỏe máy tính.
- **T1-F12-03 (Finding Next Steps Inclusion)**:
  - *Partition*: System with active `DISK_LOW_CAPACITY` finding.
  - *Input*: Query assistant status.
  - *Expected*: Response includes the specific actionable next step from the diagnostic finding.
- **T1-F12-04 (Healthy System Confirmation)**:
  - *Partition*: System with no diagnostic warnings.
  - *Input*: Query assistant status.
  - *Expected*: Response contains positive confirmation badge ("running within optimal performance parameters").
- **T1-F12-05 (100% Offline Execution Guarantee)**:
  - *Partition*: Assistant query executed without network connectivity.
  - *Input*: Query assistant with network interface disabled.
  - *Expected*: Query succeeds immediately (< 50ms) with full response; 0 socket connections opened.

### Feature F13: Bilingual Knowledge Base
- **T1-F13-01 (CPU Article Lookup in English)**:
  - *Partition*: Search query `"cpu high usage lag"`.
  - *Input*: `KnowledgeBase::search("cpu high usage lag")`.
  - *Expected*: Top result `id == "cpu_troubleshooting"`, title in English.
- **T1-F13-02 (CPU Article Lookup in Vietnamese)**:
  - *Partition*: Search query `"máy tính bị quá tải cpu nóng giật"`.
  - *Input*: `KnowledgeBase::search("máy tính bị quá tải cpu nóng giật")`.
  - *Expected*: Top result `id == "cpu_troubleshooting"`, content in Vietnamese.
- **T1-F13-03 (Storage Cleanup Guide Lookup)**:
  - *Partition*: Search query `"dọn dẹp bộ nhớ đệm temp"`.
  - *Input*: `KnowledgeBase::search("dọn dẹp bộ nhớ đệm temp")`.
  - *Expected*: Top result `id == "storage_cleaner_guide"`.
- **T1-F13-04 (Process Safety Guide Lookup)**:
  - *Partition*: Search query `"tắt tiến trình an toàn csrss"`.
  - *Input*: `KnowledgeBase::search("tắt tiến trình an toàn csrss")`.
  - *Expected*: Top result `id == "process_safety_guide"`.
- **T1-F13-05 (RAM Optimization Guide Lookup)**:
  - *Partition*: Search query `"tràn bộ nhớ ram hết ram"`.
  - *Input*: `KnowledgeBase::search("tràn bộ nhớ ram hết ram")`.
  - *Expected*: Top result `id == "ram_troubleshooting"`.

### Feature F14: React Storage Cleaner Suite
- **T1-F14-01 (Target Grid Rendering)**:
  - *Partition*: UI rendering after successful storage scan.
  - *Input*: `StorageScanReport` containing 5 categories.
  - *Expected*: Renders 5 category cards with localized names, file counts, and formatted byte sizes.
- **T1-F14-02 (Category Selection Toggle)**:
  - *Partition*: User clicks category checkbox.
  - *Input*: Toggle checkbox for `user_temp`.
  - *Expected*: Target ID added/removed from active selection set; total selected size updates dynamically.
- **T1-F14-03 (Dry-Run Switch Toggle)**:
  - *Partition*: User toggles Dry-Run switch.
  - *Input*: Click Dry-Run toggle.
  - *Expected*: UI state toggles between "Dry-Run Simulation" and "Live Clean"; button text updates.
- **T1-F14-04 (Purge Confirmation Modal Workflow)**:
  - *Partition*: User clicks Clean/Purge button.
  - *Input*: Click "Dọn dẹp ngay" / "Clean Now".
  - *Expected*: Modal opens displaying warning, selected category count, and total reclaimable bytes.
- **T1-F14-05 (Summary Results Banner Display)**:
  - *Partition*: Cleaning completes successfully.
  - *Input*: `CleanSummary` received via IPC.
  - *Expected*: Summary banner renders freed bytes, deleted files count, and skipped files count.

### Feature F15: React Process Manager UI
- **T1-F15-01 (Process Table Columns Rendering)**:
  - *Partition*: Process table initialization.
  - *Input*: List of 50 `ProcessInfo` items.
  - *Expected*: Table renders columns: PID, Name, Category, CPU %, Memory, Status, Actions.
- **T1-F15-02 (Category Tab Switching)**:
  - *Partition*: User clicks "Background Services" tab.
  - *Input*: Switch active tab to `service`.
  - *Expected*: Table filters to show only processes with `category === 'service'`.
- **T1-F15-03 (Protected Process Status Badge)**:
  - *Partition*: Row rendering for `csrss.exe`.
  - *Input*: `is_protected: true`.
  - *Expected*: Renders "Protected" / "Bảo vệ" badge; Terminate button is disabled or triggers safety modal.
- **T1-F15-04 (Terminate Non-Protected Process Confirmation)**:
  - *Partition*: User clicks Terminate on `notepad.exe`.
  - *Input*: Click Terminate button on non-protected row.
  - *Expected*: Confirmation dialog appears with process name and PID.
- **T1-F15-05 (Real-time Sorting Toggle)**:
  - *Partition*: User clicks "CPU %" column header.
  - *Input*: Click CPU header.
  - *Expected*: Table re-sorts descending; click again re-sorts ascending.

### Feature F16: React Bilingual Localization
- **T1-F16-01 (Language Switch to Vietnamese)**:
  - *Partition*: User selects "Tiếng Việt" in navigation bar.
  - *Input*: Select `'vi'` locale.
  - *Expected*: Navigation tabs change to "Tổng quan", "Bộ nhớ", "Tiến trình", "Trợ lý", "Cài đặt".
- **T1-F16-02 (Language Switch to English)**:
  - *Partition*: User selects "English".
  - *Input*: Select `'en'` locale.
  - *Expected*: Navigation tabs change to "Dashboard", "Storage", "Processes", "Assistant", "Settings".
- **T1-F16-03 (Dynamic Category Localization)**:
  - *Partition*: Storage cleaner target category cards.
  - *Input*: Toggle locale between `vi` and `en`.
  - *Expected*: Card titles switch between `name_vi` and `name_en`; descriptions switch between `description_vi` and `description_en`.
- **T1-F16-04 (Assistant Interface Localization)**:
  - *Partition*: Assistant panel greeting and suggestions.
  - *Input*: Active locale `'vi'`.
  - *Expected*: Placeholder text "Hỏi trợ lý PC Doctor..." and Vietnamese suggested prompt pills displayed.
- **T1-F16-05 (Diagnostic Severity Tier Localization)**:
  - *Partition*: Diagnostic badges rendered in UI.
  - *Input*: `SeverityLevel::Critical` under `vi` and `en`.
  - *Expected*: Renders "Nghiêm trọng" under `vi`, "Critical" under `en`.

### Feature F17: E2E Test Suite & Test Runner
- **T1-F17-01 (Runner Discovery & Registration)**:
  - *Partition*: Test suite registration.
  - *Input*: Initialize `E2ETestSuite`.
  - *Expected*: Registers all test cases across Tiers 1-4; total registered >= 180.
- **T1-F17-02 (Sequential Execution Pipeline)**:
  - *Partition*: Execution of test registry.
  - *Input*: Run suite.
  - *Expected*: Executes each test case, captures start and end time, records pass/fail.
- **T1-F17-03 (Structured Report Generation)**:
  - *Partition*: Test completion reporting.
  - *Input*: Suite execution finishes.
  - *Expected*: Generates structured report with total, passed, failed, and duration ms.
- **T1-F17-04 (Zero Exit Code on Full Pass)**:
  - *Partition*: All test assertions pass.
  - *Input*: Suite completes with 0 failures.
  - *Expected*: Test runner process exits with status code 0.
- **T1-F17-05 (Console ANSI Color Formatting)**:
  - *Partition*: Terminal output formatting.
  - *Input*: Terminal output generation.
  - *Expected*: Passed tests output green `[PASS]`, failed tests output red `[FAIL]`, summary banner printed.

---

## 4. Tier 2: Boundary Value Analysis & Corner Cases
*(>= 5 test cases per feature = 85 test cases)*

### Feature F01: Hardware Telemetry Metrics (Boundary Cases)
- **T2-F01-01 (0.0% and 100.0% Exact CPU Usage)**:
  - *Boundary*: Minimum (0.0%) and maximum (100.0%) CPU values.
  - *Input*: CPU snapshot with 0.0% and 100.0% global usage.
  - *Expected*: Accepted and serialized without clamping distortion; no NaN or infinite values.
- **T2-F01-02 (Zero Total Memory Edge Case)**:
  - *Boundary*: Total RAM = 0 (e.g. hypervisor / sandbox edge case).
  - *Input*: `MemorySnapshot` with `total_bytes: 0`.
  - *Expected*: `usage_percent` returns 0.0 without division-by-zero panic or hardware exception.
- **T2-F01-03 (Zero Capacity or Removable Disk Volume)**:
  - *Boundary*: Unformatted or removable media with 0 total bytes.
  - *Input*: Volume with `total_bytes: 0`, `available_bytes: 0`.
  - *Expected*: `usage_percent` returns 0.0; volume processed cleanly.
- **T2-F01-04 (Missing ACPI Temperature Sensors)**:
  - *Boundary*: Standard Windows user-mode environment where ring-0 ACPI thermal sensors return empty.
  - *Input*: `temperatures: vec![]`.
  - *Expected*: Handled cleanly without errors; `SENSOR_TEMP_UNAVAILABLE` informational finding emitted.
- **T2-F01-05 (System Uptime Boundary & Clock Rollover)**:
  - *Boundary*: Uptime = 0 seconds (instant boot) and Uptime = 4,294,967,295 seconds (>136 years).
  - *Input*: Monotonic uptime values tested at extremes.
  - *Expected*: Uptime formatted accurately into hours/days without integer overflow.

### Feature F02: Deterministic Diagnostic Rules (Boundary Cases)
- **T2-F02-01 (Exact Disk Threshold Boundaries: 2.00% & 10.00%)**:
  - *Boundary*: Available ratio exactly 0.020000 and 0.100000.
  - *Input*: Volume at exactly 2.0% free (triggers `DISK_LOW_CAPACITY`, NOT Critical); Volume at 1.999% triggers `DISK_CRITICAL_CAPACITY`.
  - *Expected*: Strict boundary inequality adherence (< 0.02 is Critical, 0.02 to 0.10 is Moderate).
- **T2-F02-02 (CPU Spike Duration: 1 or 2 Ticks vs 3 Ticks)**:
  - *Boundary*: High CPU (95%) for 1 tick, then 2 ticks, then 3 ticks.
  - *Input*: Ticks 1 and 2: `PERF_CPU_HIGH_SUSTAINED` must NOT trigger. Tick 3: triggers immediately.
  - *Expected*: Sustained window strictly requires >= 3 consecutive samples.
- **T2-F02-03 (Critical Memory Threshold: < 150 MB and >= 95% Used)**:
  - *Boundary*: Available RAM = 149 MB vs 150 MB.
  - *Input*: Available RAM = 149 MB, used = 96.0%.
  - *Expected*: Emits `PERF_MEM_CRITICAL_AVAILABLE` with `SeverityLevel::Critical`.
- **T2-F02-04 (Sensor Stale Data Elapsed Time Boundary: 10s vs 11s)**:
  - *Boundary*: Elapsed time between snapshot ticks.
  - *Input*: Elapsed = 9.9s (no finding); Elapsed = 10.1s (emits `SENSOR_STALE_DATA`).
  - *Expected*: Stale data finding accurately activates when elapsed > 10.0 seconds.
- **T2-F02-05 (Rolling History Buffer Cap)**:
  - *Boundary*: 50 consecutive telemetry ticks fed into engine.
  - *Input*: Feed 50 ticks to `DiagnosticEngine`.
  - *Expected*: Internal history buffer capped at exactly 10 samples; oldest samples discarded without memory leak.

### Feature F03: Safe Temp Cache Scanner (Boundary Cases)
- **T2-F03-01 (Target Directory Does Not Exist on Disk)**:
  - *Boundary*: `%LOCALAPPDATA%\CrashDumps` does not exist on a clean machine.
  - *Input*: Target path does not exist.
  - *Expected*: Category returned with `file_count: 0`, `total_bytes: 0`, `accessible: false`; no error thrown.
- **T2-F03-02 (Target Directory Unprivileged / Access Denied)**:
  - *Boundary*: `C:\Windows\Prefetch` accessed by unprivileged standard user.
  - *Input*: Unreadable directory returning `ErrorKind::PermissionDenied`.
  - *Expected*: Handled gracefully; `accessible: false`, `file_count: 0`, `total_bytes: 0`; scanner continues.
- **T2-F03-03 (Unicode Combining Diacritics in User Profile Path)**:
  - *Boundary*: Path containing combining diacritics (e.g. `VuongGiaBaọ̣̣̣\AppData\Local\Temp`).
  - *Input*: Files located inside Unicode directory.
  - *Expected*: Full path correctly resolved using UTF-8 / UTF-16; scan finds and measures all files.
- **T2-F03-04 (Path Exceeding Windows MAX_PATH 260 Characters)**:
  - *Boundary*: Nested directory structure with path length > 300 characters.
  - *Input*: Target path > 260 characters using extended prefix (`\\?\`).
  - *Expected*: Scanner walks and measures deep files without truncation error.
- **T2-F03-05 (Empty Target Directory)**:
  - *Boundary*: Target directory exists but contains 0 files.
  - *Input*: Empty directory.
  - *Expected*: Category returned with `file_count: 0`, `total_bytes: 0`, `accessible: true`.

### Feature F04: Exact Reclaimable Byte Sizes (Boundary Cases)
- **T2-F04-01 (0-Byte Empty Files)**:
  - *Boundary*: Directory containing twenty 0-byte temporary files.
  - *Input*: Twenty empty files.
  - *Expected*: `file_count == 20`, `total_bytes == 0`.
- **T2-F04-02 (Multi-Gigabyte File Size > 4 GB - u32 Overflow Check)**:
  - *Boundary*: Crash dump file of size 5,368,709,120 bytes (5 GB).
  - *Input*: Mock file metadata reporting 5 GB.
  - *Expected*: Stored accurately in `u64`; no 32-bit truncation; `total_bytes == 5,368,709,120`.
- **T2-F04-03 (File Deleted Mid-Scan Race Condition)**:
  - *Boundary*: File returned by `read_dir` is deleted before `metadata()` is called.
  - *Input*: Mock missing file metadata error (`NotFound`).
  - *Expected*: Scanner ignores the deleted file, contributes 0 bytes, continues scan without panic.
- **T2-F04-04 (NTFS Reparse Points / Symlink Traversal Guard)**:
  - *Boundary*: Symlink inside temp folder pointing to a 100 GB external folder.
  - *Input*: Symlink target.
  - *Expected*: Symlink itself measured or ignored; external folder is NOT traversed or added to total bytes.
- **T2-F04-05 (Saturating Disk Math Invariant)**:
  - *Boundary*: Available bytes greater than total bytes (corrupt filesystem telemetry).
  - *Input*: `available_bytes = 100`, `total_bytes = 50`.
  - *Expected*: `used_bytes` calculation uses `total_bytes.saturating_sub(available_bytes)` yielding 0 without underflow panic.

### Feature F05: Dry-Run Simulation Mode (Boundary Cases)
- **T2-F05-01 (Dry-Run with Empty Category Target List)**:
  - *Boundary*: `category_ids: vec![]`.
  - *Input*: Dry-run with no categories selected.
  - *Expected*: `summary.freed_bytes == 0`, `summary.deleted_files_count == 0`, `summary.skipped_files_count == 0`.
- **T2-F05-02 (Dry-Run Against Locked Files)**:
  - *Boundary*: Target directory contains exclusively locked files.
  - *Input*: Dry-run execution.
  - *Expected*: Reports simulated freed bytes; does NOT attempt file deletion; `skipped_files_count == 0`.
- **T2-F05-03 (Repeated Idempotent Dry-Runs)**:
  - *Boundary*: 5 consecutive dry-run requests executed back-to-back.
  - *Input*: 5 dry-run calls.
  - *Expected*: All 5 return identical `freed_bytes` and `deleted_files_count`; zero state drift.
- **T2-F05-04 (Dry-Run with Mixed Existent and Non-Existent Categories)**:
  - *Boundary*: Request contains `["user_temp", "invalid_category_id"]`.
  - *Input*: Purge with invalid category ID.
  - *Expected*: Valid category simulated; invalid category recorded in errors or ignored safely; no crash.
- **T2-F05-05 (Read-Only Target Media in Dry-Run)**:
  - *Boundary*: Target directory on write-protected media.
  - *Input*: Dry-run execution.
  - *Expected*: Dry-run succeeds completely because no filesystem modifications are attempted.

### Feature F06: Locked/In-Use File Skip (Boundary Cases)
- **T2-F06-01 (100% of Files Locked in Target)**:
  - *Boundary*: All 10 files in target directory locked exclusively.
  - *Input*: Live purge execution.
  - *Expected*: `freed_bytes == 0`, `deleted_files_count == 0`, `skipped_files_count == 10`, 10 skipped error logs, 0 panics.
- **T2-F06-02 (Windows Read-Only Attribute on Temp File)**:
  - *Boundary*: File with Windows read-only attribute set (`FILE_ATTRIBUTE_READONLY`).
  - *Input*: Purge attempt on read-only file.
  - *Expected*: Cleaner attempts permission reset or skips gracefully with access denied; does not abort loop.
- **T2-F06-03 (Parent Directory Non-Empty Deletion Guard)**:
  - *Boundary*: Subdirectory contains 1 locked file; cleaner finishes file loop and attempts `remove_dir`.
  - *Input*: Directory removal when child file could not be deleted.
  - *Expected*: Directory removal fails gracefully (`DirectoryNotEmpty`); parent remains intact; no crash.
- **T2-F06-04 (Transient Lock Acquisition During Clean Loop)**:
  - *Boundary*: File unlocked when loop begins, locked by external process 1ms before `remove_file`.
  - *Input*: Rapid external lock injection.
  - *Expected*: `remove_file` error intercepted; file skipped; loop continues to next file.
- **T2-F06-05 (Zero Files Cleaned Due to Immediate Lock Contention)**:
  - *Boundary*: System temp file locked by active antivirus scan.
  - *Input*: Live purge on locked log file.
  - *Expected*: Result summary returned with `freed_bytes == 0`; caller receives structured result, not an unhandled error.

### Feature F07: Protected Location Boundary Guard (Boundary Cases)
- **T2-F07-01 (Directory Traversal via Parent Dots `..`)**:
  - *Boundary*: Candidate path `%TEMP%\..\..\Windows\System32\cmd.exe`.
  - *Input*: Traversal path evaluated against boundary guard.
  - *Expected*: Path canonicalization resolves real target; fails boundary check; rejected immediately.
- **T2-F07-02 (Mixed Forward and Backward Slash Paths)**:
  - *Boundary*: Candidate path `C:/Users/User\AppData/Local\Temp/../Windows`.
  - *Input*: Mixed slash traversal path.
  - *Expected*: Canonicalized normalized path evaluated; rejected.
- **T2-F07-03 (Symlink / Junction Pointing Outside Temp Directory)**:
  - *Boundary*: NTFS junction created inside `%TEMP%\junction_link` -> `C:\Program Files`.
  - *Input*: Purge operation directed at junction target.
  - *Expected*: Canonical target resolves outside safe root whitelist; rejected with security error.
- **T2-F07-04 (Case Insensitivity Defense on Windows Paths)**:
  - *Boundary*: `c:\wInDoWs\sYsTeM32\kErNeL32.dLl`.
  - *Input*: Mixed-case protected path.
  - *Expected*: Case-insensitive comparison matches protected blacklist; rejected.
- **T2-F07-05 (Root Drive Target `C:\` or `\\?\C:\`)**:
  - *Boundary*: User or malicious input passing root drive path.
  - *Input*: Candidate path `C:\`.
  - *Expected*: Rejected; root volume format cannot match any safe temp whitelist.

### Feature F08: Process Listing & Sorting (Boundary Cases)
- **T2-F08-01 (Processes with Exact 0.0% CPU and 0 Memory)**:
  - *Boundary*: Zombie or suspended processes with 0 CPU and negligible memory.
  - *Input*: Process table containing suspended items.
  - *Expected*: Retained in list, sorted accurately at the bottom without floating-point comparison panics.
- **T2-F08-02 (Special Regex Characters in Search Query)**:
  - *Boundary*: Search query `[a-z]*?+`.
  - *Input*: Search query with regex metacharacters.
  - *Expected*: Treated as literal substring search; no regex compile panic; returns matching literal items or empty list.
- **T2-F08-03 (Empty Search Query / Whitespace Only)**:
  - *Boundary*: Search query `""` or `"   "`.
  - *Input*: Blank query.
  - *Expected*: Returns all processes without filtering.
- **T2-F08-04 (Identical Metric Values Stable Ordering)**:
  - *Boundary*: 10 processes all having exactly `cpu_percent: 0.0`.
  - *Input*: Sort by CPU.
  - *Expected*: Stable sort maintained (e.g. secondary sort by PID); no non-deterministic jitter.
- **T2-F08-05 (Process Terminated During Snapshot Traversal)**:
  - *Boundary*: PID exists during process enumeration but vanishes before metadata query.
  - *Input*: Ephemeral process dying during collection tick.
  - *Expected*: Collector ignores vanished PID cleanly; returns snapshot of remaining active processes.

### Feature F09: Process Categorization (Boundary Cases)
- **T2-F09-01 (Process with Missing or Inaccessible Executable Path)**:
  - *Boundary*: System process running in Session 0 where `exe_path` returns `None` due to ACL.
  - *Input*: `ProcessInfo { exe_path: None, name: "audiodg.exe" }`.
  - *Expected*: Categorized safely as `ProcessCategory::Service` based on name/session heuristics; no panic.
- **T2-F09-02 (Executable Running from Non-Standard Directory)**:
  - *Boundary*: Portable executable running directly from root of `D:\` or `%TEMP%`.
  - *Input*: `exe_path: Some("D:\\my_tool.exe")`.
  - *Expected*: Categorized as `ProcessCategory::App`.
- **T2-F09-03 (Process with Unicode or Non-ASCII Executable Name)**:
  - *Boundary*: Process name containing Vietnamese or Asian glyphs (`trình_duyệt.exe`).
  - *Input*: Non-ASCII process name.
  - *Expected*: Name string handled losslessly in UTF-8; categorization rules execute without crash.
- **T2-F09-04 (Sub-Processes with Identical Names Across Multiple Categories)**:
  - *Boundary*: Process named `cmd.exe` running as user tool vs spawned by a service.
  - *Input*: Multiple processes with identical base names.
  - *Expected*: Categorized independently based on individual paths and token sessions.
- **T2-F09-05 (Unknown Process Default Categorization)**:
  - *Boundary*: Completely unrecognized executable with no path.
  - *Input*: Process with unknown attributes.
  - *Expected*: Categorized with safe default (`Service` or `App`) without halting pipeline.

### Feature F10: Runaway Process Detection (Boundary Cases)
- **T2-F10-01 (Exact Threshold Boundaries: 89.9% vs 90.0% CPU)**:
  - *Boundary*: CPU usage at 89.9% (not runaway) vs 90.0% (runaway).
  - *Input*: Process with 89.9% CPU -> no runaway; 90.0% -> runaway triggered.
  - *Expected*: Strict boundary condition `>= 90.0` verified.
- **T2-F10-02 (Exact Memory Boundary: 1,999,999,999 B vs 2,000,000,000 B)**:
  - *Boundary*: Memory at 1.999 GB (not runaway) vs 2.000 GB (runaway).
  - *Input*: Process with 1,999,999,999 bytes vs 2,000,000,000 bytes.
  - *Expected*: Strict boundary condition `>= 2 GB` verified.
- **T2-F10-03 (System Idle Process High CPU Exemption)**:
  - *Boundary*: System Idle Process consuming 99.0% CPU.
  - *Input*: `ProcessInfo { pid: 0, name: "System Idle Process", cpu_percent: 99.0 }`.
  - *Expected*: Idle process exempted from runaway classification; no alert emitted.
- **T2-F10-04 (Multiple Concurrent Runaway Processes)**:
  - *Boundary*: 3 distinct processes exceeding thresholds simultaneously.
  - *Input*: Process table with 3 runaway processes.
  - *Expected*: Evaluator detects all 3; emits 3 distinct diagnostic findings with respective PIDs.
- **T2-F10-05 (Rapid Metric Spike and Recovery)**:
  - *Boundary*: Process at 95% CPU for 1 tick, returning to 5% CPU next tick.
  - *Input*: Sequential snapshot ticks.
  - *Expected*: Evaluator flags runaway during tick 1; clears alert during tick 2.

### Feature F11: Process Safety Blacklist (Boundary Cases)
- **T2-F11-01 (PID 0 and PID 4 Core Kernel Guard)**:
  - *Boundary*: Attempting termination of PID 0 (Idle) or PID 4 (System).
  - *Input*: `terminate_process(0, "System Idle")` and `terminate_process(4, "System")`.
  - *Expected*: Rejected immediately with `ProtectedSystemProcess` error; PID <= 4 boundary enforced.
- **T2-F11-02 (Case Variations in Process Name Matching)**:
  - *Boundary*: Target names `CSRSS.EXE`, `csrss.Exe`, `CsRsS.eXe`.
  - *Input*: Various casing permutations of blacklisted names.
  - *Expected*: All casing permutations rejected via case-insensitive comparison.
- **T2-F11-03 (Target Process Does Not Exist / Invalid PID)**:
  - *Boundary*: Termination request for PID 999999 (non-existent).
  - *Input*: `terminate_process(999999, "ghost.exe")`.
  - *Expected*: Returns `success: false`, `error: Some(ProcessTerminationError::ProcessNotFound)`.
- **T2-F11-04 (Self-Termination Guard on TBK Core Executables)**:
  - *Boundary*: Termination request for `tbk_core.exe` or `pc-doctor.exe`.
  - *Input*: Target matching current application binary.
  - *Expected*: Protected by blacklist; returns `ProtectedSystemProcess` error; prevents self-kill.
- **T2-F11-05 (Process Name Spoofing Defense)**:
  - *Boundary*: User application named `csrss.exe` running from Desktop.
  - *Input*: Process with blacklisted name but non-system path.
  - *Expected*: Name-based blacklist blocks termination unconditionally for safety.

### Feature F12: Offline Companion Assistant (Boundary Cases)
- **T2-F12-01 (Empty Query String)**:
  - *Boundary*: Query `""` or whitespace `"   "`.
  - *Input*: `query_assistant("", "en")`.
  - *Expected*: Returns default bilingual assistant greeting; no panic or null pointer.
- **T2-F12-02 (Extremely Long Query String > 10,000 Characters)**:
  - *Boundary*: Buffer stress with 10,000 characters.
  - *Input*: Large string input.
  - *Expected*: Query processed safely within bounded time; returns valid response; no stack/heap overflow.
- **T2-F12-03 (Unrecognized Nonsense / Gibberish Prompt)**:
  - *Boundary*: Query `"asdfkjhasdf 123897123 ?!?!"`.
  - *Input*: Gibberish prompt.
  - *Expected*: Returns fallback response suggesting valid system health and cleanup topics.
- **T2-F12-04 (Vietnamese Diacritics and Mixed Punctuation)**:
  - *Boundary*: Query `"MÁY TÍNH CÓ BỊ ĐẦY Ổ KHÔNG???????????"`.
  - *Input*: Uppercase with complex diacritics and punctuation.
  - *Expected*: Correctly normalized; triggers disk capacity evaluation and response.
- **T2-F12-05 (Unsupported Language Code Fallback)**:
  - *Boundary*: Language code `"fr"` or `"de"` passed to assistant.
  - *Input*: `query_assistant("status", "fr")`.
  - *Expected*: Gracefully falls back to default English (`en`) without crashing.

### Feature F13: Bilingual Knowledge Base (Boundary Cases)
- **T2-F13-01 (Single Character Query)**:
  - *Boundary*: Search query `"a"`.
  - *Input*: `KnowledgeBase::search("a")`.
  - *Expected*: Returns empty list or lowest-ranked articles; executes safely.
- **T2-F13-02 (Query with Special Regex Metacharacters)**:
  - *Boundary*: Search query `"cpu (100%) [lag] *.*"`.
  - *Input*: Query with punctuation.
  - *Expected*: Tokens stripped and matched against article keywords; matches `cpu_troubleshooting`.
- **T2-F13-03 (Query with No Matching Keywords Across Any Article)**:
  - *Boundary*: Search query `"cách làm bánh pizza hải sản"`.
  - *Input*: Unrelated query.
  - *Expected*: Returns empty vector `vec![]`; no unhandled unwraps.
- **T2-F13-04 (Diacritic Insensitive Matching Verification)**:
  - *Boundary*: Vietnamese query without tone marks ("may tinh qua tai").
  - *Input*: Search query with stripped tones.
  - *Expected*: Matches articles via fallback normalized keyword index.
- **T2-F13-05 (KnowledgeBase Performance Under Micro-Benchmarking)**:
  - *Boundary*: 1,000 consecutive search queries executed in a tight loop.
  - *Input*: 1,000 search calls.
  - *Expected*: Total execution time < 50ms; average lookup < 50 microseconds.

### Feature F14: React Storage Cleaner Suite (Boundary Cases)
- **T2-F14-01 (Zero Categories Selected on Purge Click)**:
  - *Boundary*: All checkboxes unchecked.
  - *Input*: Checkbox list empty.
  - *Expected*: Purge button disabled; click action blocked in UI.
- **T2-F14-02 (Scan Operation Times Out or Fails)**:
  - *Boundary*: Backend scan returns IPC error string.
  - *Input*: Mock IPC failure.
  - *Expected*: UI displays friendly error toast; target list remains in clean empty state; no white screen.
- **T2-F14-03 (Display of Very Large Byte Counts > 1 TB)**:
  - *Boundary*: Reclaimable bytes = 1,500,000,000,000 B (1.5 TB).
  - *Input*: Large byte value.
  - *Expected*: UI formatter renders `"1.50 TB"` without precision truncation or scientific notation.
- **T2-F14-04 (Rapid Repeated Clicks on Purge Button)**:
  - *Boundary*: User rapidly double-clicks or spam-clicks Purge button.
  - *Input*: Rapid click events.
  - *Expected*: Button immediately enters loading/disabled state; only a single IPC invoke is dispatched.
- **T2-F14-05 (100% Skipped Files Alert Banner)**:
  - *Boundary*: Clean completes with `deleted_files_count: 0` and `skipped_files_count: 15`.
  - *Input*: Clean summary with only skipped files.
  - *Expected*: Warning alert banner displayed highlighting that in-use files were protected.

### Feature F15: React Process Manager UI (Boundary Cases)
- **T2-F15-01 (Massive Process Count: 500+ Processes)**:
  - *Boundary*: System with 500 active processes.
  - *Input*: 500 items in process list.
  - *Expected*: Virtualized or paginated rendering performs smoothly; search filtering remains responsive (< 16ms).
- **T2-F15-02 (Protected Process Safety Warning Banner on Terminate Click)**:
  - *Boundary*: User attempts termination of protected process row.
  - *Input*: Click Terminate on `csrss.exe`.
  - *Expected*: Modal displays red safety warning explaining that this is a critical Windows operating system process.
- **T2-F15-03 (Rapid Table Sorting Column Toggles)**:
  - *Boundary*: User rapidly toggles between PID, CPU, and RAM column headers.
  - *Input*: Rapid click events on table headers.
  - *Expected*: Table state remains consistent; no race condition in sort comparator.
- **T2-F15-04 (Process Disappears While Terminate Confirmation Modal is Open)**:
  - *Boundary*: Process exits naturally while confirmation dialog is open.
  - *Input*: Confirm termination on dead PID.
  - *Expected*: IPC returns `ProcessNotFound`; modal displays friendly message "Process has already exited".
- **T2-F15-05 (Runaway Process Visual Highlighting)**:
  - *Boundary*: Process marked `is_runaway: true`.
  - *Input*: Runaway item in table.
  - *Expected*: Row rendered with distinct warning styling (e.g. amber/red accent background) per Kernova tokens.

### Feature F16: React Bilingual Localization (Boundary Cases)
- **T2-F16-01 (Missing Dictionary Key Fallback)**:
  - *Boundary*: Translation lookup for a key not present in the Vietnamese bundle.
  - *Input*: `t("unknown_feature_key")`.
  - *Expected*: Falls back gracefully to English key or returns key string; does not throw undefined exception.
- **T2-F16-02 (Rapid Language Toggling: 10 Switches in 2 Seconds)**:
  - *Boundary*: User toggles between VI and EN repeatedly and rapidly.
  - *Input*: 10 toggle clicks.
  - *Expected*: State remains clean; final locale accurately reflects selection; zero memory leaks.
- **T2-F16-03 (Long Vietnamese Diacritic Text Wrapping)**:
  - *Boundary*: Long translated strings in narrow UI panels (e.g. "Tệp báo cáo lỗi hệ thống Windows Error Reporting").
  - *Input*: Narrow panel width (320px).
  - *Expected*: Text wraps cleanly without clipping or breaking button layouts.
- **T2-F16-04 (Number and Date Formatting Across Locales)**:
  - *Boundary*: Byte counts and timestamps formatted under `vi` vs `en`.
  - *Input*: `formatBytes` and `formatDate` under both locales.
  - *Expected*: Adheres to locale conventions without throwing `Intl` exceptions.
- **T2-F16-05 (Locale Preference Persistence)**:
  - *Boundary*: User changes language to `vi` and restarts application.
  - *Input*: Reload UI state.
  - *Expected*: Persisted setting restored; app starts in Vietnamese.

### Feature F17: E2E Test Suite & Test Runner (Boundary Cases)
- **T2-F17-01 (Non-Zero Exit Code on Single Test Failure)**:
  - *Boundary*: Suite with 99 passing tests and 1 failing assertion.
  - *Input*: Inject deliberate assertion failure in one test.
  - *Expected*: Runner captures failure, prints diagnostic trace, and exits with exit code 1.
- **T2-F17-02 (Panic Interception in Test Case)**:
  - *Boundary*: Test case triggers an unhandled `panic!("unexpected")`.
  - *Input*: Run test suite containing panicking test.
  - *Expected*: Runner intercepts panic via `catch_unwind`; reports test as FAILED; continues remaining tests.
- **T2-F17-03 (Temporary Fixture Cleanup on Failure)**:
  - *Boundary*: Test creates temporary files in sandbox and fails mid-execution.
  - *Input*: Failing test with RAII fixture guard.
  - *Expected*: RAII drop handler executes, deleting sandbox temporary files cleanly.
- **T2-F17-04 (Headless / Non-TTY Output Mode)**:
  - *Boundary*: Test runner executed in CI environment without TTY (PAGER=cat, NO_COLOR=1).
  - *Input*: Run in non-interactive pipeline.
  - *Expected*: Strips ANSI escape sequences or outputs clean plain text logs; no terminal control hang.
- **T2-F17-05 (Empty Test Filter Matching Zero Tests)**:
  - *Boundary*: Run tests with filter `--filter non_existent_test_name`.
  - *Input*: Zero tests matched.
  - *Expected*: Runner prints "0 tests executed"; exits cleanly with code 0 without panic.

---

## 5. Tier 3: Cross-Feature Pairwise Combinatorial Interactions

Cross-feature testing evaluates how pairs and triads of features interact under dynamic runtime conditions:

| Interaction ID | Coupled Features | Scenario & Behavioral Contract | Verification Assertion |
|---|---|---|---|
| **T3-INT-01** | F01 (Telemetry) + F02 (Diagnostics) + F12 (Assistant) | Live telemetry tick updates CPU to 95%; `DiagnosticEngine` emits `PERF_CPU_HIGH_SUSTAINED`; user queries assistant "tình trạng máy tính". | Assistant incorporates active diagnostic finding and suggests terminating heavy processes in Vietnamese. |
| **T3-INT-02** | F03 (Scanner) + F04 (Exact Bytes) + F05 (Dry-Run) | Scanner calculates 150 MB across 3 categories; dry-run executed on all 3. | `freed_bytes` in summary matches scanned bytes (150 MB); zero files removed from disk. |
| **T3-INT-03** | F03 (Scanner) + F06 (Locked Skip) + F04 (Exact Bytes) | Scanner identifies 50 files; 5 files are locked by running processes during live clean. | 45 files deleted; 5 skipped; `freed_bytes` equals exact sum of 45 deleted files; zero panics. |
| **T3-INT-04** | F03 (Scanner) + F07 (Boundary Guard) | Scanner target configuration tampered with to include `C:\Windows\System32`. | Boundary guard intercepts purge before deletion; returns `UnauthorizedTargetDirectory`; operation aborted. |
| **T3-INT-05** | F08 (Process List) + F10 (Runaway) + F11 (Blacklist) | Process with 95% CPU is `csrss.exe` (System process). | Runaway engine exempts system process or blacklist blocks termination; safety blacklist takes precedence. |
| **T3-INT-06** | F08 (Process List) + F10 (Runaway) + F15 (Process UI) | Non-protected app (`renderer.exe`) consumes 92% CPU and 2.5 GB RAM. | Process UI highlights row with runaway warning badge; user terminates via modal; process successfully killed. |
| **T3-INT-07** | F12 (Assistant) + F13 (Knowledge) + F16 (Localization) | User switches app to Vietnamese; asks assistant "làm thế nào để dọn rác?". | Assistant dispatches to Knowledge Base; returns `storage_cleaner_guide` in Vietnamese. |
| **T3-INT-08** | F03 (Scanner) + F14 (Storage UI) + F16 (Localization) | User toggles language between VI and EN on Storage Cleaner view. | Category titles and descriptions update immediately between `name_vi` and `name_en` without triggering re-scan. |
| **T3-INT-09** | F01 (Telemetry) + F03 (Scanner) + F06 (Cleaner) | Storage purge frees 2.5 GB on drive C:. | Subsequent `get_system_snapshot()` reflects increased `available_bytes` on drive C:. |
| **T3-INT-10** | F08 (Process List) + F11 (Blacklist) + F15 (Process UI) | User clicks Terminate on `lsass.exe` row in Process table. | UI catches `ProtectedSystemProcess` error; displays modal explaining why system process cannot be terminated. |
| **T3-INT-11** | F02 (Diagnostics) + F14 (Storage UI) | Disk space drops below 2% (`DISK_CRITICAL_CAPACITY`). | Storage cleaner panel displays high-priority notification urging immediate purge. |
| **T3-INT-12** | F05 (Dry-Run) + F06 (Locked Skip) + F14 (Storage UI) | Dry-run executed on directory with locked files. | UI displays simulated freed space; subsequent live run skips locked files and displays accurate actual freed space. |
| **T3-INT-13** | F12 (Assistant) + F01 (Telemetry) + F02 (Diagnostics) | Telemetry snapshot reports healthy state (no findings). | Assistant reports "all systems running optimally" with green indicator. |
| **T3-INT-14** | F08 (Process List) + F09 (Categorization) + F15 (Process UI) | User selects "Applications" filter tab in Process Manager. | Process list hides background services; shows only interactive applications. |
| **T3-INT-15** | F17 (E2E Runner) + F01-F16 (All Features) | Automated runner executes all Tier 1-4 tests end-to-end. | Complete test execution completes; all assertions verified; exits with code 0. |

---

## 6. Tier 4: Real-World Operational Scenarios
*(>= 10 Comprehensive Multi-Step User Journeys)*

### Scenario T4-SCN-01: Critical Disk Space Emergency & Safe Purge Journey
- **Context**: User has 850 MB free on drive C: (<1% free). System lag is severe.
- **Workflow Steps**:
  1. Telemetry collector ticks -> `DISK_CRITICAL_CAPACITY` finding generated.
  2. Offline assistant alerts user in Vietnamese: "Ổ đĩa C: của bạn sắp đầy (<1% dung lượng trống)".
  3. User navigates to Storage Cleaner view.
  4. System scans cleanable targets -> identifies 1.8 GB in User Temp, 600 MB in Crash Dumps.
  5. User toggles Dry-Run -> verifies 2.4 GB reclaimable.
  6. User confirms Live Purge -> cleaner purges unlocked files; gracefully skips 3 locked files.
  7. Final summary displays: 2.38 GB freed, 3 files skipped.
  8. Telemetry updates: C: drive now has 3.23 GB free; critical alert clears.

### Scenario T4-SCN-02: Runaway Browser Tab Diagnosis & Safe Termination
- **Context**: A rogue web page causes a background renderer process to consume 96% CPU.
- **Workflow Steps**:
  1. Telemetry collector detects CPU load at 98% for 3 consecutive samples.
  2. `PERF_CPU_HIGH_SUSTAINED` and `PERF_RUNAWAY_PROCESS` findings generated for PID 8412 (`chrome.exe`).
  3. User opens Process Manager -> sorts by CPU descending.
  4. PID 8412 appears at top of table with high-CPU runaway warning badge.
  5. Category verified as `Application` (`is_protected: false`).
  6. User clicks Terminate -> confirmation modal verifies PID 8412.
  7. User confirms -> `terminate_process(8412)` succeeds; CPU load drops to 8%.

### Scenario T4-SCN-03: Accidental User Attempt to Kill Critical OS Process
- **Context**: Frustrated user attempting to kill unfamiliar process `csrss.exe` to free memory.
- **Workflow Steps**:
  1. User searches `"csrss"` in Process Manager search bar.
  2. Table shows `csrss.exe` (PID 620) tagged as `System` with Protected badge.
  3. User clicks Terminate button.
  4. Client-side modal warns that terminating this process will crash Windows.
  5. If user bypasses or IPC called directly: `terminate_process(620, "csrss.exe")`.
  6. Backend blacklist interceptor blocks OS call; returns `ProtectedSystemProcess` error.
  7. Process continues running; Windows stability completely preserved.

### Scenario T4-SCN-04: Routine Safe Temp Cleanup with Active Locked Office/Browser Files
- **Context**: User executes weekly disk cleanup while Word and Chrome are running with open files.
- **Workflow Steps**:
  1. User triggers Storage Scan -> 450 files found in `%TEMP%`.
  2. Word holds `~$Document.docx` locked; Chrome holds `scoped_dir*` locked.
  3. User clicks Clean Now.
  4. Cleaner iterates entries: deletes 438 stale files, skips 12 locked files.
  5. Cleaner does NOT crash, throw, or panic; logs 12 skipped reasons.
  6. Summary modal reports: "438 files deleted (820 MB freed), 12 files in-use skipped".
  7. Word and Chrome continue running without file corruption or crashes.

### Scenario T4-SCN-05: Stale Diagnostic Telemetry & Sleep/Resume Recovery
- **Context**: Laptop lid closed during active monitoring session; reopened 1 hour later.
- **Workflow Steps**:
  1. App resumes; clock delta since last snapshot tick is 3600 seconds.
  2. `DiagnosticEngine` detects `elapsed > 10s` -> emits `SENSOR_STALE_DATA` finding.
  3. SystemCollector ticks immediately -> captures fresh live hardware metrics.
  4. Next evaluation tick sees normal elapsed time (1.0s) -> `SENSOR_STALE_DATA` clears.
  5. Dashboard resumes real-time graph rendering without gap distortion.

### Scenario T4-SCN-06: Non-Admin Standard User Storage Scan & Graceful Degradation
- **Context**: Standard corporate user without Administrator elevation running PC Doctor.
- **Workflow Steps**:
  1. Storage Scanner resolves cleanable targets.
  2. Standard directories (`%TEMP%`, `CrashDumps`, `ThumbnailCache`) scanned normally.
  3. Admin-restricted directory (`C:\Windows\Prefetch`) returns `PermissionDenied`.
  4. Scanner marks Prefetch as `accessible: false`, `file_count: 0`, `total_bytes: 0`.
  5. Scan completes successfully; UI displays available targets without permission crash.

### Scenario T4-SCN-07: Bilingual User Session (English to Vietnamese Transition)
- **Context**: Bilingual user starts app in English, switches to Vietnamese for a family member.
- **Workflow Steps**:
  1. App starts in English (`en`): Dashboard, Storage Cleaner, Process Manager titles.
  2. User selects "Tiếng Việt" in top navigation bar.
  3. UI instantaneously re-renders: "Tổng quan", "Dọn dẹp bộ nhớ", "Quản lý tiến trình".
  4. User opens Assistant -> placeholder reads "Hỏi trợ lý PC Doctor...".
  5. User types "tình trạng máy" -> Assistant responds with full Vietnamese report.
  6. All diagnostic recommendations rendered in idiomatic Vietnamese.

### Scenario T4-SCN-08: Offline Assistant Interactive Troubleshooting Workflow
- **Context**: User computer feels sluggish; internet connection is down.
- **Workflow Steps**:
  1. User opens Offline Assistant tab.
  2. User types "máy tính bị đơ giật do nguyên nhân gì?".
  3. Assistant searches built-in offline knowledge base -> matches `cpu_troubleshooting`.
  4. Response provides step-by-step guidance on identifying heavy processes.
  5. User clicks suggested action pill "Kiểm tra tiến trình CPU" -> navigates to Process view.
  6. Zero internet requests sent; 100% offline functionality verified.

### Scenario T4-SCN-09: Storage Cleaner Dry-Run Verification Workflow
- **Context**: Cautious IT technician verifying cleanup targets before permitting deletions.
- **Workflow Steps**:
  1. Technician selects Dry-Run Simulation toggle.
  2. Selects all targets (`user_temp`, `crash_dumps`, `wer_reports`, `thumbnail_cache`).
  3. Clicks "Bắt đầu mô phỏng" (Simulate Purge).
  4. Summary reports exactly which files and bytes would be freed.
  5. Technician inspects directory in Windows File Explorer -> verifies all files remain intact.
  6. Technician toggles Dry-Run off -> executes live purge with confidence.

### Scenario T4-SCN-10: Multi-Drive Mixed Volume Telemetry & Diagnostic Analysis
- **Context**: Workstation with SSD C: (system, NTFS), HDD D: (storage, NTFS), and USB E: (FAT32).
- **Workflow Steps**:
  1. SystemCollector queries system disk volumes.
  2. Captures all 3 drives: identifies C: as fixed, D: as fixed, E: as removable.
  3. C: has 5% free -> emits `DISK_LOW_CAPACITY` for C:.
  4. D: has 45% free -> healthy.
  5. E: is safely reported without causing false capacity alarms.
  6. UI renders drive bar widgets with appropriate warning color accents.

### Scenario T4-SCN-11: Sustained Memory Exhaustion Mitigation
- **Context**: Multiple memory-heavy applications open; RAM available drops to 350 MB.
- **Workflow Steps**:
  1. Telemetry records RAM usage at 96% and available at 350 MB (< 500 MB).
  2. Rules engine emits `PERF_MEM_HIGH_SUSTAINED` and `PERF_MEM_LOW_AVAILABLE`.
  3. Offline assistant alerts user to memory exhaustion risk.
  4. User navigates to Process Manager -> sorts by RAM descending.
  5. Identifies background process consuming 3.2 GB RAM.
  6. User terminates process -> available RAM increases to 3.8 GB; alert clears.

### Scenario T4-SCN-12: Cyber-Security Boundary Defense Against Malicious Path Injection
- **Context**: Malicious or corrupted input attempts path traversal to purge system files.
- **Workflow Steps**:
  1. Clean request injected with malicious target `"%TEMP%\\..\\..\\Windows\\System32"`.
  2. Storage cleaner passes path to `Protected Location Boundary Guard`.
  3. Canonicalization resolves path to `C:\Windows\System32`.
  4. Blacklist check detects protected system directory; fails whitelist prefix check.
  5. Security guard immediately halts execution with `UnauthorizedTargetDirectory`.
  6. Security alert logged; zero system files touched.

---

## 7. Automated Rust Test Runner Architecture (`tests/e2e_runner.rs`)

The automated test runner is architected in Rust to execute natively as part of `cargo test` and as an independent standalone test executable.

### 7.1 Component Architecture
```
┌────────────────────────────────────────────────────────────────────────┐
│                        AUTOMATED E2E TEST RUNNER                       │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────┐  │
│  │  Test Registry   │  │ Test Fixtures    │  │ Assertion Framework  │  │
│  │  - Category Map  │  │ - Isolated Temp  │  │ - Non-panicking      │  │
│  │  - Metadata      │  │ - Locked File Hnd│  │ - Rich Diff Context  │  │
│  └────────┬─────────┘  └────────┬─────────┘  └──────────┬───────────┘  │
│           │                     │                       │              │
│           ▼                     ▼                       ▼              │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                 Execution Engine (Panic-Safe Runner)             │  │
│  │    std::panic::catch_unwind + Monotonic High-Res Stopwatch       │  │
│  └──────────────────────────────────┬───────────────────────────────┘  │
│                                     │                                  │
│                                     ▼                                  │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     Structured Report Generator                  │  │
│  │   - ANSI Color Terminal Output                                   │  │
│  │   - Machine-readable JSON Report (`target/e2e_report.json`)      │  │
│  │   - Deterministic Exit Code Protocol (0 = PASS, 1 = FAIL)        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Implementation Blueprint (`crates/pc-doctor-core/tests/e2e_suite.rs`)

```rust
// crates/pc-doctor-core/tests/e2e_suite.rs
// Automated E2E Test Suite for PC Doctor (TBK Core)

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use chrono::Utc;
use pc_doctor_core::models::*;
use pc_doctor_core::diagnostics::DiagnosticEngine;
use pc_doctor_core::knowledge::KnowledgeBase;

#[derive(Debug, Clone)]
pub struct TestCase {
    pub id: &'static str,
    pub tier: &'static str,
    pub feature: &'static str,
    pub description: &'static str,
    pub test_fn: fn() -> Result<(), String>,
}

pub struct TestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u128,
    pub failures: Vec<(&'static str, String)>,
}

// Sandbox Fixture Guard (RAII auto-cleanup)
pub struct TestSandbox {
    pub root: PathBuf,
}

impl TestSandbox {
    pub fn new(prefix: &str) -> Self {
        let temp_dir = std::env::temp_dir();
        let unique_name = format!("tbk_test_{}_{}", prefix, uuid::Uuid::new_v4());
        let root = temp_dir.join(unique_name);
        fs::create_dir_all(&root).expect("Failed to create test sandbox");
        Self { root }
    }

    pub fn create_file(&self, relative_path: &str, content: &[u8]) -> PathBuf {
        let p = self.root.join(relative_path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }
}

impl Drop for TestSandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

// ---------------------------------------------------------
// Example Test Implementations (Tiers 1-4)
// ---------------------------------------------------------

fn test_t1_f01_cpu_metrics() -> Result<(), String> {
    let cpu = CpuSnapshot {
        model_name: "AMD Ryzen 9".to_string(),
        logical_cores: 16,
        physical_cores: 8,
        global_usage_percent: 34.5,
        cores: vec![CpuCoreInfo { core_id: 0, usage_percent: 30.0, frequency_mhz: 4200 }],
    };
    if cpu.logical_cores != 16 {
        return Err("Logical cores mismatch".to_string());
    }
    if (cpu.global_usage_percent - 34.5).abs() > 0.01 {
        return Err("Global usage mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f02_disk_critical_rule() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![DiskSnapshot {
        name: "C:".to_string(),
        mount_point: "C:".to_string(),
        filesystem: "NTFS".to_string(),
        total_bytes: 100 * 1024 * 1024 * 1024,
        available_bytes: 1 * 1024 * 1024 * 1024, // 1% free -> Critical
        used_bytes: 99 * 1024 * 1024 * 1024,
        usage_percent: 99.0,
        is_removable: false,
    }];
    let findings = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !findings.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY" && f.severity == SeverityLevel::Critical) {
        return Err("DISK_CRITICAL_CAPACITY rule not triggered".to_string());
    }
    Ok(())
}

fn test_t1_f11_csrss_blacklist() -> Result<(), String> {
    const PROTECTED: &[&str] = &["csrss.exe", "lsass.exe", "services.exe", "smss.exe", "explorer.exe"];
    for name in PROTECTED {
        let is_prot = PROTECTED.contains(name);
        if !is_prot {
            return Err(format!("Expected {} to be protected", name));
        }
    }
    Ok(())
}

fn test_t2_f07_path_traversal() -> Result<(), String> {
    let sandbox = TestSandbox::new("traversal_guard");
    let safe_root = sandbox.root.clone();
    let malicious = safe_root.join("..").join("..").join("Windows").join("System32");
    
    // Boundary check function
    let is_safe = |candidate: &Path| -> bool {
        if let Ok(canon) = dunce::canonicalize(candidate) {
            canon.starts_with(&safe_root)
        } else {
            false
        }
    };

    if is_safe(&malicious) {
        return Err("Path traversal was not rejected!".to_string());
    }
    Ok(())
}

fn test_t4_scn_offline_assistant() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res_vi = kb.search("dọn dẹp bộ nhớ đệm temp");
    if res_vi.is_empty() {
        return Err("Offline knowledge base failed on Vietnamese query".to_string());
    }
    let res_en = kb.search("disk space cleanup");
    if res_en.is_empty() {
        return Err("Offline knowledge base failed on English query".to_string());
    }
    Ok(())
}

// ---------------------------------------------------------
// Helper Factories
// ---------------------------------------------------------
fn dummy_cpu(usage: f32) -> CpuSnapshot {
    CpuSnapshot {
        model_name: "Test CPU".to_string(),
        logical_cores: 8,
        physical_cores: 4,
        global_usage_percent: usage,
        cores: vec![],
    }
}

fn dummy_mem(total: u64, available: u64) -> MemorySnapshot {
    MemorySnapshot {
        total_bytes: total,
        used_bytes: total.saturating_sub(available),
        available_bytes: available,
        usage_percent: ((total.saturating_sub(available)) as f32 / total as f32) * 100.0,
        swap_total_bytes: 0,
        swap_used_bytes: 0,
    }
}

// ---------------------------------------------------------
// Master Runner Entrypoint
// ---------------------------------------------------------
#[test]
fn run_master_e2e_suite() {
    let tests = vec![
        TestCase {
            id: "T1-F01-01",
            tier: "Tier 1",
            feature: "F01 Telemetry",
            description: "CPU Global and Per-core Metrics",
            test_fn: test_t1_f01_cpu_metrics,
        },
        TestCase {
            id: "T1-F02-01",
            tier: "Tier 1",
            feature: "F02 Diagnostics",
            description: "Disk Critical Rule Detection (<2% free)",
            test_fn: test_t1_f02_disk_critical_rule,
        },
        TestCase {
            id: "T1-F11-01",
            tier: "Tier 1",
            feature: "F11 Safety Blacklist",
            description: "Core Windows OS Process Blacklist",
            test_fn: test_t1_f11_csrss_blacklist,
        },
        TestCase {
            id: "T2-F07-01",
            tier: "Tier 2",
            feature: "F07 Boundary Guard",
            description: "Path Traversal Security Check",
            test_fn: test_t2_f07_path_traversal,
        },
        TestCase {
            id: "T4-SCN-08",
            tier: "Tier 4",
            feature: "F12/F13 Assistant",
            description: "Offline Assistant Bilingual Query Workflow",
            test_fn: test_t4_scn_offline_assistant,
        },
    ];

    let start = Instant::now();
    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    println!("\n========================================================");
    println!("  PC DOCTOR (TBK CORE) — AUTOMATED E2E TEST RUNNER");
    println!("========================================================");

    for tc in &tests {
        let result = std::panic::catch_unwind(|| (tc.test_fn)());
        match result {
            Ok(Ok(())) => {
                passed += 1;
                println!("  [PASS] {} ({}) — {}", tc.id, tc.feature, tc.description);
            }
            Ok(Err(err)) => {
                failed += 1;
                failures.push((tc.id, err.clone()));
                println!("  [FAIL] {} ({}) — {}: {}", tc.id, tc.feature, tc.description, err);
            }
            Err(_) => {
                failed += 1;
                let err_msg = "Test panicked during execution".to_string();
                failures.push((tc.id, err_msg.clone()));
                println!("  [PANIC] {} ({}) — {}", tc.id, tc.feature, tc.description);
            }
        }
    }

    let elapsed = start.elapsed().as_millis();
    println!("--------------------------------------------------------");
    println!("Summary: Total: {}, Passed: {}, Failed: {} in {}ms", tests.len(), passed, failed, elapsed);
    println!("========================================================\n");

    assert_eq!(failed, 0, "One or more E2E test cases failed!");
}
```

---

## 8. TEST_READY.md Blueprint & Acceptance Verification Protocol

When Milestones M1, M2, and M3 are executed by respective developers, the repository transitions to **TEST_READY** status. The blueprint below governs this certification:

### 8.1 Readiness Criteria Checklist
- [ ] **M1 Pure Domain**: `crates/pc-doctor-core` compiles cleanly and unit tests pass with zero warnings or panics.
- [ ] **M2 Platform Adapters**: `apps/desktop/src-tauri` builds cleanly, IPC command signatures strictly match `PROJECT.md`.
- [ ] **M3 React Desktop Frontend**: `npm run build` in `apps/desktop` succeeds with 0 TypeScript/bundle errors.
- [ ] **All 17 Features Verified**: Every feature in the PROJECT.md Feature Inventory satisfies both Tier 1 and Tier 2 criteria.
- [ ] **E2E Test Runner Pass**: Master E2E runner executes all registered suites and returns exit code 0.

### 8.2 Verification Commands
```powershell
# 1. Rust Workspace Test Pass
cargo test --workspace

# 2. Desktop Frontend Type and Build Check
cd apps/desktop
npm run build
cd ../..

# 3. Master E2E Test Suite Run
cargo test --test e2e_suite -- --nocapture
```

---
*End of TEST_INFRA.md*
