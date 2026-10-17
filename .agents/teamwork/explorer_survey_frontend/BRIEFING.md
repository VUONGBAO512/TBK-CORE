# BRIEFING — 2026-10-16T07:10:00Z

## Mission
Investigate the React frontend in d:/HOCC++/pc-doctor/apps/desktop (structure, build tools, UI screens, i18n, Kernova design tokens, IPC wrappers) and deliver a complete architectural survey and integration plan.

## 🔒 My Identity
- Archetype: explorer
- Roles: Frontend Architecture Explorer for PC Doctor (TBK Core)
- Working directory: d:/HOCC++/pc-doctor/.agents/teamwork/explorer_survey_frontend
- Original parent: d0833960-9ed1-454f-b7d6-be733f742605
- Milestone: Survey & Architecture Analysis

## 🔒 Key Constraints
- Read-only investigation — do NOT implement or modify source code
- Map out existing frontend architecture, build tools, screens, i18n, design system, and IPC invoke wrappers
- Deliver a 5-component handoff report (Observation, Logic Chain, Caveats, Conclusion, Verification Method) in handoff.md

## Current Parent
- Conversation ID: d0833960-9ed1-454f-b7d6-be733f742605
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `apps/desktop/package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`
  - `apps/desktop/src/main.tsx`, `App.tsx`, `App.css`, `types.ts`
  - `apps/desktop/src/styles/theme.css`
  - `apps/desktop/src/i18n/index.tsx`, `vi.ts`, `en.ts`
  - `apps/desktop/src/components/dashboard/Dashboard.tsx`, `Dashboard.css`
  - `apps/desktop/src/components/processes/ProcessPanel.tsx`, `ProcessPanel.css`
  - `apps/desktop/src/components/chat/ChatPanel.tsx`, `ChatPanel.css`
  - `apps/desktop/src/components/companion/Companion.tsx`, `Companion.css`
  - `apps/desktop/src/components/settings/SettingsPanel.tsx`, `SettingsPanel.css`
  - `apps/desktop/src/hooks/useMonitoring.ts`
  - `.agents/teamwork/explorer_survey_backend/handoff.md`
- **Key findings**:
  - `npm run build` succeeds cleanly in 160ms with zero errors.
  - Dashboard and Offline Assistant are highly mature and functional.
  - Storage Cleaner is currently only a static tip display in `App.tsx` (`StorageAdvisorSection`) and lacks scanning, breakdown, dry-run, purge confirmation, and summary reporting.
  - Process Manager lists processes and sorts by CPU/RAM, but lacks category filtering, process termination, and protected Windows process indicators.
  - Localization is cleanly implemented via `I18nProvider` with `vi` and `en` resource bundles, requiring expansion for new cleaner & process features.
  - Kernova design system uses CSS custom properties with light/dark theme support and zero external CSS bloat.
  - Tauri IPC calls are currently ad-hoc raw strings; a typed `src/api/ipc.ts` module is recommended.
- **Unexplored areas**: None. Frontend survey is complete.

## Key Decisions Made
- Proposed full component plan for `StorageCleanerPanel.tsx` in `src/components/storage/`.
- Proposed enhancements for `ProcessPanel.tsx` with category tabs and protected OS process guardrails.
- Formulated typed IPC wrapper specification in `src/api/ipc.ts`.
- Formulated exact bilingual translation additions for `vi.ts` and `en.ts`.

## Artifact Index
- DISPATCH.md — Initial dispatch instructions
- BRIEFING.md — Persistent context and state
- progress.md — Liveness and execution heartbeat
- handoff.md — Complete 5-component architectural handoff report
