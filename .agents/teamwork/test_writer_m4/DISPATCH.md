## 2026-10-16T08:40:49Z
You are the E2E Test Writer for Milestone 4 (Final Milestone) in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/test_writer_m4.

You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md
Path to test architecture: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_e2e_infra/TEST_INFRA.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

File Ownership:
You have exclusive write ownership of:
- d:/HOCC++/pc-doctor/crates/pc-doctor-core/tests/e2e_suite.rs (create new)

Task:
Implement the 4-Tier E2E Test Suite in `crates/pc-doctor-core/tests/e2e_suite.rs` per the specifications in `TEST_INFRA.md`:
1. Tier 1: Feature Coverage (Category-Partition tests across telemetry, diagnostic rules with 5 severity tiers, storage scanner, exact byte counting, dry-run mode, locked file skip, protected boundary guard, process listing, categorization, runaway detection, process safety blacklist, bilingual knowledge base, assistant engine).
2. Tier 2: Boundary Value Analysis & Corner Cases (empty targets, deep hierarchies, read-only permissions, locked file sharing violations OS error 32, path traversal with `..` and Windows forbidden paths, PID 0, PID 4, PID <= 4, self PID, casing invariance, boundary CPU 89.9% vs 90%, boundary RAM 2GB, dual pressure, unicode diacritics, bilingual fallback).
3. Tier 3: Cross-Feature Combinatorial Interactions (storage clean + telemetry, process list + runaway + termination guard, diagnostic rules + runaway pressure, knowledge base search + diagnostic findings).
4. Tier 4: Real-World Operational Scenarios (multi-step operational user workflows: full PC temp cache scanning and purging with locked file skips, detecting runaway processes and terminating rogue app while strictly protecting Windows system processes, diagnostic crisis evaluation and remediation).

Verification:
- Run `cargo check --workspace`
- Run `cargo test --workspace`
- Ensure all tests pass with 0 failures, 0 ignored, and 0 warnings.
- Write your complete handoff report to `d:/HOCC++/pc-doctor/.agents/teamwork/test_writer_m4/handoff.md`.
- Send a message to your caller when done.
