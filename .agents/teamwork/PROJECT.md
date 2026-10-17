# Project: PC Doctor (TBK Core) — Actionable Diagnostics & Safe Cleanup Suite

## Architecture
- **crates/pc-doctor-core**: Pure domain models, diagnostic rules, safe storage engine (targets, scanning, dry-run, locked-file handling, protected boundary validation), process manager (categorization, runaway rules, safety blacklist), and bilingual knowledge base. Completely decoupled from Tauri and I/O where possible.
- **apps/desktop/src-tauri**: Platform execution layer. System telemetry via `sysinfo`, file system deletion routines, process termination via PID, local pairing server, and Tauri 2 IPC command handlers.
- **apps/desktop**: React 19 + TypeScript + Vite desktop UI matching Kernova design system tokens. Native desktop views for Dashboard, Storage Cleaner, Process Manager, Offline Assistant, and Settings with instant Vietnamese (`vi`) / English (`en`) localization.
- **packages/shared-types**: Shared TypeScript interfaces synchronized with `pc-doctor-core` models.

## Code Layout
- `crates/pc-doctor-core/src/models.rs` — Domain models for telemetry, diagnostics, storage, processes, and assistant
- `crates/pc-doctor-core/src/storage/` — Storage cleanable targets, safe root whitelist, protected boundary guard, scanning and purging engine
- `crates/pc-doctor-core/src/processes/` — Process categorization, runaway criteria, and hardcoded Windows OS safety blacklist
- `crates/pc-doctor-core/src/knowledge/` — Bilingual knowledge base and offline troubleshooting articles
- `apps/desktop/src-tauri/src/monitoring/collector.rs` — SystemCollector gathering telemetry and process list
- `apps/desktop/src-tauri/src/commands/` — Tauri IPC command implementations
- `apps/desktop/src/api/ipc.ts` — Typed IPC wrappers for Tauri invoke
- `apps/desktop/src/components/storage/StorageCleanerPanel.tsx` — Storage cleaner view with itemized targets, dry-run toggle, purge modal, results summary
- `apps/desktop/src/components/processes/ProcessPanel.tsx` — Process inspection table with category tabs, protected badges, termination modal
- `apps/desktop/src/i18n/{vi,en}.ts` — Bilingual dictionary bundles

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Hardware Telemetry Metrics | CPU (global + per-core), RAM, Disks, OS, Uptime, Sensors | M1, M2 | R1 |
| 2 | Deterministic Diagnostic Rules | 8+ deterministic rules with 5 severity tiers and rolling history | M1, M2 | R1 |
| 3 | Safe Temp Cache Scanner | Scans %TEMP%, CrashDumps, WER, Explorer thumbnails, logs | M1, M2 | R2 |
| 4 | Exact Reclaimable Byte Sizes | Non-negative byte calculation with itemized breakdown | M1, M2 | R2 |
| 5 | Dry-Run Simulation Mode | Calculates reclaimable space without deleting any files | M1, M2 | R2 |
| 6 | Locked/In-Use File Skip | Safely skips locked files without panicking, reports freed bytes | M1, M2 | R2 |
| 7 | Protected Location Boundary Guard | Enforces boundary check rejecting %WINDIR%, System32, Desktop, etc. | M1, M2 | R2 |
| 8 | Process Listing & Sorting | Query running processes sorted by CPU and memory consumption | M1, M2 | R3 |
| 9 | Process Categorization | Categorizes processes into app, service, system, browser | M1, M2 | R3 |
| 10 | Runaway Process Detection | Detects processes exceeding healthy resource thresholds | M1, M2 | R3 |
| 11 | Process Safety Blacklist | Hardcoded blacklist (csrss, lsass, services, explorer, smss) | M1, M2 | R3 |
| 12 | Offline Companion Assistant | Local context-aware diagnostics and guidance | M1, M2 | R4 |
| 13 | Bilingual Knowledge Base | Bilingual troubleshooting articles in Vietnamese and English | M1, M2 | R4 |
| 14 | React Storage Cleaner Suite | Full UI with itemized breakdown, dry-run toggle, purge confirmation | M3 | R2 |
| 15 | React Process Manager UI | Category tabs, protected status badges, terminate button & modal | M3 | R3 |
| 16 | React Bilingual Localization | Seamless vi/en switching across all UI views and assistant | M3 | R4 |
| 17 | E2E Test Suite & Test Runner | Automated opaque-box test runner covering Tiers 1-4 | E2E Track, M4 | Acceptance Criteria |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Core Domain Logic & Safety Engines | `crates/pc-doctor-core` models, storage engine, process blacklist, and bilingual knowledge | None | DONE |
| M2 | Tauri Platform Adapters & IPC Commands | `apps/desktop/src-tauri` commands, collector integration, workspace Cargo fix | M1 | DONE |
| M3 | Frontend UI & Bilingual Localization | `apps/desktop` React components, IPC client, Kernova styling, i18n | M2 | DONE |
| M4 | Final Milestone: E2E Pass & Hardening | Pass 100% E2E tests (Tiers 1-4) + Tier 5 Adversarial Coverage Hardening | M3, E2E Track | DONE |
| E2E | E2E Testing Track | Independent opaque-box test runner and test cases (Tiers 1-4) | None (parallel) | READY |

## Interface Contracts

### 1. `pc-doctor-core::storage` ↔ `desktop_lib::commands::storage`
```rust
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

pub struct StorageScanReport {
    pub targets: Vec<CleanableTarget>,
    pub total_reclaimable_bytes: u64,
    pub total_files: usize,
    pub scanned_at: DateTime<Utc>,
}

pub struct CleanRequest {
    pub target_ids: Vec<String>,
    pub dry_run: bool,
}

pub struct StorageCleanSummary {
    pub dry_run: bool,
    pub bytes_freed: u64,
    pub files_deleted: usize,
    pub files_skipped_locked: usize,
    pub targets_processed: Vec<String>,
    pub errors: Vec<String>,
}
```

### 2. `pc-doctor-core::processes` ↔ `desktop_lib::commands::process`
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

### 3. Tauri IPC ↔ React Frontend (`apps/desktop/src/api/ipc.ts`)
- `get_system_snapshot() -> Promise<SystemSnapshot>`
- `get_process_list() -> Promise<ProcessItem[]>`
- `terminate_process(pid: number) -> Promise<TerminateResult>`
- `scan_cleanable_storage() -> Promise<StorageScanReport>`
- `clean_storage_targets(request: CleanRequest) -> Promise<StorageCleanSummary>`
- `query_assistant(prompt: string, lang: string) -> Promise<AssistantResponse>`
