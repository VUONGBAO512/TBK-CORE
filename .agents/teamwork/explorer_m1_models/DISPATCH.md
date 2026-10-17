## 2026-10-16T07:05:05Z
You are an Explorer for Milestone 1 Models, Diagnostics & Knowledge Base in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_models.
You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md

Task:
Analyze and design the exact implementation for:
1. Workspace `Cargo.toml`: Moving `[profile.release]` from `apps/desktop/src-tauri/Cargo.toml` to root `Cargo.toml` to eliminate the workspace profile warning.
2. `crates/pc-doctor-core/src/models.rs`: Adding CleanableTarget, StorageScanReport, CleanRequest, StorageCleanSummary, ProcessCategory, ProcessItem, TerminateResult, and aligning with telemetry & diagnostic types.
3. `crates/pc-doctor-core/src/knowledge/mod.rs`: Expanding bilingual knowledge base with articles on storage cleanup safety and process management safety in Vietnamese and English.
4. `crates/pc-doctor-core/src/lib.rs`: Exposing modules cleanly and setting up unit test runners.
Write your complete analysis and implementation blueprint to d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_models/handoff.md. Update progress.md with your liveness.
Send a message to your caller upon completion with the path to your handoff report.
