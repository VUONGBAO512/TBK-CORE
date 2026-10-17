# Review Report — Milestone 2: Tauri Platform Adapters & IPC Commands

**Reviewer**: Reviewer 1 (Archetype: reviewer-critic)  
**Parent / Caller**: `d0833960-9ed1-454f-b7d6-be733f742605`  
**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/reviewer_m2_1`  
**Verdict**: **APPROVE**  
**Date**: 2026-10-16  

---

## 1. Observation

### 1.1 Codebase Inspection & Line-by-Line Evidence

1. **Storage Commands Decoupling (`apps/desktop/src-tauri/src/commands/storage.rs`)**:
   - `scan_cleanable_storage` (lines 15–23):
     ```rust
     #[tauri::command]
     pub async fn scan_cleanable_storage() -> Result<StorageScanReport, String> {
         tokio::task::spawn_blocking(|| {
             let engine = StorageEngine::new();
             scan_cleanable_storage_impl(&engine)
         })
         .await
         .map_err(|e| format!("Storage scan background task failed: {}", e))?
     }
     ```
   - `clean_storage_targets` (lines 30–40):
     ```rust
     #[tauri::command]
     pub async fn clean_storage_targets(
         request: CleanRequest,
     ) -> Result<StorageCleanSummary, String> {
         tokio::task::spawn_blocking(move || {
             let engine = StorageEngine::new();
             clean_storage_targets_impl(&engine, &request)
         })
         .await
         .map_err(|e| format!("Storage clean background task failed: {}", e))?
     }
     ```
   - Both commands offload synchronous filesystem traversal and file deletion to Tokio's dedicated thread pool via `tokio::task::spawn_blocking`, preventing starvation of Tokio async worker threads and the local companion Axum HTTP server (`server.rs` listening on `127.0.0.1:45120`).
   - Any panic or failure inside the blocking task is mapped cleanly into a `String` error via `map_err`, preventing Tauri application crashes.

2. **Interface Contracts & JSON Serialization (`PROJECT.md § Interface Contracts`)**:
   - `CleanRequest`, `StorageCleanSummary`, and `StorageScanReport` in `storage.rs` reference domain models defined in `crates/pc-doctor-core/src/models.rs`.
   - Field names and types match `PROJECT.md` exactly:
     - `CleanableTarget`: `id`, `name_en`, `name_vi`, `description_en`, `description_vi`, `root_path`, `file_count`, `total_bytes`, `is_safe`.
     - `StorageScanReport`: `targets`, `total_reclaimable_bytes`, `total_files`, `scanned_at`.
     - `CleanRequest`: `target_ids`, `dry_run`.
     - `StorageCleanSummary`: `dry_run`, `bytes_freed`, `files_deleted`, `files_skipped_locked`, `targets_processed`, `errors`.
   - Serialization contracts verified via unit test `test_ipc_json_serialization_contracts` (lines 274–297).

3. **Tauri Handler Registration (`apps/desktop/src-tauri/src/lib.rs`)**:
   - Lines 30–37 confirm all 6 required IPC commands are registered in `tauri::generate_handler!`:
     ```rust
     .invoke_handler(tauri::generate_handler![
         get_system_snapshot,
         get_process_list,
         terminate_process,
         scan_cleanable_storage,
         clean_storage_targets,
         query_assistant
     ])
     ```
   - Matches the 6 IPC functions declared in `PROJECT.md § Interface Contracts (3. Tauri IPC ↔ React Frontend)`.

4. **Dependencies (`apps/desktop/src-tauri/Cargo.toml`)**:
   - `tokio = { version = "1", features = ["full"] }` is present (line 28).
   - `tempfile = "3"` is declared under `[dev-dependencies]` (line 34).
   - `pc-doctor-core = { path = "../../../crates/pc-doctor-core" }` is linked (line 31).

5. **Process Safety & Inspection (`apps/desktop/src-tauri/src/commands/process.rs` & `monitoring/collector.rs`)**:
   - `get_process_list` retrieves all processes via `collector.refresh_processes()`, maps each process with `categorize_process`, attaches `is_protected` using `ProcessGuard`, and sorts descending by CPU percentage then memory.
   - `terminate_process` implements defense-in-depth:
     - Stage 1: Validates `(pid, &process_name)` through `ProcessGuard`. Returns serialized JSON `ProcessTerminationError::ProtectedSystemProcess` if rejected.
     - Stage 2: Queries the live OS process from `sysinfo::System`, extracts real binary name, and re-validates through `ProcessGuard` to thwart process name spoofing.
     - Stage 3: Invokes `sysinfo::Process::kill()` and returns structured `TerminateResult`.

---

## 2. Logic Chain

1. **Tokio Runtime & Axum Server Health**:
   - Observation 1.1 shows heavy, synchronous storage scans and disk purges are enclosed in `tokio::task::spawn_blocking`.
   - Therefore, the Tokio async event loop remains free to service incoming Axum HTTP requests on `127.0.0.1:45120` without latency spikes or connection dropouts.
2. **Contract Conformance**:
   - Observations 1.2 and 1.3 verify all data structures and command endpoints match `PROJECT.md § Interface Contracts` 1, 2, and 3.
   - Type definitions are synchronized between `pc-doctor-core` and `desktop_lib`.
3. **Safety & Robustness**:
   - Observations 1.1 and 1.5 verify that locked files are skipped without panicking (`files_skipped_locked`), invalid targets return structured error strings, and protected Windows processes (`csrss`, `lsass`, `services`, `explorer`, `smss`, PID <= 4, self PID) cannot be terminated even under spoofed names or case variations.
4. **Independent Build & Test Verification**:
   - `cargo check --workspace` returned exit code 0 with 0 warnings, 0 errors.
   - `cargo test --workspace` passed 89/89 tests (26 desktop + 27 core + 16 adversarial process + 20 adversarial storage) with 0 failures and 0 ignored.
   - `npm run build` in `apps/desktop` completed in 227ms with zero errors.

---

## 3. Caveats

1. Real temporary file counts and byte sizes vary across host machines, but exact arithmetic and locked-file skip logic are hermetically verified via mock resolvers and `tempfile::tempdir()` environments.
2. Frontend integration components (`StorageCleanerPanel.tsx` and `ProcessPanel.tsx`) will be fully wired in Milestone 3, but the underlying IPC endpoints and types are fully operational and verified.

---

## 4. Conclusion

**Verdict: APPROVE**

Milestone 2 implementation satisfies all requirements from `ORIGINAL_REQUEST.md` (R2, R3) and `PROJECT.md`:
- Both storage commands utilize `tokio::task::spawn_blocking` and return structured reports matching interface contracts.
- All 6 Tauri IPC commands are registered in `tauri::generate_handler!`.
- 100% test pass rate across the workspace with zero compiler warnings and zero errors.
- Frontend builds cleanly with zero TypeScript or bundler errors.
- No integrity violations or facade implementations detected.

---

## 5. Verification Method

To independently reproduce this verification:

1. **Workspace Compilation Check**:
   ```powershell
   cargo check --workspace
   ```
   *Expected*: Code 0, 0 warnings, 0 errors.

2. **Full Workspace Unit & Adversarial Tests**:
   ```powershell
   cargo test --workspace
   ```
   *Expected*: 89 passed; 0 failed; 0 ignored (26 in `desktop_lib`, 27 in `pc_doctor_core`, 16 in `adversarial_process_tests`, 20 in `adversarial_storage_tests`).

3. **Frontend Build Check**:
   ```powershell
   cd d:\HOCC++\pc-doctor\apps\desktop
   npm run build
   ```
   *Expected*: Code 0, Vite build completes with zero errors.

---

## 6. Adversarial Review & Integrity Audit

- **Integrity Violation Check**:
  - Checked for hardcoded test outputs or fake results: **None**. Tests dynamically write and measure files using `tempfile`.
  - Checked for dummy facades: **None**. Real `StorageEngine`, `SafetyGuard`, `ProcessGuard`, and `SystemCollector` are invoked.
  - Checked for task shortcuts or bypasses: **None**. Full end-to-end Rust implementation.
- **Stress-Test Scenarios Verified**:
  - PID <= 4 kernel processes (`System`, `System Idle Process`): Blocked.
  - Mandatory protected executables with case and extension variations (`CSRSS.EXE`, `csrss`, `EXPLORER.EXE`, `services`): Blocked.
  - Self-PID termination: Blocked.
  - In-use / locked files during purge: Safely skipped, counted, root preserved.
  - Dry-run mode: Calculates exact bytes without deleting disk files.
  - Non-existent target IDs: Structured error returned without panicking.
