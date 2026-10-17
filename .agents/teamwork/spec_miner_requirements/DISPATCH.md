# DISPATCH — spec_miner_requirements
Target: Mine and document exhaustive requirements, constraints, and contracts for R1-R4.
Original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements


## 2026-10-16T06:54:40Z
Sender: d0833960-9ed1-454f-b7d6-be733f742605
Priority: MESSAGE_PRIORITY_HIGH
Content:
You are a Requirements and Windows Spec Miner for PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements.
You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md

Task:
Extract and formalize precise specifications, constraints, and contracts for all 4 core requirements:
1. R1: Hardware telemetry metrics schema (CPU total & per-core %, RAM used/available/total, disks with mount/capacity/free, uptime in seconds, top CPU/RAM processes). Deterministic diagnostic rules engine: exact rules, metric thresholds, and mapping into severity tiers (Critical, High, Moderate, Low, Informational).
2. R2: Actionable Storage & Temp Cache Scanner: Exhaustive list of designated safe Windows temporary paths (%TEMP%, LocalAppData Temp, user crash dumps, application logs, Windows thumbnail/icon cache, prefetch if accessible). Safety boundaries: exact list of protected locations that must NEVER be touched (e.g. System32, Windows, user personal documents/desktop). Scanning return types (exact non-negative byte sizes, itemized breakdown, count). Clean/purge action specification (dry-run mode, locked/open file skip semantics with error recording, summary of freed bytes).
3. R3: Safe Process Inspection & Resource Management: Process listing schema (pid, name, exe path, CPU %, memory bytes, status, category: app vs background service). Runaway process detection criteria. Process safety blacklist: hardcoded list of protected Windows system processes (csrss.exe, lsass.exe, services.exe, explorer.exe, smss.exe, wininit.exe, winlogon.exe, etc.) that MUST return a structured safety error if termination is requested.
4. R4: Offline Assistant & Bilingual Guidance: Diagnostic explanation and optimization recommendation knowledge base in both Vietnamese (vi) and English (en). Assistant prompt/response format, deterministic fallback responses based on live telemetry and rule evaluations.
Write your exhaustive specification and contract definitions to d:/HOCC++/pc-doctor/.agents/teamwork/spec_miner_requirements/handoff.md. Update progress.md with your liveness.
Send a message to your caller upon completion with the path to your handoff report.
