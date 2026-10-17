# Review & Adversarial Challenge Report — Milestone 2: Tauri Platform Adapters & IPC Commands

**Reviewer**: Reviewer 2 (Roles: Reviewer, Adversarial Critic)  
**Parent / Caller**: `d0833960-9ed1-454f-b7d6-be733f742605`  
**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/reviewer_m2_2`  
**Date**: 2026-10-16  
**Verdict**: **REQUEST_CHANGES**

---

## Review Summary

**Verdict**: **REQUEST_CHANGES**

### Summary Rationale:
1. **Process IPC Implementation Quality: EXCELLENT / APPROVED**.  
   The process inspection and termination implementation in `apps/desktop/src-tauri/src/commands/process.rs` and `apps/desktop/src-tauri/src/monitoring/collector.rs` strictly adheres to requirements R1 and R3:
   - `get_process_list` correctly calls `collector.refresh_processes()`, maps process category (`System`, `BackgroundService`, `BrowserEngine`, `Application`), computes `is_protected`, and sorts descending by CPU percentage then memory bytes.
   - `terminate_process` implements a genuine two-layer defense-in-depth architecture: Stage 1 validates `(pid, &process_name)` through `ProcessGuard`, while Stage 2 queries the live OS process via `sysinfo`, extracts the actual OS executable binary name, and re-validates it through `ProcessGuard`.
   - Core Windows system processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, PID <= 4, and self PID) can NEVER be terminated under any circumstances, even when targeted via spoofed names, case variations, or empty strings.
   - Defense-in-depth was empirically proven against a live running host process (`svchost.exe`, PID 2264), where attempting to terminate it under the alias `"notepad.exe"` was blocked at Stage 2 and returned structured JSON `ProcessTerminationError::ProtectedSystemProcess`.
   - Unit tests in `desktop_lib` (26/26 passed) and integration tests in `tests/adversarial_process_ipc.rs` (10/10 passed) all succeed.
   - Zero integrity violations were detected (no hardcoded return mocks, no dummy facade logic, no bypasses).

2. **Blocking Finding (Why REQUEST_CHANGES is required)**:  
   While `cargo check --workspace` passes, running `cargo test -p desktop` or `cargo test --workspace` fails to compile with exit code 1 due to a type mismatch in `apps/desktop/src-tauri/tests/adversarial_storage_ipc_tests.rs:475:22`:
   ```
   error[E0308]: mismatched types
      --> apps\desktop\src-tauri\tests\adversarial_storage_ipc_tests.rs:475:22
      ...
      = note: expected struct `JoinHandle<Result<StorageScanReport, _>>`
                 found struct `JoinHandle<Result<StorageCleanSummary, _>>`
   ```
   Because Cargo automatically includes all `.rs` files in `apps/desktop/src-tauri/tests/` during `cargo test -p desktop`, this file causes the test runner command explicitly mandated by Acceptance Criteria and our review prompt to fail. The test file must be repaired (e.g. separating `handles` into `scan_handles` and `clean_handles`) to restore full workspace build and test integrity.

---

## 1. Observation

### 1.1 Process Listing & Categorization (`apps/desktop/src-tauri/src/monitoring/collector.rs` & `commands/process.rs`)

1. **`get_process_list` Implementation in `commands/process.rs`** (lines 118–128):
   ```rust
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
   ```
   - Locks the shared thread-safe `CollectorState`.
   - Calls `collector.refresh_processes()`, which runs `self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true)`.
   - Returns full `Vec<ProcessItem>`.

2. **Process Item Collection & Sorting in `monitoring/collector.rs`** (lines 171–203):
   ```rust
   pub fn get_process_list(&self) -> Vec<ProcessItem> {
       let mut list: Vec<ProcessItem> = self
           .sys
           .processes()
           .iter()
           .map(|(pid, proc)| {
               let pid_u32 = pid.as_u32();
               let name = proc.name().to_string_lossy().to_string();
               let exe_path = proc.exe().and_then(|p| p.to_str());
               let category = categorize_process(pid_u32, &name, exe_path);
               let is_protected = self.guard.is_protected(pid_u32, &name);

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
   ```
   - Maps each OS process to domain `ProcessItem`.
   - Invokes `categorize_process(pid, name, exe_path)`: PID <= 4 and system binaries mapped to `ProcessCategory::System`, browsers mapped to `ProcessCategory::BrowserEngine`, daemons/services/system32 paths mapped to `ProcessCategory::BackgroundService`, remainder mapped to `ProcessCategory::Application`.
   - Evaluates `self.guard.is_protected(pid, name)`.
   - Sorts descending by `cpu_percent`, breaking ties with `memory_bytes`.

### 1.2 Two-Layer Defense-in-Depth Termination (`apps/desktop/src-tauri/src/commands/process.rs`)

1. **`terminate_process_with_guard` Implementation** (lines 53–115):
   ```rust
   pub fn terminate_process_with_guard(
       guard: &ProcessGuard,
       pid: u32,
       process_name: &str,
   ) -> Result<TerminateResult, String> {
       // 1. Validate termination against ProcessGuard (Stage 1)
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
               // Defense-in-depth: check actual OS binary name against ProcessGuard (Stage 2)
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

               // 3. Invoke sysinfo::Process::kill() (Stage 3)
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
   ```
   - **Stage 1 (Pre-flight check)**: Checks `guard.validate_termination(pid, process_name)` before interacting with the OS.
   - **Stage 2 (Real OS identity check)**: Directly fetches the live process record from the operating system via `sys.process(sys_pid)` and tests `guard.validate_termination(pid, &real_name)`.
   - **Stage 3 (Execution)**: Calls `proc.kill()` only after both safety stages pass.
   - If blocked at either Stage 1 or Stage 2, returns serialized JSON `ProcessTerminationError::ProtectedSystemProcess` matching the contract.

### 1.3 Blacklist Enforcement & Self PID Protection

- `crates/pc-doctor-core/src/processes/guard.rs` (lines 5–86):
  Contains all mandatory system binaries: `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `wininit.exe`, `winlogon.exe`, `svchost.exe`, `dwm.exe`, `system`, `system idle process`, `registry`, `fontdrvhost.exe`, `conhost.exe`, `taskhostw.exe`, `sihost.exe`, `securityhealthservice.exe`, `msmpeng.exe`, `tbk_core.exe`, `pc-doctor.exe`.
- Lines 123–140: Unconditionally blocks all PIDs `<= 4` (`PID 0 Idle` and `PID 4 System`).
- Lines 144–150: Unconditionally blocks `pid == self.self_pid` (`std::process::id()`).

### 1.4 Test Commands Execution & Results

1. **`cargo check --workspace`**:
   - Exit code: 0
   - Output: `Finished dev profile [unoptimized + debuginfo] in 0.48s`.
2. **`cargo test -p desktop --lib`**:
   - Exit code: 0
   - Result: 26 passed; 0 failed; 0 ignored (includes 13 process tests, 9 storage tests, 4 collector tests).
3. **`cargo test -p desktop --test adversarial_process_ipc`**:
   - Exit code: 0
   - Result: 10 passed; 0 failed; 0 ignored.
   - Verbatim probe result: Live protected host process `svchost.exe` (PID 2264) targeted with spoofed name `"notepad.exe"` was blocked by Stage 2 defense-in-depth with `ProtectedSystemProcess` and remained running.
4. **`cargo test -p pc-doctor-core`**:
   - Exit code: 0
   - Result: 63 passed; 0 failed; 0 ignored (27 unit + 16 adversarial process + 20 adversarial storage).
5. **`npm run build` in `apps/desktop`**:
   - Exit code: 0
   - Result: `built in 209ms` with zero TypeScript or Vite bundle errors.
6. **`cargo test -p desktop` / `cargo test --workspace`**:
   - Exit code: 1 (FAILED)
   - Verbatim compiler error:
     ```
     error[E0308]: mismatched types
        --> apps\desktop\src-tauri\tests\adversarial_storage_ipc_tests.rs:475:22
         |
     468 |           handles.push(tokio::spawn(async {
         |  _________-------______-
         | |         |
         | |         ... which causes `handles` to have type `Vec<tokio::task::JoinHandle<Result<StorageScanReport, String>>>`
     469 | |             scan_cleanable_storage().await
     470 | |         }));
         | |__________- this argument has type `tokio::task::JoinHandle<Result<StorageScanReport, String>>`...
     ...
     475 |           handles.push(tokio::spawn(async {
         |  _________________----_^
         | |                 |
         | |                 arguments to this method are incorrect
     476 | |             let req = CleanRequest {
     477 | |                 target_ids: vec!["user_temp".to_string()],
     478 | |                 dry_run: true,
     479 | |             };
     480 | |             clean_storage_targets(req).await
     481 | |         }));
         | |__________^ expected `StorageScanReport`, found `StorageCleanSummary`
     ```

---

## 2. Logic Chain

1. **Process Listing Compliance**:
   - Observation 1.1 establishes that `get_process_list` locks the collector state, executes on-demand OS process discovery via `refresh_processes()`, maps process records with domain categorization, computes protection flags via `ProcessGuard`, and enforces descending sort order by CPU percentage and memory.
   - Therefore, Requirement R1 (top processes query) and Requirement R3 (categorized process listing) are fully satisfied.

2. **Defense-in-Depth Guarantee Against Protected Processes**:
   - Observation 1.2 demonstrates that termination occurs only if both Stage 1 (user input name against blacklist, PID <= 4, and self PID) and Stage 2 (live OS process binary name against blacklist) pass.
   - If a caller supplies a spoofed benign name (e.g. `notepad.exe`) with a protected PID (e.g. `csrss.exe` or `svchost.exe`), Stage 1 permits it, but Stage 2 queries the real binary name from the operating system, triggers `guard.validate_termination`, and aborts execution before `proc.kill()` is invoked.
   - Observation 1.4 confirms this empirically on a live Windows machine with running PID 2264 (`svchost.exe`).
   - Therefore, protected processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, PID <= 4, self PID) can NEVER be terminated.

3. **Integrity & Code Quality**:
   - Examination of `process.rs` and `collector.rs` confirms that no test assertions or results are hardcoded into production code. Real sysinfo calls and real domain evaluations are executed.
   - Deserialization and serialization of `ProcessTerminationError` adhere to `#[serde(tag = "error_type", content = "details")]`, enabling frontend UI components to inspect `error_type` and display localized safety warnings.

4. **Test Suite Integrity & Blocker Rationale**:
   - Acceptance Criteria state: "`cargo check --workspace` and `cargo test --workspace` pass with zero failures or unhandled panics."
   - Observation 1.4 shows that while `cargo test -p desktop --lib` passes 26/26 tests, running `cargo test -p desktop` fails because of a type error in `apps/desktop/src-tauri/tests/adversarial_storage_ipc_tests.rs:475:22`.
   - In accordance with the Reviewer rules ("Never trust unverified claims", "Report any failures as findings — do NOT fix them yourself"), a passing test suite is required before issuing a final APPROVE.
   - Consequently, the appropriate and honest verdict is `REQUEST_CHANGES` to fix the test compilation error.

---

## 3. Findings

### [Critical / Blocker] Finding 1: Type Mismatch in `adversarial_storage_ipc_tests.rs` Breaks `cargo test -p desktop`

- **What**: Compilation error `error[E0308]: mismatched types` in integration test file `apps/desktop/src-tauri/tests/adversarial_storage_ipc_tests.rs`.
- **Where**: `apps/desktop/src-tauri/tests/adversarial_storage_ipc_tests.rs:475:22`
- **Why**: The test instantiates a single `let mut handles = Vec::new();`. In the first loop, `tokio::spawn` produces `JoinHandle<Result<StorageScanReport, String>>`. In the second loop, `tokio::spawn` attempts to push `JoinHandle<Result<StorageCleanSummary, String>>` into the same `Vec`, which Rust's type system rejects.
- **Suggestion**: Separate into two vectors (`let mut scan_handles = Vec::new();` and `let mut clean_handles = Vec::new();`), and await them in separate loops.

### [Minor / Hardening] Finding 2: Path-Prefixed Names Rely Exclusively on Stage 2 Defense

- **What**: Input names containing path separators (e.g. `C:\Windows\System32\csrss.exe` or `C:/Windows/explorer.exe`) bypass Stage 1 validation when PID > 4.
- **Where**: `crates/pc-doctor-core/src/processes/category.rs:64–71` (`normalize_proc_name`)
- **Why**: `normalize_proc_name` converts to lowercase and strips `.exe`, but does not strip path prefixes. Thus `"c:\\windows\\system32\\csrss"` is compared to `"csrss"` and evaluates to not equal.
- **Impact Assessment**: LOW risk because Stage 2 queries the live OS process where `proc.name()` returns the unpathed binary name (`csrss.exe`), which is caught and blocked at Stage 2. However, defense-in-depth is strongest when Stage 1 also catches path-prefixed inputs.
- **Suggestion**: Update `normalize_proc_name` or `validate_termination` to extract the filename component via `std::path::Path::new(name).file_name().and_then(|f| f.to_str()).unwrap_or(name)` before normalizing.

---

## 4. Adversarial Attack Surface Assessment

| # | Attack Scenario | Target PID & Input | Stage 1 Behavior | Stage 2 Behavior | Result |
|---|-----------------|-------------------|------------------|------------------|--------|
| 1 | Direct Blacklist Termination | PID 2001, `"csrss.exe"` | Blocked (`ProtectedSystemProcess`) | Never reached | **PASS (Blocked)** |
| 2 | Casing Variation | PID 2002, `"CSRSS.EXE"` | Blocked (`normalize_proc_name`) | Never reached | **PASS (Blocked)** |
| 3 | Extension Omission | PID 2003, `"explorer"` | Blocked (`normalize_proc_name`) | Never reached | **PASS (Blocked)** |
| 4 | Whitespace Trickery | PID 2004, `"\texplorer.exe\n"` | Blocked (`trim()`) | Never reached | **PASS (Blocked)** |
| 5 | Kernel PID Boundary | PID 4, `"System"` | Blocked (`pid <= 4`) | Never reached | **PASS (Blocked)** |
| 6 | Kernel PID Spoofing | PID 4, `"notepad.exe"` | Blocked (`pid <= 4`) | Never reached | **PASS (Blocked)** |
| 7 | Idle Process | PID 0, `""` | Blocked (`pid <= 4`, auto-named) | Never reached | **PASS (Blocked)** |
| 8 | Self PID Termination | Self PID, `"TBK_Core.exe"` | Blocked (`pid == self.self_pid`) | Never reached | **PASS (Blocked)** |
| 9 | Self PID Spoofing | Self PID, `"calculator.exe"` | Blocked (`pid == self.self_pid`) | Never reached | **PASS (Blocked)** |
| 10 | Live Host Process Spoofing | PID 2264 (live `svchost.exe`), `"notepad.exe"` | Passes (benign name) | **BLOCKED (OS binary name evaluated)** | **PASS (Blocked)** |
| 11 | Non-existent Unprotected App | PID 999999, `"notepad.exe"` | Passes guard | OS process `None` -> graceful failure | **PASS (Handled)** |
| 12 | Path-Prefixed Blacklist Input | Non-existent PID, `"C:\\Windows\\System32\\csrss.exe"` | Passes Stage 1 | OS process `None` -> graceful failure | **PASS (Handled)** |

---

## 5. Verified Claims

| Claim | Verification Method | Result |
|---|---|---|
| `get_process_list` refreshes processes and categorizes | `cargo test -p desktop --lib monitoring::collector::tests` | PASS |
| `get_process_list` sorts descending by CPU then RAM | `test_collect_process_items_structure` | PASS |
| `terminate_process` Stage 1 blocks mandatory blacklist | `test_adversarial_all_nine_mandatory_protected_processes` | PASS |
| `terminate_process` Stage 2 blocks spoofed live host process | `test_adversarial_spoofing_live_host_protected_process` | PASS |
| PID <= 4 and Self PID cannot be terminated | `test_adversarial_pid_boundaries_zero_to_four` & `self_pid` | PASS |
| Workspace compilation (`cargo check --workspace`) | `cargo check --workspace` | PASS (0 warnings) |
| Frontend compilation (`npm run build`) | `npm run build` in `apps/desktop` | PASS (209ms) |
| Workspace integration tests (`cargo test -p desktop`) | `cargo test -p desktop` | **FAIL** (E0308 in storage test) |

---

## 6. Caveats

1. **Elevation / Administrator Privileges**:  
   Termination of processes running in another user session or higher integrity level (e.g. system services) will fail at the OS level with `proc.kill() == false`. The command properly reports `failed: access denied or already exiting` without panicking.
2. **CPU Measurement Delta**:  
   `sysinfo` computes CPU utilization as a delta across two sampling periods. On the first snapshot invocation, CPU usage for newly spawned processes may report 0.0%, which is expected behavior for periodic desktop telemetry polling.

---

## 7. Conclusion

The process management and safety subsystem in `apps/desktop/src-tauri` (`commands/process.rs` and `monitoring/collector.rs`) is exemplary: it fulfills all design criteria, implements ironclad defense-in-depth, and protects critical operating system processes from accidental or malicious termination.

However, because `apps/desktop/src-tauri/tests/adversarial_storage_ipc_tests.rs` contains a compilation error that causes `cargo test -p desktop` to exit with code 1, the verdict is **REQUEST_CHANGES** until that test compilation error is resolved.

---

## 8. Verification Method

To independently verify after resolving Finding 1:

1. **Verify Desktop Unit Tests**:
   ```powershell
   cargo test -p desktop --lib -- --nocapture
   ```
   *Expected*: 26 passed, 0 failed.

2. **Verify Process Adversarial Integration Tests**:
   ```powershell
   cargo test -p desktop --test adversarial_process_ipc -- --nocapture
   ```
   *Expected*: 10 passed, 0 failed.

3. **Verify Full Desktop Test Suite**:
   ```powershell
   cargo test -p desktop
   ```
   *Expected*: Clean compilation and execution of all unit and integration tests.

4. **Verify Frontend Build**:
   ```powershell
   cd apps/desktop
   npm run build
   ```
   *Expected*: Build succeeds in < 300ms.
