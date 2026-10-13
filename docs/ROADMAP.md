# PC Doctor — Project Roadmap

## Phase 0: Toolchain & Scaffolding (Completed)
- Tauri 2 CLI, Rust toolchain, Node.js setup
- Monorepo workspace structure

## Phase 1: Core Library & Type Sharing (Completed)
- `crates/pc-doctor-core` diagnostic rules and models
- `packages/shared-types` TypeScript contracts

## Phase 2: User Experience & Design (Current)
- Original companion robot face with SVG/CSS micro-animations
- Bilingual i18n (Vietnamese & English)
- Dark / light mode support

## Phase 3: Live System Monitoring (Current)
- Rust `sysinfo` collector thread/state
- Real CPU, RAM, Disk, and Process monitoring
- Tauri IPC commands

## Phase 4: Deterministic Diagnostics & Offline Assistant
- Diagnostic finding cards with severity levels
- Offline conversational Q&A linked with real machine metrics
