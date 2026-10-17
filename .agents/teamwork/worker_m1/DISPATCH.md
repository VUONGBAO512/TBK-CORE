# DISPATCH — worker_m1
Milestone 1 Implementation: Core Domain Logic & Safety Engines in crates/pc-doctor-core
Original Request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Project Plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md
Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/worker_m1
Explorer Blueprints:
- Storage Engine: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_storage/handoff.md
- Process Guard & Blacklist: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_process/handoff.md
- Models & Knowledge: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_models/handoff.md

## 2026-10-16T07:14:01Z
You are the Implementation Worker for Milestone 1: Core Domain Logic & Safety Engines in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/worker_m1.
You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md

Read the three Explorer handoff reports which contain the complete, tested blueprints:
- Storage Engine: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_storage/handoff.md
- Process Guard & Blacklist: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_process/handoff.md
- Models & Knowledge: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_models/handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

File Ownership:
You have exclusive write ownership of:
- d:/HOCC++/pc-doctor/Cargo.toml (move [profile.release] here from apps/desktop/src-tauri/Cargo.toml to fix Cargo workspace warning)
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/Cargo.toml (remove [profile.release])
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/Cargo.toml
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/models.rs
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/storage/ (mod.rs, models.rs, safety.rs, targets.rs, scanner.rs, purge.rs)
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/processes/ (mod.rs, category.rs, runaway.rs, guard.rs, error.rs)
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/knowledge/mod.rs
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/src/lib.rs

Implementation Requirements:
1. Fix root & tauri Cargo.toml profile warning.
2. Implement `crates/pc-doctor-core/src/models.rs` with all shared domain types.
3. Implement `crates/pc-doctor-core/src/storage/`:
   - Full cleanable targets (%TEMP%, CrashDumps, WER, Explorer thumbnails, logs).
   - Strict safety boundary guard with safe root whitelist, forbidden blacklist, canonicalization, and component-aware prefix matching.
   - Recursive scanner with symlink escape prevention and non-negative byte calculations.
   - Purge engine with dry-run support, locked/in-use file skip handling without panics (handling Windows sharing violations OS error 32 and access denied error 5), and root directory preservation.
4. Implement `crates/pc-doctor-core/src/processes/`:
   - Categorization into System, BackgroundService, Application, BrowserEngine.
   - Runaway detection thresholds (CPU >= 90%, RAM > 2GB, or dual pressure).
   - Hardcoded Windows OS safety blacklist: csrss.exe, lsass.exe, services.exe, explorer.exe, smss.exe, wininit.exe, winlogon.exe, svchost.exe, dwm.exe, PID <= 4, and self PID.
   - Structured error `ProcessTerminationError::ProtectedSystemProcess`.
5. Implement `crates/pc-doctor-core/src/knowledge/mod.rs` with bilingual articles for storage cleanup safety and process management safety in vi and en.
6. Implement comprehensive unit tests across storage, processes, models, and knowledge in `crates/pc-doctor-core`.
7. Run `cargo check --workspace` and `cargo test --workspace`. Ensure 0 errors, 0 warnings, and all unit tests pass.

Write your complete handoff report to d:/HOCC++/pc-doctor/.agents/teamwork/worker_m1/handoff.md including build/test command results and files changed. Send a message to your caller when done.

