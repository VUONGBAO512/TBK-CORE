## 2026-10-17T12:43:28Z
You are the E2E Remediation Worker for Milestone 4 in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/worker_e2e_remediation.

You MUST read the original request and the victory audit report before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md
Path to test architecture: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_e2e_infra/TEST_INFRA.md
Path to victory audit report: d:/HOCC++/pc-doctor/.agents/teamwork/victory_auditor_1/handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

File Ownership:
You have exclusive write ownership of:
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/tests/e2e_suite.rs
- d:/HOCC++/pc-doctor/apps/desktop/src-tauri/tests/adversarial_storage_ipc_tests.rs (if needed to fix warnings)

Task Requirements:
1. Fix all 7 compiler warnings in `crates/pc-doctor-core/tests/e2e_suite.rs`:
   - Fix line 623 useless comparison on `u64` (`report.total_reclaimable_bytes >= 0`).
   - Remove unused imports (`DiagnosticHistory`, `KnowledgeArticle`, `RunawayThresholds`, `WINDOWS_PROTECTED_PROCESSES`, `detect_runaway_with_thresholds`, `is_runaway_with_thresholds`, `normalize_proc_name`, `StorageCleanSummary`).
   - Fix unused variables (`_engine`, `_initial_disk`).
   - Ensure 0 compiler warnings remain in `e2e_suite.rs` or `adversarial_storage_ipc_tests.rs`.

2. Implement the missing 30 tests in `crates/pc-doctor-core/tests/e2e_suite.rs`:
   - 15 Tier 1 Feature Coverage tests:
     - F14: Storage Cleaner UI Contracts (T1-F14-01 through T1-F14-05)
     - F15: Process Manager UI Contracts (T1-F15-01 through T1-F15-05)
     - F16: React Bilingual Localization Contracts (T1-F16-01 through T1-F16-05)
   - 15 Tier 2 Boundary Value Analysis tests:
     - F14: Storage Cleaner BVA & Corner Cases (T2-F14-01 through T2-F14-05)
     - F15: Process Manager BVA & Corner Cases (T2-F15-01 through T2-F15-05)
     - F16: Bilingual Localization BVA & Corner Cases (T2-F16-01 through T2-F16-05)

3. Register all 30 tests into `build_master_registry()`:
   - Tier 1: 85 tests (17 features x 5)
   - Tier 2: 85 tests (17 features x 5)
   - Tier 3: 15 tests
   - Tier 4: 12 tests
   - Total: 197 tests!

4. Independent Verification:
   - Run `cargo check --workspace` — ensure 0 errors and 0 warnings.
   - Run `cargo test --test e2e_suite -- --nocapture` — verify `Summary: Total: 197, Passed: 197, Failed: 0`.
   - Run `cargo test --workspace` — ensure all workspace tests pass with 0 errors and 0 warnings.
   - Run `npm run build` in `apps/desktop` — ensure 0 errors.

5. Write handoff report to `d:/HOCC++/pc-doctor/.agents/teamwork/worker_e2e_remediation/handoff.md`.
6. Send a message to your caller when done.
