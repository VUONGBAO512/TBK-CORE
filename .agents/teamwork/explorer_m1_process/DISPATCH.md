# DISPATCH — explorer_m1_process
Target: Milestone 1 Process Guard & Safety Blacklist in crates/pc-doctor-core/src/processes/
Project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md
Original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_process

## 2026-10-16T07:05:05Z
Task:
Analyze and design the exact implementation for `pc-doctor-core::processes`:
1. Process categorization logic (System, BackgroundService, Application, BrowserEngine).
2. Runaway process detection criteria (CPU >= 90% or RAM > 2GB or dual pressure).
3. Hardcoded Windows OS process safety blacklist: csrss.exe, lsass.exe, services.exe, explorer.exe, smss.exe, wininit.exe, winlogon.exe, svchost.exe, dwm.exe, PID <= 4, and self PID.
4. Structured safety error definition (`ProcessTerminationError::ProtectedSystemProcess`) returning structured reason.
5. Unit test suite specifications verifying that attempting to terminate protected processes returns the structured safety error, unprotected processes are approved, and categories are accurate.
Write your complete analysis and implementation blueprint to d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_process/handoff.md. Update progress.md with your liveness.
Send a message to your caller upon completion with the path to your handoff report.
