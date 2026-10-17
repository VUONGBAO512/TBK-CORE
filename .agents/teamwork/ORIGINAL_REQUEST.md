# Original User Request

## 2026-10-16T06:52:27Z

Build and verify the Actionable Diagnostics & Safe Cleanup suite for PC Doctor (TBK Core), advancing the desktop application to a fully functional, verified MVP.

Working directory: d:/HOCC++/pc-doctor
Integrity mode: development

## Requirements

### R1. Deterministic System Diagnostics & Health Telemetry
The core engine must provide real-time hardware telemetry (CPU global and per-core usage, RAM used/available, disk volumes and capacity, uptime, and top resource-consuming processes) with deterministic diagnostic rules evaluating findings into severity tiers (Critical, High, Moderate, Low, Informational).

### R2. Actionable Storage & Temp Cache Scanner
The system must provide safe scanning and calculation of reclaimable storage across safe Windows temporary directories (such as `%TEMP%`, user crash dumps, application log files, and Windows thumbnail/prefetch caches where accessible). The scanner must compute exact reclaimable byte sizes, display an itemized breakdown, and execute safe purges only upon user confirmation with dry-run/preview support and protection against open/locked files.

### R3. Safe Process Inspection & Resource Management
The application must list and filter running processes with categorization (CPU/RAM consumers, app vs background service). It must detect runaway processes exceeding healthy thresholds and provide safe process termination with a hardcoded safety blacklist protecting vital Windows operating system processes (e.g. `csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`).

### R4. Offline Assistant & Bilingual Guidance
The offline companion assistant must provide context-aware diagnostic explanations and optimization recommendations in both Vietnamese and English, answering user questions about current machine state using live telemetry and the built-in diagnostic rule knowledge base.

## Acceptance Criteria

### Build & Type Safety
- [ ] `cargo check --workspace` and `cargo test --workspace` pass with zero failures or unhandled panics.
- [ ] `npm run build` inside `apps/desktop` succeeds with zero TypeScript or Vite bundle errors.

### Safe Disk Cleaner Verification
- [ ] Scanning temporary directories returns a structured list of cleanable targets with verified non-negative byte sizes.
- [ ] Cleaning actions safely skip locked or in-use files without crashing or panicking, returning a summary of freed bytes.
- [ ] Core operating system files and directories outside designated safe temp locations are strictly untouched.

### Process Safety Guardrails
- [ ] System process blacklist unit test verifies that attempting to terminate protected processes returns a structured safety error.
- [ ] Successfully queries top processes sorted by CPU and memory consumption.

### User Interface & Localization
- [ ] Storage cleaning and process views are integrated into the React desktop interface matching the Kernova design system.
- [ ] All UI strings and assistant responses switch seamlessly between Vietnamese (`vi`) and English (`en`).

## 2026-10-17T12:30:30Z

Server resumed. cargo test --workspace and npm run build both pass 100% cleanly with all 197 E2E tests passing. Please complete the Sentinel independent Victory Audit and provide the final report.
