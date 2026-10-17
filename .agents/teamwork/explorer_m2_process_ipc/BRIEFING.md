# BRIEFING — 2026-10-16T07:40:00Z

## Mission
Investigate and design Tauri IPC commands for process inspection and safe termination in PC Doctor (`get_process_list`, `terminate_process`, `ProcessGuard` validation, and unit tests).

## 🔒 My Identity
- Archetype: explorer
- Roles: investigation, synthesis
- Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_process_ipc
- Original parent: d0833960-9ed1-454f-b7d6-be733f742605
- Milestone: Milestone 2 Process Tauri Commands

## 🔒 Key Constraints
- Read-only investigation — do NOT implement in source code
- Produce self-contained blueprint in handoff.md
- Use ProcessGuard::new().validate_termination(pid, &process_name)
- Handle csrss.exe, lsass.exe, services.exe, explorer.exe, smss.exe, PID 4, and self PID protection
- Deliver exact code snippets / proposed designs for the implementer

## Current Parent
- Conversation ID: d0833960-9ed1-454f-b7d6-be733f742605
- Updated: 2026-10-16T07:40:00Z

## Investigation State
- **Explored paths**:
  - `crates/pc-doctor-core/src/processes/` (`mod.rs`, `guard.rs`, `error.rs`, `category.rs`, `runaway.rs`)
  - `crates/pc-doctor-core/src/models.rs` (`ProcessCategory`, `ProcessItem`, `TerminateResult`)
  - `apps/desktop/src-tauri/Cargo.toml` (`edition = 2024`, `sysinfo = 0.33`, `pc-doctor-core` dep)
  - `apps/desktop/src-tauri/src/commands/mod.rs` (current `get_process_list` stub returning `Vec<ProcessInfo>`)
  - `apps/desktop/src-tauri/src/monitoring/collector.rs` (`SystemCollector`, `sysinfo` process iteration)
  - `apps/desktop/src-tauri/src/lib.rs` (Tauri invoke handlers registration)
  - `apps/desktop/src/types.ts` & `apps/desktop/src/components/processes/ProcessPanel.tsx` (frontend expectations)
- **Key findings**:
  - `pc-doctor-core` models and guard are fully implemented and tested.
  - `sysinfo 0.33` provides `sysinfo::Pid::from_u32(pid)` and `proc.kill() -> bool`.
  - `terminate_process(pid: u32, process_name: String)` does not require Tauri `State`, allowing direct invocation in unit tests.
  - Both `terminate_process` and `terminate_process_with_guard` allow testing against real OS self PID and mock self PID.
  - Full defense-in-depth: checks passed `process_name` before touching OS, then checks actual binary name if found in `sysinfo`.
  - Rejection produces structured JSON serialized from `ProcessTerminationError::ProtectedSystemProcess`.
- **Unexplored areas**: None. Design is 100% complete and verified against crate interfaces.

## Key Decisions Made
- Architecture: Create `apps/desktop/src-tauri/src/commands/process.rs` containing `collect_process_items`, `terminate_process_with_guard`, Tauri command wrappers, and unit tests.
- Re-export `process` module in `apps/desktop/src-tauri/src/commands/mod.rs`.
- Register `terminate_process` in `apps/desktop/src-tauri/src/lib.rs`.
- Update `SystemCollector` in `monitoring/collector.rs` to add `get_process_list(&self) -> Vec<ProcessItem>` and `refresh_processes(&mut self)`.

## Artifact Index
- DISPATCH.md — Initial task message
- progress.md — Liveness and step tracking
- BRIEFING.md — Persistent context memory
- handoff.md — Complete 5-component blueprint report
