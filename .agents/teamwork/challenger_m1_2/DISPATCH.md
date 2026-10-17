# DISPATCH — challenger_m1_2
Adversarial Verification of Process Guard & Safety Blacklist in crates/pc-doctor-core
Original Request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Project Plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md
Worker Handoff: d:/HOCC++/pc-doctor/.agents/teamwork/worker_m1/handoff.md
Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/challenger_m1_2
## 2026-10-16T07:25:42Z
Sender: d0833960-9ed1-454f-b7d6-be733f742605
Content:
You are Challenger 2 for Milestone 1 (Process Guard) in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/challenger_m1_2.
You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md
Path to worker handoff: d:/HOCC++/pc-doctor/.agents/teamwork/worker_m1/handoff.md

Task:
Adversarially challenge the Process Guard & Safety Blacklist in `crates/pc-doctor-core/src/processes/`:
1. Test mandatory protected processes: `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`, `wininit.exe`, `winlogon.exe`, `svchost.exe`, `dwm.exe`.
2. Test casing variations and extensions: `CSRSS.EXE`, `csrss`, `Explorer.exe`, `EXPLORER`.
3. Test PID boundaries: PID 0, PID 4, PID <= 4, self PID.
4. Test runaway criteria: CPU boundary (89.9% vs 90%), RAM boundary (2GB), dual pressure (CPU >= 70% & RAM >= 1.5GB).
5. Verify structured error `ProcessTerminationError::ProtectedSystemProcess` is returned on rejections.
Deliver an explicit verdict (APPROVE or REQUEST_CHANGES). Write your report to d:/HOCC++/pc-doctor/.agents/teamwork/challenger_m1_2/handoff.md. Update progress.md with your liveness.
Send a message to your caller upon completion with the path to your handoff report and verdict.
