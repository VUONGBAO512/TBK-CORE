# Handoff Report — Milestone 2: Process Tauri Commands (`desktop_lib::commands::process`)

**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_process_ipc`  
**Author**: Explorer for Milestone 2 Process Tauri Commands  
**Parent / Caller**: `d0833960-9ed1-454f-b7d6-be733f742605`  
**Date**: 2026-10-16  
**Status**: COMPLETE (Hard Handoff)  

---

## 1. Observation

Direct codebase inspection and runtime verification observations:

1. **Existing Process Domain Logic & Models (`crates/pc-doctor-core`)**:
   - `crates/pc-doctor-core/src/models.rs:216-318`:
     - `pub enum ProcessCategory { System, BackgroundService, Application, BrowserEngine }`
     - `pub struct ProcessItem { pub pid: u32, pub name: String, pub cpu_percent: f32, pub memory_bytes: u64, pub status: String, pub category: ProcessCategory, pub is_protected: bool }`
     - `pub struct TerminateResult { pub pid: u32, pub process_name: String, pub success: bool, pub message: String }`
   - `crates/pc-doctor-core/src/processes/guard.rs:5-86, 117-178`:
     - `WINDOWS_PROTECTED_PROCESSES` contains hardcoded entries: `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `wininit.exe`, `winlogon.exe`, `svchost.exe`, `dwm.exe`, `system`, `system idle process`, `registry`, etc.
     - `ProcessGuard::new()` initializes with `self_pid: std::process::id()`.
     - `ProcessGuard::with_self_pid(u32)` allows mock self-PIDs for deterministic testing.
     - `ProcessGuard::validate_termination(pid: u32, name: &str) -> Result<(), ProcessTerminationError>`:
       - Returns `Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason })` if `pid <= 4`.
       - Returns `Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason })` if `pid == self.self_pid`.
       - Returns `Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason })` if normalized name matches any entry in `WINDOWS_PROTECTED_PROCESSES`.
   - `crates/pc-doctor-core/src/processes/error.rs:4-24`:
     - `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`
     - `#[serde(tag = "error_type", content = "details")]`
     - `pub enum ProcessTerminationError { ProtectedSystemProcess { pid: u32, name: String, reason: String }, ProcessNotFound { pid: u32 }, AccessDenied { ... }, InternalError { ... } }`

2. **Existing Tauri Commands & Platform Layer (`apps/desktop/src-tauri`)**:
   - `apps/desktop/src-tauri/src/commands/mod.rs:17-24`:
     ```rust
     #[tauri::command]
     pub fn get_process_list(state: State<'_, Arc<CollectorState>>) -> Result<Vec<ProcessInfo>, String> {
         let collector = state
             .collector
             .lock()
             .map_err(|e| format!("Failed to acquire collector lock: {}", e))?;
         Ok(collector.get_top_processes(30))
     }
     ```
     Currently returns un-categorized `ProcessInfo` truncated to 30 items, lacking `ProcessItem` fields (`category`, `is_protected`).
     `terminate_process` is completely absent from `apps/desktop/src-tauri/src/commands/`.
   - `apps/desktop/src-tauri/src/lib.rs:27-31`:
     ```rust
     .invoke_handler(tauri::generate_handler![
         get_system_snapshot,
         get_process_list,
         query_assistant
     ])
     ```
     `terminate_process` is not yet registered in the invoke handler.
   - `apps/desktop/src-tauri/src/monitoring/collector.rs:160-188`:
     `SystemCollector::get_top_processes` produces `Vec<ProcessInfo>` using `sysinfo::System`.
     `sysinfo = "0.33"` is declared in `apps/desktop/src-tauri/Cargo.toml:26`.
     In `sysinfo 0.33`:
     - `sysinfo::Pid::from_u32(pid)` converts `u32` to `Pid`.
     - `sysinfo::System::refresh_processes(ProcessesToUpdate::Some(&[pid]), true)` refreshes individual process telemetry.
     - `sysinfo::Process::kill(&self) -> bool` executes process termination at the OS layer.
     - `sysinfo::Process::exe(&self) -> Option<&Path>` provides the binary executable path for categorization.

3. **Current Test Status**:
   - `cargo test -p desktop`: Ran 0 tests, passed in 0.51s.
   - `cargo check --workspace`: Passed with code 0 in 0.46s.

---

## 2. Logic Chain

1. **Decoupled Safety Gate (Observation 1 -> Observation 2)**:
   - Destructive operations (killing a process) must never be executed without prior validation.
   - `ProcessGuard::validate_termination` is a pure domain function that evaluates process safety in zero I/O cycles.
   - Therefore, in `terminate_process(pid: u32, process_name: String)`, the very first step must be:
     `ProcessGuard::new().validate_termination(pid, &process_name)`.
   - If this check fails, the command MUST return early with `Err(...)` containing the structured JSON serialization of `ProcessTerminationError::ProtectedSystemProcess`. It must NEVER call `sysinfo::Process::kill()` or even look up the OS process handle.

2. **Defense-in-Depth against Process Spoofing**:
   - An attacker or buggy frontend might pass `{ pid: 1234, process_name: "harmless.exe" }`, where PID 1234 in the OS is actually `csrss.exe` or `services.exe`.
   - By querying `sysinfo::System::process(Pid::from_u32(pid))` after initial name approval, the command retrieves the genuine OS executable name (`proc.name()`).
   - Re-running `ProcessGuard::validate_termination(pid, &real_name)` ensures that even masqueraded or falsified process names cannot bypass safety guardrails.

3. **Command Signature & Testability**:
   - The dispatch requirement specifies:
     `terminate_process(pid: u32, process_name: String) -> Result<TerminateResult, String>`.
   - By structuring the core termination logic as:
     `terminate_process_with_guard(guard: &ProcessGuard, pid: u32, process_name: &str) -> Result<TerminateResult, String>`
     and wrapping it in:
     `#[tauri::command] pub fn terminate_process(pid: u32, process_name: String) -> Result<TerminateResult, String>`
     we achieve two critical design wins:
     1. The Tauri command matches the exact parameter signature specified in the contract without requiring `tauri::State`.
     2. Unit tests inside `apps/desktop/src-tauri` can directly invoke `terminate_process(pid, name)` or `terminate_process_with_guard(&mock_guard, pid, name)` without requiring a running Tauri application or mock runtime.

4. **Process Listing & Categorization (`get_process_list`)**:
   - `get_process_list` must transition from returning `Vec<ProcessInfo>` (30 items) to `Result<Vec<ProcessItem>, String>` (all system processes).
   - In `SystemCollector`, `get_process_list(&self)` iterates all processes in `sys.processes()`, extracts `pid`, `name`, `exe_path`, `cpu_usage()`, `memory()`, calls `pc_doctor_core::processes::categorize_process(pid, &name, exe_path)`, queries `guard.is_protected(pid, &name)`, constructs `ProcessItem`, and sorts descending by CPU percent then memory.
   - Calling `collector.refresh_processes()` prior to taking the list guarantees that the UI receives fresh, accurate process metrics.

5. **Structured Error Representation**:
   - `ProcessTerminationError` derives `Serialize` and `Deserialize` with `#[serde(tag = "error_type", content = "details")]`.
   - Serializing `ProcessTerminationError` produces:
     `{"error_type":"ProtectedSystemProcess","details":{"pid":4,"name":"System","reason":"..."}}`.
   - In Rust unit tests, this string can be parsed back into `ProcessTerminationError` via `serde_json::from_str`.
   - On the React frontend, TypeScript callers in `api/ipc.ts` can catch the rejection and inspect `JSON.parse(err as string)`.

---

## 3. Caveats

1. **OS Elevation / UAC (Access Denied)**:
   - Even when a process is approved by `ProcessGuard` (e.g. an unprotected application), `proc.kill()` may return `false` if PC Doctor runs as a standard user and the target process runs under another user or `NT AUTHORITY\SYSTEM`. In this case, `terminate_process` returns `Ok(TerminateResult::failure(pid, name, "... Access denied ..."))`.
2. **Race Conditions in Process Lifetimes**:
   - A process may terminate naturally between validation and `proc.kill()`. The implementation handles this gracefully by checking `sys.process(sys_pid)` and returning `Ok(TerminateResult::failure(pid, name, "Process with PID ... was not found."))`.
3. **No Direct Code Changes by Explorer**:
   - In accordance with the Explorer archetype constraint, no files outside `.agents/teamwork/explorer_m2_process_ipc` have been modified during this investigation. The blueprint below is ready for implementation by the builder/worker agent.

---

## 4. Conclusion & Complete Implementation Blueprint

### File Modification Inventory

| Action | File Path | Purpose |
|---|---|---|
| **CREATE** | `apps/desktop/src-tauri/src/commands/process.rs` | Implements `collect_process_items`, `terminate_process_with_guard`, `get_process_list`, `terminate_process`, and the 12+ unit test suite. |
| **MODIFY** | `apps/desktop/src-tauri/src/commands/mod.rs` | Re-exports `pub mod process;` and delegates `get_process_list` and `terminate_process`. |
| **MODIFY** | `apps/desktop/src-tauri/src/monitoring/collector.rs` | Adds `get_process_list(&self) -> Vec<ProcessItem>` and `refresh_processes(&mut self)`. |
| **MODIFY** | `apps/desktop/src-tauri/src/lib.rs` | Registers `terminate_process` in `tauri::generate_handler!`. |

---

### File 1: `apps/desktop/src-tauri/src/commands/process.rs` (CREATE NEW FILE)

```rust
use crate::monitoring::CollectorState;
use pc_doctor_core::models::{ProcessItem, TerminateResult};
use pc_doctor_core::processes::{categorize_process, ProcessGuard, ProcessTerminationError};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::State;
use std::sync::Arc;

/// Collects and categorizes all system processes, attaching safety guard protection status.
/// Sorted descending by CPU percentage, fallback to memory consumption.
pub fn collect_process_items(sys: &System, guard: &ProcessGuard) -> Vec<ProcessItem> {
    let mut list: Vec<ProcessItem> = sys
        .processes()
        .iter()
        .map(|(pid, proc)| {
            let pid_u32 = pid.as_u32();
            let name = proc.name().to_string_lossy().to_string();
            let exe_path = proc.exe().and_then(|p| p.to_str());
            let category = categorize_process(pid_u32, &name, exe_path);
            let is_protected = guard.is_protected(pid_u32, &name);

            ProcessItem {
                pid: pid_u32,
                name,
                cpu_percent: proc.cpu_usage(),
                memory_bytes: proc.memory(),
                status: format!("{:?}", proc.status()),
                category,
                is_protected,
            }
        })
        .collect();

    list.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
    });

    list
}

/// Core safe process termination logic with injected ProcessGuard.
/// Validates process against hardcoded OS blacklist, PID <= 4, and self PID.
/// If blocked, returns structured JSON ProcessTerminationError without touching OS process.
/// If approved, refreshes target process in sysinfo and invokes Process::kill().
pub fn terminate_process_with_guard(
    guard: &ProcessGuard,
    pid: u32,
    process_name: &str,
) -> Result<TerminateResult, String> {
    // 1. Validate termination against ProcessGuard
    if let Err(err) = guard.validate_termination(pid, process_name) {
        let err_json = serde_json::to_string(&err)
            .unwrap_or_else(|_| format!(r#"{{"error_type":"ProtectedSystemProcess","details":{{"pid":{},"name":"{}","reason":"{}"}}}}"#, pid, process_name, err));
        return Err(err_json);
    }

    // 2. Query target process from OS via sysinfo
    let mut sys = System::new();
    let sys_pid = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);

    match sys.process(sys_pid) {
        Some(proc) => {
            // Defense-in-depth: check actual OS binary name against ProcessGuard
            let real_name = proc.name().to_string_lossy().to_string();
            if let Err(err) = guard.validate_termination(pid, &real_name) {
                let err_json = serde_json::to_string(&err)
                    .unwrap_or_else(|_| format!(r#"{{"error_type":"ProtectedSystemProcess","details":{{"pid":{},"name":"{}","reason":"{}"}}}}"#, pid, real_name, err));
                return Err(err_json);
            }

            let effective_name = if process_name.trim().is_empty() {
                real_name
            } else {
                process_name.to_string()
            };

            // 3. Invoke sysinfo::Process::kill()
            let killed = proc.kill();
            if killed {
                Ok(TerminateResult::success(
                    pid,
                    effective_name.clone(),
                    format!("Process '{}' (PID {}) was successfully terminated.", effective_name, pid),
                ))
            } else {
                Ok(TerminateResult::failure(
                    pid,
                    effective_name.clone(),
                    format!("Failed to terminate process '{}' (PID {}). Access denied or process is already exiting.", effective_name, pid),
                ))
            }
        }
        None => {
            let effective_name = if process_name.trim().is_empty() {
                format!("PID {}", pid)
            } else {
                process_name.to_string()
            };
            Ok(TerminateResult::failure(
                pid,
                effective_name,
                format!("Process with PID {} was not found.", pid),
            ))
        }
    }
}

/// Tauri IPC command: returns complete list of running processes with categorization and protection flags.
#[tauri::command]
pub fn get_process_list(
    state: State<'_, Arc<CollectorState>>,
) -> Result<Vec<ProcessItem>, String> {
    let mut collector = state
        .collector
        .lock()
        .map_err(|e| format!("Failed to acquire collector lock: {}", e))?;
    collector.refresh_processes();
    Ok(collector.get_process_list())
}

/// Tauri IPC command: terminates a process by PID and process_name after validating safety guardrails.
#[tauri::command]
pub fn terminate_process(pid: u32, process_name: String) -> Result<TerminateResult, String> {
    let guard = ProcessGuard::new();
    terminate_process_with_guard(&guard, pid, &process_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_doctor_core::models::ProcessCategory;

    // ==========================================
    // 1. Mandatory Windows Blacklist Rejection Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_csrss() {
        let res = terminate_process(1001, "csrss.exe".to_string());
        assert!(res.is_err(), "csrss.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("csrss.exe"));
        assert!(err_json.contains("CRITICAL_PROCESS_DIED"));

        let err: ProcessTerminationError = serde_json::from_str(&err_json).expect("Must deserialize into ProcessTerminationError");
        match err {
            ProcessTerminationError::ProtectedSystemProcess { pid, name, reason } => {
                assert_eq!(pid, 1001);
                assert_eq!(name, "csrss.exe");
                assert!(!reason.is_empty());
            }
            _ => panic!("Expected ProtectedSystemProcess variant"),
        }
    }

    #[test]
    fn test_terminate_process_rejects_lsass() {
        let res = terminate_process(1002, "lsass.exe".to_string());
        assert!(res.is_err(), "lsass.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("lsass.exe"));
    }

    #[test]
    fn test_terminate_process_rejects_services() {
        let res = terminate_process(1003, "services.exe".to_string());
        assert!(res.is_err(), "services.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("services.exe"));
    }

    #[test]
    fn test_terminate_process_rejects_explorer() {
        let res = terminate_process(1004, "explorer.exe".to_string());
        assert!(res.is_err(), "explorer.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("explorer.exe"));
    }

    #[test]
    fn test_terminate_process_rejects_smss() {
        let res = terminate_process(1005, "smss.exe".to_string());
        assert!(res.is_err(), "smss.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("smss.exe"));
    }

    // ==========================================
    // 2. PID 4 and Kernel Boundary Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_pid_four_system() {
        let res = terminate_process(4, "System".to_string());
        assert!(res.is_err(), "PID 4 System must be rejected");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("\"pid\":4"));
        assert!(err_json.contains("kernel process"));
    }

    #[test]
    fn test_terminate_process_rejects_pid_four_with_arbitrary_name() {
        let res = terminate_process(4, "my_random_app.exe".to_string());
        assert!(res.is_err(), "PID 4 must be rejected regardless of passed name");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("\"pid\":4"));
    }

    #[test]
    fn test_terminate_process_rejects_pid_zero_idle() {
        let res = terminate_process(0, "System Idle Process".to_string());
        assert!(res.is_err(), "PID 0 must be rejected");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("\"pid\":0"));
    }

    // ==========================================
    // 3. Self PID Protection Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_self_pid() {
        let self_pid = std::process::id();
        let res = terminate_process(self_pid, "TBK_Core.exe".to_string());
        assert!(res.is_err(), "PC Doctor must not terminate its own PID");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains(&self_pid.to_string()));
        assert!(err_json.contains("own application process"));
    }

    #[test]
    fn test_terminate_process_rejects_self_pid_with_mock_guard() {
        let mock_pid = 77777;
        let mock_guard = ProcessGuard::with_self_pid(mock_pid);
        let res = terminate_process_with_guard(&mock_guard, mock_pid, "anything.exe");
        assert!(res.is_err());
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("77777"));
        assert!(err_json.contains("own application process"));
    }

    // ==========================================
    // 4. Case Variations & Extensions Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_casing_variations() {
        let variations = [
            ("CSRSS.EXE", 2001),
            ("csrss", 2002),
            ("CsRss.Exe", 2003),
            ("EXPLORER.EXE", 2004),
            ("explorer", 2005),
            ("Explorer.exe", 2006),
            ("LSASS.EXE", 2007),
            ("lsass", 2008),
            ("SERVICES.EXE", 2009),
            ("services", 2010),
            ("SMSS.EXE", 2011),
            ("smss", 2012),
            ("dwm.exe", 2013),
            ("DWM.EXE", 2014),
            ("svchost.exe", 2015),
            ("SVCHOST.EXE", 2016),
            ("wininit.exe", 2017),
            ("winlogon.exe", 2018),
        ];

        for (name, pid) in variations {
            let res = terminate_process(pid, name.to_string());
            assert!(
                res.is_err(),
                "Process variation '{}' must be blocked from termination",
                name
            );
            let err_json = res.unwrap_err();
            assert!(
                err_json.contains("ProtectedSystemProcess"),
                "Error for '{}' must be ProtectedSystemProcess",
                name
            );
        }
    }

    // ==========================================
    // 5. Unprotected Application Approval
    // ==========================================

    #[test]
    fn test_terminate_process_allows_unprotected_application_attempt() {
        // Attempting to terminate an unprotected dummy PID should pass guard validation
        // and return a TerminateResult (failure because PID does not exist, but NOT blocked as ProtectedSystemProcess)
        let non_existent_pid = 999_999;
        let res = terminate_process(non_existent_pid, "notepad.exe".to_string());
        assert!(res.is_ok(), "Guard should approve unprotected notepad.exe");
        let term_res = res.unwrap();
        assert_eq!(term_res.pid, non_existent_pid);
        assert_eq!(term_res.process_name, "notepad.exe");
        assert!(!term_res.success, "Non-existent process termination should report success = false");
        assert!(term_res.message.contains("not found"));
    }

    // ==========================================
    // 6. Process Listing & Categorization Tests
    // ==========================================

    #[test]
    fn test_collect_process_items_structure() {
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let guard = ProcessGuard::new();

        let items = collect_process_items(&sys, &guard);
        assert!(!items.is_empty(), "Process list must not be empty on running system");

        // Verify protected flag is present on core processes
        let has_protected = items.iter().any(|p| p.is_protected);
        assert!(has_protected, "At least one system process should be marked is_protected");

        // Verify categories exist
        let has_system_or_service = items.iter().any(|p| {
            p.category == ProcessCategory::System || p.category == ProcessCategory::BackgroundService
        });
        assert!(has_system_or_service, "Process list should include System or Service categories");

        // Verify descending sort order by CPU percentage
        for i in 1..items.len() {
            assert!(
                items[i - 1].cpu_percent >= items[i].cpu_percent
                    || (items[i - 1].cpu_percent == items[i].cpu_percent
                        && items[i - 1].memory_bytes >= items[i].memory_bytes),
                "Process items must be sorted descending by CPU percent then memory"
            );
        }
    }
}
```

---

### File 2: `apps/desktop/src-tauri/src/monitoring/collector.rs` (DIFF & REPLACEMENTS)

Add imports and methods to `SystemCollector`:

```rust
// In apps/desktop/src-tauri/src/monitoring/collector.rs:
// Add to imports:
use pc_doctor_core::models::{
    CpuCoreInfo, CpuSnapshot, DiskSnapshot, MemorySnapshot, OsSnapshot,
    ProcessInfo, ProcessItem, SystemSnapshot, TemperatureSensor,
};
use pc_doctor_core::processes::{categorize_process, ProcessGuard};

// Inside `impl SystemCollector`:
    pub fn get_process_list(&self) -> Vec<ProcessItem> {
        let guard = ProcessGuard::new();
        let mut list: Vec<ProcessItem> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, proc)| {
                let pid_u32 = pid.as_u32();
                let name = proc.name().to_string_lossy().to_string();
                let exe_path = proc.exe().and_then(|p| p.to_str());
                let category = categorize_process(pid_u32, &name, exe_path);
                let is_protected = guard.is_protected(pid_u32, &name);

                ProcessItem {
                    pid: pid_u32,
                    name,
                    cpu_percent: proc.cpu_usage(),
                    memory_bytes: proc.memory(),
                    status: format!("{:?}", proc.status()),
                    category,
                    is_protected,
                }
            })
            .collect();

        // Sort descending by CPU percentage, fallback to memory
        list.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
        });

        list
    }

    pub fn refresh_processes(&mut self) {
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    }
```

---

### File 3: `apps/desktop/src-tauri/src/commands/mod.rs` (DIFF & REPLACEMENTS)

Replace the old `get_process_list` stub in `apps/desktop/src-tauri/src/commands/mod.rs` and export `process`:

```rust
// Replace lines 1-25 of apps/desktop/src-tauri/src/commands/mod.rs with:
pub mod process;

pub use process::{get_process_list, terminate_process, terminate_process_with_guard};

use crate::monitoring::CollectorState;
use pc_doctor_core::knowledge::KnowledgeBase;
use pc_doctor_core::models::SystemSnapshot;
use tauri::State;
use std::sync::Arc;

#[tauri::command]
pub fn get_system_snapshot(state: State<'_, Arc<CollectorState>>) -> Result<SystemSnapshot, String> {
    let mut collector = state
        .collector
        .lock()
        .map_err(|e| format!("Failed to acquire collector lock: {}", e))?;
    Ok(collector.take_snapshot())
}
```

---

### File 4: `apps/desktop/src-tauri/src/lib.rs` (DIFF & REPLACEMENTS)

Update imports and `invoke_handler`:

```rust
// In apps/desktop/src-tauri/src/lib.rs line 5:
use commands::{get_process_list, get_system_snapshot, query_assistant, terminate_process};

// In apps/desktop/src-tauri/src/lib.rs lines 27-31:
        .invoke_handler(tauri::generate_handler![
            get_system_snapshot,
            get_process_list,
            terminate_process,
            query_assistant
        ])
```

---

## 5. Verification Method

To independently verify the implementation after code integration:

1. **Workspace Compilation & Type Check**:
   ```powershell
   cargo check --workspace
   ```
   *Expected Result*: Exits with code 0 and zero compilation or type errors.

2. **Run Desktop Crate Tests**:
   ```powershell
   cargo test -p desktop
   ```
   *Expected Result*: All 12+ unit tests in `src/commands/process.rs` pass:
   - `test_terminate_process_rejects_csrss` ... ok
   - `test_terminate_process_rejects_lsass` ... ok
   - `test_terminate_process_rejects_services` ... ok
   - `test_terminate_process_rejects_explorer` ... ok
   - `test_terminate_process_rejects_smss` ... ok
   - `test_terminate_process_rejects_pid_four_system` ... ok
   - `test_terminate_process_rejects_pid_four_with_arbitrary_name` ... ok
   - `test_terminate_process_rejects_pid_zero_idle` ... ok
   - `test_terminate_process_rejects_self_pid` ... ok
   - `test_terminate_process_rejects_self_pid_with_mock_guard` ... ok
   - `test_terminate_process_rejects_casing_variations` ... ok
   - `test_terminate_process_allows_unprotected_application_attempt` ... ok
   - `test_collect_process_items_structure` ... ok

3. **Run Full Workspace Tests**:
   ```powershell
   cargo test --workspace
   ```
   *Expected Result*: Zero failures across both `pc-doctor-core` and `desktop`.

4. **Invalidation Conditions**:
   - Calling `terminate_process(pid, name)` for any of `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `PID 4`, or `self PID` returns `Ok(...)` instead of `Err(...)`.
   - Error string returned from a blocked termination fails to deserialize into `ProcessTerminationError::ProtectedSystemProcess`.
   - `get_process_list` returns items missing `category` or `is_protected` fields.
   - Any unhandled panic or OS-level crash during process listing or termination checks.
