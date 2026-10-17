# Adversarial Challenge & Handoff Report — Milestone 2 (Process IPC)

**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/challenger_m2_2`  
**Author**: Challenger 2 (Empirical Adversarial Reviewer)  
**Parent / Caller**: `d0833960-9ed1-454f-b7d6-be733f742605`  
**Date**: 2026-10-16  
**Verdict**: **APPROVE**  
**Overall Risk Assessment**: **LOW**  

---

## 1. Observation

### 1.1 Implementation Architecture
In `apps/desktop/src-tauri/src/commands/process.rs`:
- Line 16: `collect_process_items(sys: &System, guard: &ProcessGuard) -> Vec<ProcessItem>` gathers running processes, assigns categories via `pc_doctor_core::processes::categorize_process`, attaches `is_protected = guard.is_protected(pid_u32, &name)`, and sorts descending by `cpu_percent`, then `memory_bytes`.
- Line 53: `terminate_process_with_guard(guard: &ProcessGuard, pid: u32, process_name: &str) -> Result<TerminateResult, String>` implements a multi-stage defense-in-depth:
  - **Stage 1 (Pre-execution validation)** (lines 58-63): Validates `(pid, process_name)` against `guard.validate_termination(pid, process_name)`. If blocked, serializes and returns `ProcessTerminationError::ProtectedSystemProcess` immediately without querying or touching the OS process.
  - **Stage 2 (OS Process Resolution & Verification)** (lines 65-79): Queries OS via `sysinfo::System::process(Pid::from_u32(pid))`. If found, extracts actual OS binary image name `real_name = proc.name().to_string_lossy().to_string()` and re-evaluates `guard.validate_termination(pid, &real_name)`. If blocked, immediately returns `ProcessTerminationError::ProtectedSystemProcess`.
  - **Stage 3 (Execution)** (lines 86-100): Calls `proc.kill()` only after both Stage 1 and Stage 2 have passed.

In `crates/pc-doctor-core/src/processes/guard.rs`:
- Lines 123-141: `pid <= 4` boundary check strictly blocks PID 0 (System Idle Process), PID 4 (System Kernel), and PIDs 1..=3.
- Lines 144-150: `pid == self.self_pid` strictly blocks self-termination of the desktop host process.
- Lines 153-162: Case-insensitive normalized matching against `WINDOWS_PROTECTED_PROCESSES`.

In `crates/pc-doctor-core/src/processes/category.rs`:
- Lines 64-71: `normalize_proc_name(name: &str) -> String` performs `lower = name.trim().to_lowercase()` followed by `.strip_suffix(".exe")`.

### 1.2 Verification Commands Executed
1. `cargo test -p desktop --lib`
   - Output: `running 26 tests ... test result: ok. 26 passed; 0 failed; 0 ignored; finished in 0.57s`
2. `cargo test -p desktop --test adversarial_process_ipc`
   - Output: `running 10 tests ... test result: ok. 10 passed; 0 failed; 0 ignored; finished in 0.31s`
3. `cargo test -p pc-doctor-core`
   - Output: `running 27 tests (lib) + 16 tests (adversarial_process) + 20 tests (adversarial_storage) ... test result: ok. 63 passed; 0 failed; finished in 0.26s`

---

## 2. Logic Chain

1. **Mandatory Protected Processes (All 9 Targets)**:
   - *Observation*: `test_adversarial_all_nine_mandatory_protected_processes` invoked `terminate_process` against `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `wininit.exe`, `winlogon.exe`, `svchost.exe`, and `dwm.exe`.
   - *Logic*: All 9 targets are listed in `WINDOWS_PROTECTED_PROCESSES` with explicit crash/safety explanations. In every case, Stage 1 returned `Err(serde_json::to_string(&ProcessTerminationError::ProtectedSystemProcess { .. }))`. Deserialization confirmed the error payload contains exact PID, target name, and non-empty rationale. No OS kill command was reached.

2. **Casing & Extension Invariance**:
   - *Observation*: `test_adversarial_casing_and_extension_tricks` tested 32 variations including `CSRSS.EXE`, `csrss`, `Explorer.exe`, `EXPLORER`, `LSASS`, `SvChOsT.eXe`, and whitespace-padded variants like `  csrss.exe  ` and `\texplorer.exe\n`.
   - *Logic*: `normalize_proc_name` trims surrounding whitespace, lowercases all characters, and strips any `.exe` suffix before matching against normalized table tokens. All 32 variations returned `ProtectedSystemProcess`.

3. **PID Boundaries (PID 0, PID 4, PID <= 4, Self PID)**:
   - *Observation*: `test_adversarial_pid_boundaries_zero_to_four` tested PIDs 0, 1, 2, 3, 4 with benign names (`"notepad.exe"`), empty names (`""`), and system names. `test_adversarial_self_pid_protection` tested current OS PID (`std::process::id()`) and mock PID (98765).
   - *Logic*: The guard checks `pid <= 4` and `pid == self.self_pid` prior to examining process names. Even when an attacker passes a benign name, the numerical check immediately halts execution and returns `ProtectedSystemProcess`. PID 5 (right above the boundary) with a benign name passes the guard and fails gracefully at the OS level ("Process with PID 5 was not found"), proving there is no off-by-one boundary flaw.

4. **Spoofing Defense-in-Depth Against Live Host Processes**:
   - *Observation*: `test_adversarial_spoofing_live_host_protected_process` located a running protected Windows service on the host (PID 4600, `svchost.exe`) and invoked `terminate_process(4600, "notepad.exe".to_string())`.
   - *Logic*:
     - Stage 1: `(4600, "notepad.exe")` passed initial validation because PID 4600 > 4 and `"notepad.exe"` is benign.
     - Stage 2: `terminate_process` queried the live process via `sysinfo::System::process(Pid::from_u32(4600))`. The OS returned `proc.name() = "svchost.exe"`. Stage 2 evaluated `guard.validate_termination(4600, "svchost.exe")`, detected that `svchost.exe` is protected, and aborted with `ProtectedSystemProcess`.
     - Live verification confirmed PID 4600 remained running and unkilled.
     - This empirically verifies that defense-in-depth functions against live operating system processes.

5. **Sorting & Listing Invariants**:
   - *Observation*: `test_adversarial_process_listing_invariants` sampled running host processes via `collect_process_items`.
   - *Logic*: Every returned process item with PID <= 4 or matching protected names possessed `is_protected == true` and `category == ProcessCategory::System`. The items conformed strictly to descending order by CPU usage percentage, fallback to memory consumption.

---

## 3. Adversarial Challenge Analysis & Stress Test Results

### Challenge Summary
**Overall Risk Assessment**: **LOW**  
The Process IPC implementation has robust two-layer defense-in-depth protection. Critical system processes, kernel PIDs, and self-processes cannot be killed through the IPC boundary under any tested scenario.

### Challenges Evaluated

#### Challenge 1: Spoofing Name to Disguise Kernel or Self PID (Risk: LOW, Status: BLOCKED)
- **Assumption Challenged**: Can a caller bypass protection by passing PID 4 or self PID with a benign name like `"notepad.exe"`?
- **Attack Scenario**: Calling `terminate_process(4, "notepad.exe")` or `terminate_process(self_pid, "calc.exe")`.
- **Result**: BLOCKED at Stage 1. Guard checks `pid <= 4` and `pid == self.self_pid` before process name checks.

#### Challenge 2: Spoofing Name to Disguise Live Running System Daemon (Risk: LOW, Status: BLOCKED)
- **Assumption Challenged**: Can a caller kill a running Windows service (e.g. `explorer.exe`, `svchost.exe`, `dwm.exe`) by passing its real PID with a spoofed name?
- **Attack Scenario**: Calling `terminate_process(real_svchost_pid, "notepad.exe")`.
- **Result**: BLOCKED at Stage 2. The handler inspects the actual binary name returned by the OS kernel and halts before calling `kill()`. Verified empirically on live host process PID 4600.

#### Challenge 3: Casing, Extension, and Formatting Evasion (Risk: LOW, Status: BLOCKED)
- **Assumption Challenged**: Can variation in capitalization, whitespace, or extension omission (`CSRSS.EXE`, `csrss`, `Explorer.exe`, `  lsass  `) evade blacklist string comparisons?
- **Attack Scenario**: 32 variations tested across all 9 protected processes.
- **Result**: BLOCKED. `normalize_proc_name` normalizes all tokens deterministically.

#### Challenge 4: Passing Full Binary Path (Risk: LOW / INFORMATIONAL, Status: MITIGATED BY STAGE 2)
- **Assumption Challenged**: What happens if a caller passes `"C:\\Windows\\System32\\csrss.exe"` or `"C:\\Windows\\explorer.exe"`?
- **Attack Scenario**: Calling `terminate_process(pid, "C:\\Windows\\explorer.exe")`.
- **Observation**: `normalize_proc_name` strips `.exe` and lowercases, but does not strip directory paths (`"c:\\windows\\explorer"` != `"explorer"`). Consequently, Stage 1 does not block the request solely based on the path string.
- **Mitigation / Defense**: If the process is alive with that PID, Stage 2 queries `proc.name()`, which `sysinfo` reports as `"explorer.exe"` (without path), so Stage 2 catches and blocks it with `ProtectedSystemProcess`. If the process does not exist, it fails safely with `"Process with PID x was not found."` No protected process can be killed.
- **Recommendation**: For Milestone 4 hardening, `normalize_proc_name` or `ProcessGuard::validate_termination` could also strip path prefixes (`std::path::Path::new(name).file_name()`) so Stage 1 rejects path-containing strings even before the OS lookup.

### Stress Test Matrix

| # | Scenario / Attack Vector | Expected Behavior | Actual Behavior | Result |
|---|---------------------------|-------------------|-----------------|--------|
| 1 | Terminate `csrss.exe` (PID 20000) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 2 | Terminate `lsass.exe` (PID 20001) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 3 | Terminate `services.exe` (PID 20002) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 4 | Terminate `explorer.exe` (PID 20003) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 5 | Terminate `smss.exe` (PID 20004) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 6 | Terminate `wininit.exe` (PID 20005) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 7 | Terminate `winlogon.exe` (PID 20006) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 8 | Terminate `svchost.exe` (PID 20007) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 9 | Terminate `dwm.exe` (PID 20008) | Rejected with `ProtectedSystemProcess` | Rejected with `ProtectedSystemProcess` | **PASS** |
| 10 | 32 Casing/Whitespace variations | All rejected with `ProtectedSystemProcess` | All 32 rejected with `ProtectedSystemProcess` | **PASS** |
| 11 | PID 0 (System Idle Process) | Rejected as NT kernel process | Rejected as NT kernel process | **PASS** |
| 12 | PID 4 (System Kernel) | Rejected as NT kernel process | Rejected as NT kernel process | **PASS** |
| 13 | PID 1..=3 | Rejected as NT kernel process | Rejected as NT kernel process | **PASS** |
| 14 | Self PID (`std::process::id()`) | Rejected with own application reason | Rejected with own application reason | **PASS** |
| 15 | Mock Self PID (98765) | Rejected with own application reason | Rejected with own application reason | **PASS** |
| 16 | PID 4 spoofed with `"notepad.exe"` | Rejected by PID <= 4 guard | Rejected by PID <= 4 guard | **PASS** |
| 17 | Live host `svchost.exe` (PID 4600) spoofed as `"notepad.exe"` | Stage 2 intercepts and blocks; process kept alive | Stage 2 intercepted, returned `ProtectedSystemProcess`, process verified alive | **PASS** |
| 18 | Unprotected dummy process (`notepad.exe`, PID 987654) | Guard approves, fails gracefully at OS | Guard approved, returned `success: false, "not found"` | **PASS** |
| 19 | Process list sorting invariants | Descending CPU %, fallback to memory bytes | Strict descending order verified | **PASS** |

---

## 4. Caveats

1. **Concurrent Peer Test File in Workspace**:
   - `cargo test --workspace` encountered a compilation type mismatch in `apps/desktop/src-tauri/tests/adversarial_storage_ipc_tests.rs` (owned by concurrent agent `challenger_m2_1`). Per isolation and review-only rules, this file was not modified.
   - All tests targeting the Process IPC domain (`cargo test -p desktop --lib` and `cargo test -p desktop --test adversarial_process_ipc`) compile cleanly with zero warnings and pass 100%.
2. **Standard User Privileges**:
   - On Windows, terminating certain system processes requires `SeDebugPrivilege` or Administrator rights. Even in elevated mode, the software guardrail in `desktop_lib` halts termination before reaching the OS API.

---

## 5. Conclusion

**Verdict**: **APPROVE**

The Process IPC commands in `apps/desktop/src-tauri/src/commands/process.rs` fulfill all requirements in `ORIGINAL_REQUEST.md` (R3) and `PROJECT.md`. The termination guard successfully withstands:
1. All 9 mandatory protected processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `wininit.exe`, `winlogon.exe`, `svchost.exe`, `dwm.exe`).
2. Casing and extension evasion (`CSRSS.EXE`, `csrss`, `Explorer.exe`, `EXPLORER`, etc.).
3. PID boundary attacks (PID 0, PID 4, PID <= 4, self PID, and transition above boundary).
4. Spoofing attacks (disguising PID 4 as `"notepad.exe"` and disguising live Windows host processes as benign applications).
5. Two-stage defense-in-depth ensures that even if a caller attempts process name spoofing, the live OS binary name is resolved and blocked before any termination call is made.

---

## 6. Verification Method

To independently reproduce the empirical findings:

1. **Run Desktop Unit Tests**:
   ```powershell
   cargo test -p desktop --lib
   ```
   *Expected Output*: `26 passed; 0 failed; 0 ignored`

2. **Run Desktop Adversarial Process IPC Test Suite**:
   ```powershell
   cargo test -p desktop --test adversarial_process_ipc -- --nocapture
   ```
   *Expected Output*: `10 passed; 0 failed; 0 ignored` (covers all 9 mandatory processes, casing variations, PID boundaries, live host spoofing on running services, and sorting invariants).

3. **Run PC Doctor Core Adversarial Tests**:
   ```powershell
   cargo test -p pc-doctor-core --test adversarial_process_tests
   ```
   *Expected Output*: `16 passed; 0 failed; 0 ignored`
