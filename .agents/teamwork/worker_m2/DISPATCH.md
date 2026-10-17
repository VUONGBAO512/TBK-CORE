## 2026-10-16T07:47:01Z
You are the Implementation Worker for Milestone 2: Tauri Platform Adapters & IPC Commands in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/worker_m2.

You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md

Read the three Explorer handoff reports which contain the complete, tested blueprints and code:
- Storage Commands Blueprint: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc/handoff.md
- Process Commands Blueprint: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_process_ipc/handoff.md
- Collector & Tauri Lib Blueprint: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_collector_lib/handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

File Ownership:
You have exclusive write ownership of:
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/Cargo.toml (add tempfile = "3" to [dev-dependencies])
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/src/commands/storage.rs (create new)
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/src/commands/process.rs (create new)
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/src/commands/mod.rs (re-export storage and process commands)
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/src/monitoring/collector.rs (add refresh_processes, get_process_list -> Vec<ProcessItem>)
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/src/lib.rs (register all 6 commands in generate_handler)

Implementation Requirements:
1. Update `apps/desktop/src-tauri/Cargo.toml` to include `tempfile = "3"` under `[dev-dependencies]`.
2. Create `apps/desktop/src-tauri/src/commands/storage.rs`:
   - Implement `scan_cleanable_storage()` using `tokio::task::spawn_blocking` calling `StorageEngine::new().scan()`.
   - Implement `clean_storage_targets(request: CleanRequest)` using `tokio::task::spawn_blocking` calling `StorageEngine::new().clean(&request)`.
   - Include unit tests verifying `scan_cleanable_storage` and `clean_storage_targets` with mock resolvers / temp directories.
3. Create `apps/desktop/src-tauri/src/commands/process.rs`:
   - Implement `get_process_list(state: State<'_, Arc<CollectorState>>) -> Result<Vec<ProcessItem>, String>`: locks collector, calls `refresh_processes()`, and returns `get_process_list()`.
   - Implement `terminate_process(pid: u32, process_name: String) -> Result<TerminateResult, String>`:
     - Validates through `ProcessGuard::new().validate_termination(pid, &process_name)`.
     - Rejects protected processes with structured error (`ProcessTerminationError::ProtectedSystemProcess`).
     - Defense-in-depth: checks real OS process name from sysinfo if running, re-validating with guard.
     - Kills process via `sysinfo::Process::kill()` if approved, returning `TerminateResult`.
   - Implement `terminate_process_with_guard` helper function for unit testing.
   - Include unit tests verifying rejection of protected processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, PID <= 4, self PID).
4. Update `apps/desktop/src-tauri/src/commands/mod.rs`:
   - Declare `pub mod storage;` and `pub mod process;`.
   - Re-export `storage::{clean_storage_targets, scan_cleanable_storage};` and `process::{get_process_list, terminate_process};`.
   - Preserve `get_system_snapshot` and `query_assistant`.
5. Update `apps/desktop/src-tauri/src/monitoring/collector.rs`:
   - Add `refresh_processes(&mut self)` updating sysinfo process data.
   - Add `get_process_list(&self) -> Vec<ProcessItem>` iterating processes, categorizing with `categorize_process`, checking `guard.is_protected(pid, &name)`, and sorting by CPU then memory.
6. Update `apps/desktop/src-tauri/src/lib.rs`:
   - Update `tauri::generate_handler![get_system_snapshot, get_process_list, terminate_process, scan_cleanable_storage, clean_storage_targets, query_assistant]`.
7. Verification:
   - Run `cargo check --workspace`
   - Run `cargo test --workspace`
   - Ensure 0 compilation errors, 0 warnings, and all tests pass (both core and desktop crates).
8. Write your complete handoff report to `d:/HOCC++/pc-doctor/.agents/teamwork/worker_m2/handoff.md`.
   Include all files modified/created, test execution outputs, and verification details.
   Send a message to your caller when done.
