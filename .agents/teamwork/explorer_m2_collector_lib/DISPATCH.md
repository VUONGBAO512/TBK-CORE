# DISPATCH — explorer_m2_collector_lib
Target: Milestone 2 SystemCollector & Tauri Registration in apps/desktop/src-tauri
Original Request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Project Plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md
Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_collector_lib

## 2026-10-16T07:33:49Z
Task:
Examine `apps/desktop/src-tauri/src/lib.rs` and `apps/desktop/src-tauri/src/monitoring/collector.rs`:
1. Design Tauri builder registration for all commands: `get_system_snapshot`, `get_process_list`, `terminate_process`, `scan_cleanable_storage`, `clean_storage_targets`, `query_assistant`.
2. Update `SystemCollector` to populate `category` and `is_protected` for all queried processes.
3. Verify module exports and compilation health with `cargo check --workspace` and `cargo test --workspace`.
Write your complete blueprint to d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_collector_lib/handoff.md. Update progress.md with your liveness.
Send a message to your caller upon completion with the path to your handoff report.
