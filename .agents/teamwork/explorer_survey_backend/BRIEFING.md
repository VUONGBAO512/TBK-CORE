# BRIEFING — 2026-10-16T07:10:00Z

## Mission
Investigate the existing Rust backend codebase in PC Doctor, assess crate boundaries, dependencies, implementation status of R1-R4, build/test health, and propose a clean, deep backend architecture with safety guardrails.

## 🔒 My Identity
- Archetype: explorer
- Roles: Backend Architecture Explorer
- Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_survey_backend
- Original parent: d0833960-9ed1-454f-b7d6-be733f742605
- Milestone: Backend Architecture Survey & Proposal

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Strictly observe .agents/teamwork metadata layout rules
- Keep message concise, deliver findings via handoff.md

## Current Parent
- Conversation ID: d0833960-9ed1-454f-b7d6-be733f742605
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `Cargo.toml` (root workspace)
  - `crates/pc-doctor-core/Cargo.toml`, `src/lib.rs`, `src/models.rs`, `src/diagnostics/rules.rs`, `src/knowledge/mod.rs`
  - `apps/desktop/src-tauri/Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/server.rs`, `src/monitoring/collector.rs`, `src/commands/mod.rs`, `capabilities/default.json`, `tauri.conf.json`
  - `apps/desktop/src/hooks/useMonitoring.ts`, `apps/desktop/src/components/processes/ProcessPanel.tsx`, `apps/desktop/src/App.tsx`, `apps/desktop/src/i18n/vi.ts`
- **Key findings**:
  - R1 (Telemetry & Diagnostics): 85% implemented in `collector.rs` and `rules.rs` (8 rules, rolling history of 10 samples).
  - R2 (Storage Scanner & Cleaner): 0% implemented on backend. Entire scanning, byte calculation, dry-run, and safe deletion engine must be built.
  - R3 (Safe Process Inspection & Termination): 30% implemented (top 30 processes query only). Termination command, OS blacklist, and process categorization missing.
  - R4 (Bilingual Assistant): 75% implemented in `KnowledgeBase` and `query_assistant`.
  - Cargo profile warning identified in `apps/desktop/src-tauri/Cargo.toml` (needs to move to root `Cargo.toml`).
  - `cargo check --workspace`, `cargo test --workspace`, and `npm run build` all pass with exit code 0.
- **Unexplored areas**: None for backend scope. Full survey complete.

## Key Decisions Made
- Architecture recommendation: Place core business rules (storage scanner targets, directory whitelist containment, process blacklist, classification, and knowledge engine) in `pc-doctor-core` to maximize in-memory unit testability. Keep platform adapters (sysinfo collection, file deletion execution, process termination syscalls, Tauri IPC) in `desktop_lib`.
- Safety design: Path canonicalization whitelist check prevents arbitrary deletions outside safe temp folders; locked file error catching (`io::ErrorKind::PermissionDenied` / `ERROR_SHARING_VIOLATION`) safely increments skipped counter without crashing; process blacklist immediately rejects termination of vital Windows system processes before OS calls.

## Artifact Index
- handoff.md — Comprehensive backend architectural survey, diagnostic analysis, and recommendation report
- progress.md — Liveness heartbeat and milestone tracking
- DISPATCH.md — Record of dispatch instructions
