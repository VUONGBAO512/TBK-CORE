# BRIEFING — 2026-10-16T07:04:00Z

## Mission
Extract and formalize precise specifications, constraints, and contracts for PC Doctor R1-R4 (Telemetry, Temp/Cache Scanner, Process Management, Offline Assistant).

## 🔒 My Identity
- Archetype: specification_miner
- Roles: Requirements and Windows Spec Miner, Teamwork specialist
- Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements
- Original parent: d0833960-9ed1-454f-b7d6-be733f742605
- Milestone: Requirements & Windows Spec Formalization (M1 Spec Mining)

## 🔒 Key Constraints
- Read-only: Do NOT implement anything. Discover, probe, and formalize specifications.
- Must cover all 4 core requirements: R1, R2, R3, R4.
- Exact schemas, deterministic rules, safety blacklists, safe paths, bilingual templates.
- Work solely within assigned directory d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements.
- Output handoff report to handoff.md and notify parent d0833960-9ed1-454f-b7d6-be733f742605 via send_message.

## Current Parent
- Conversation ID: d0833960-9ed1-454f-b7d6-be733f742605
- Updated: 2026-10-16T07:04:00Z

## Task Summary
- **What to build**: Exhaustive specification and contracts for R1 (Telemetry & Rules Engine), R2 (Storage & Temp Scanner), R3 (Process Inspection & Safety Blacklist), R4 (Bilingual Knowledge Base & Offline Assistant).
- **Success criteria**: Complete typed contracts (Rust & TypeScript), error behaviors, threshold matrices, Windows path catalogs, safety guarantees, and edge case specifications.
- **Interface contracts**: Shared types in packages/shared-types, Tauri commands in src-tauri, Core engine in pc-doctor-core.
- **Code layout**: Rust workspace (crates/pc-doctor-core, apps/desktop/src-tauri) + TypeScript frontend (apps/desktop, packages/shared-types).

## Key Decisions Made
- Dynamic environment variable resolution for temporary paths (%TEMP%, %LOCALAPPDATA%, %WINDIR%) rather than hardcoded username paths to support international characters and Unicode user accounts.
- Locked/open file skip semantics returning structured `skipped_errors` and non-negative `freed_bytes` rather than panicking on `ERROR_SHARING_VIOLATION`.
- Protected system process blacklist (`csrss.exe`, `lsass.exe`, `services.exe`, etc.) and PID <= 4 guard returning structured safety error `ProtectedSystemProcess`.
- Two-tier offline assistant architecture: live telemetry evaluation for system health/status queries, and offline keyword search for diagnostic articles.

## Artifact Index
- d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md — Source requirements
- d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements/DISPATCH.md — Dispatch log
- d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements/progress.md — Liveness heartbeat & progress
- d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements/handoff.md — Exhaustive specification and contract formalization
