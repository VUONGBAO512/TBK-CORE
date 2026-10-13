# PC Doctor — Architecture Documentation

## Overview
PC Doctor is an independent, native desktop health assistant built using Tauri 2, Rust, React, and TypeScript. It monitors local system metrics, detects hardware and performance bottlenecks deterministically, and provides a companion assistant interface without sending sensitive telemetry to third parties.

## System Topology

```
+-------------------------------------------------------+
|                   React 18 Frontend                   |
|  - Companion Avatar (CSS/SVG reactive state machine)  |
|  - Health Dashboard (Real-time CPU/RAM/Disks charts)  |
|  - Process Inspector (Live PID, CPU%, Memory table)   |
|  - Diagnostic Chat (Local deterministic engine)       |
|  - Bilingual i18n (English & Vietnamese)              |
+---------------------------+---------------------------+
                            | Tauri IPC (invoke)
+---------------------------v---------------------------+
|               apps/desktop/src-tauri                  |
|  - Tauri Commands (`get_system_snapshot`, etc.)       |
|  - Collector Module (`sysinfo 0.33`)                  |
|  - State Manager & Mutex (Thread-safe caching)        |
+---------------------------+---------------------------+
                            | Rust workspace dep
+---------------------------v---------------------------+
|               crates/pc-doctor-core                   |
|  - Pure Rust Models (No GUI dependencies)             |
|  - Deterministic Diagnostic Rule Engine               |
|  - Local Bilingual Knowledge Base                     |
+-------------------------------------------------------+
```

## Modules
1. **`pc-doctor-core`**: Defines all core data contracts (`SystemSnapshot`, `DiagnosticFinding`, etc.), rule evaluations (`PERF_CPU_HIGH_SUSTAINED`, `DISK_LOW_CAPACITY`, etc.), and offline knowledge base articles.
2. **`desktop/src-tauri`**: Rust backend interacting with Windows APIs through `sysinfo`.
3. **`apps/desktop/src`**: React + TypeScript user interface with dark/light theme, SVG animations, and i18n.
4. **`packages/shared-types`**: Shared TypeScript type definitions matching Rust serde models.
