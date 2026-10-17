# BRIEFING — 2026-10-16T07:41:00Z

## Mission
Investigate and design Tauri commands for storage cleaning (`scan_cleanable_storage`, `clean_storage_targets`), including async/sync execution, error serialization, registration, and unit tests.

## 🔒 My Identity
- Archetype: explorer
- Roles: investigator, synthesizer
- Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc
- Original parent: d0833960-9ed1-454f-b7d6-be733f742605
- Milestone: Milestone 2 Storage Tauri Commands

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Design integration of `pc_doctor_core::storage` into `apps/desktop/src-tauri`
- Commands: `scan_cleanable_storage()` -> `Result<StorageScanReport, String>`, `clean_storage_targets(request: CleanRequest)` -> `Result<StorageCleanSummary, String>`
- Design async/sync execution, error serialization, and unit tests in `apps/desktop/src-tauri`

## Current Parent
- Conversation ID: d0833960-9ed1-454f-b7d6-be733f742605
- Updated: 2026-10-16T07:41:00Z

## Investigation State
- **Explored paths**:
  - `ORIGINAL_REQUEST.md` (lines 15-17, 30-33)
  - `PROJECT.md` (lines 53-87, 120-121)
  - `crates/pc-doctor-core/src/storage/` (`mod.rs`, `models.rs`, `safety.rs`, `scanner.rs`, `purge.rs`, `targets.rs`)
  - `apps/desktop/src-tauri/` (`Cargo.toml`, `src/lib.rs`, `src/commands/mod.rs`, `src/monitoring/collector.rs`, `src/server.rs`)
  - `apps/desktop/src/types.ts`, `packages/shared-types/src/index.ts`
- **Key findings**:
  - `pc_doctor_core::storage::StorageEngine` is pure, thread-safe, and decoupled from Tauri.
  - Tauri `async fn` commands must use `tokio::task::spawn_blocking` to avoid stalling the Tokio async runtime and the local pairing server.
  - Implemented decoupled `_impl` functions to enable direct, hermetic unit tests in `apps/desktop/src-tauri`.
- **Unexplored areas**: Milestone 3 React components (scheduled for next milestone).

## Key Decisions Made
- Designed `apps/desktop/src-tauri/src/commands/storage.rs` as a dedicated module.
- Re-exported commands via `commands/mod.rs` and registered in `src/lib.rs`.
- Created staged patch files and complete proposed code for worker implementation.

## Artifact Index
- `DISPATCH.md` — Record of task dispatch
- `BRIEFING.md` — Persistent agent memory
- `progress.md` — Heartbeat and status
- `proposed_storage.rs` — Complete Rust implementation of storage commands and 8 unit tests
- `desktop_cargo.patch` — Diff adding tempfile dev-dependency to desktop Cargo.toml
- `commands_mod.patch` — Diff exporting storage commands in commands/mod.rs
- `src_lib.patch` — Diff registering commands in tauri generate_handler
- `proposed_frontend_ipc.ts` — TypeScript types and invoke wrappers for frontend
- `handoff.md` — Final 5-component blueprint report
