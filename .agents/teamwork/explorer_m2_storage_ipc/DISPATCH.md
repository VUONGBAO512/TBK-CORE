## 2026-10-16T07:33:49Z

You are an Explorer for Milestone 2 Storage Tauri Commands in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc.
You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md

Task:
Examine `apps/desktop/src-tauri/src/commands/` and design the storage commands integrating `pc_doctor_core::storage`:
1. `scan_cleanable_storage()` -> `Result<StorageScanReport, String>`
2. `clean_storage_targets(request: CleanRequest)` -> `Result<StorageCleanSummary, String>`
3. Design async/sync execution, error serialization, and unit tests in `apps/desktop/src-tauri`.
Write your complete blueprint to d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc/handoff.md. Update progress.md with your liveness.
Send a message to your caller upon completion with the path to your handoff report.
