# Frontend Architectural Survey & Component Integration Plan for PC Doctor (TBK Core)

**Report Date**: 2026-10-16  
**Author**: Frontend Architecture Explorer  
**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_survey_frontend`  
**Target Codebase**: `d:/HOCC++/pc-doctor/apps/desktop`  
**Reference Document**: `d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md`  
**Backend Survey Reference**: `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_survey_backend/handoff.md`  

---

## 1. Observation

### 1.1 Directory Structure & Build Configuration
The React desktop application resides at `d:/HOCC++/pc-doctor/apps/desktop`.

#### File Hierarchy Observed:
```
apps/desktop/
├── index.html                   # HTML template (lang="vi", data-theme="light", #root)
├── package.json                 # React 19.1.0, @tauri-apps/api v2, Vite 8
├── tsconfig.json                # ES2020 target, ESNext module, bundler resolution, strict mode
├── tsconfig.node.json           # Node configuration for Vite config
├── vite.config.ts               # Vite 8 config with Tauri dev server host and port 1420
└── src/
    ├── main.tsx                 # Root bootstrap, renders <App /> into #root in StrictMode
    ├── App.tsx                  # AppInner shell: layout, header navbar, theme/lang toggles, companion dock, routing
    ├── App.css                  # Core layout styling, header, nav, cards, badges, progress bars
    ├── types.ts                 # Local TypeScript interfaces for telemetry, processes, diagnostics
    ├── vite-env.d.ts            # Vite client types
    ├── hooks/
    │   └── useMonitoring.ts     # Polling hook (get_system_snapshot, get_process_list), mood calculation
    ├── i18n/
    │   ├── index.tsx            # I18nContext, I18nProvider, useI18n hook
    │   ├── vi.ts                # Vietnamese translation bundle
    │   └── en.ts                # English translation bundle
    ├── styles/
    │   └── theme.css            # Kernova design tokens: CSS custom variables, light/dark themes, resets
    └── components/
        ├── companion/
        │   ├── Companion.tsx    # Animated CSS robot avatar (eyes, mouth states, antenna)
        │   └── Companion.css
        ├── dashboard/
        │   ├── Dashboard.tsx    # Hero banner, CPU/RAM/OS/Disk/Temp cards, Diagnostic finding cards
        │   └── Dashboard.css
        ├── processes/
        │   ├── ProcessPanel.tsx # Process table, search filter, CPU/RAM sort toggle
        │   └── ProcessPanel.css
        ├── chat/
        │   ├── ChatPanel.tsx    # Offline assistant chat, live telemetry context strip, quick prompts
        │   └── ChatPanel.css
        └── settings/
            ├── SettingsPanel.tsx# Lang switch, theme switch, polling interval selector, About card
            └── SettingsPanel.css
```

#### Dependencies & Scripts (`package.json:6-25`):
- **Scripts**:
  - `"dev": "vite"`
  - `"build": "tsc && vite build"`
  - `"preview": "vite preview"`
  - `"tauri": "tauri"`
- **Dependencies**:
  - `"react": "^19.1.0"`
  - `"react-dom": "^19.1.0"`
  - `"@tauri-apps/api": "^2"`
  - `"@tauri-apps/plugin-opener": "^2"`
- **DevDependencies**:
  - `"@types/react": "^19.1.8"`, `"@types/react-dom": "^19.1.6"`
  - `"@vitejs/plugin-react": "^6.0.2"`
  - `"typescript": "~6.0.3"`
  - `"vite": "^8.0.16"`
  - `"@tauri-apps/cli": "^2"`
- **External UI/Routing Libraries**: None. The project uses pure idiomatic React 19 + pure CSS variables with zero bloated dependencies (no Tailwind, no MUI, no Lucide, no React Router). All components are lightweight, fast, and self-contained.

#### Build Verification:
Execution of `npm run build` in `apps/desktop`:
```
> desktop@0.1.0 build
> tsc && vite build

vite v8.3.4 building client environment for production...
transforming...
✓ 33 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                   0.50 kB │ gzip:  0.34 kB
dist/assets/index-Bq9vP0ui.css   27.03 kB │ gzip:  5.53 kB
dist/assets/index-BVcdlEQU.js   255.70 kB │ gzip: 78.80 kB
✓ built in 160ms
```
- Exit code: `0`.
- Zero compiler or bundler errors.

---

### 1.2 Current Status of UI Screens by Requirement

| UI Screen | Current File(s) | Status | Implemented Features | Missing / Deficient Features (Gaps) |
| :--- | :--- | :--- | :--- | :--- |
| **Dashboard & Diagnostics (R1)** | `components/dashboard/Dashboard.tsx`, `Dashboard.css` | **95% Functional** | Stethoscope brand hero banner, system status indicator, live CPU sparkline, detailed CPU stats with per-core bars, RAM & Swap metrics, OS host/uptime/kernel, storage mount points, hardware temperature fallback, diagnostic findings cards with severity badges, evidence, and actionable steps. | Missing direct navigation link from critical/high findings to trigger Storage Cleaner or Process Manager actions. |
| **Storage Cleaner (R2)** | `App.tsx:214-277` (`StorageAdvisorSection`) | **10% Static Placeholder** | Static view of disk usage bars and 3 informational cards (Recycle bin, %TEMP%, Downloads). | **Entirely missing actionable scanning & purge suite**: No scan button, no itemized breakdown of cleanable locations (`%TEMP%`, crash dumps, WER reports, thumbnails), no dry-run/preview toggle, no purge confirmation modal, no execution of purge, no summary of freed bytes or locked files skipped. |
| **Process Manager (R3)** | `components/processes/ProcessPanel.tsx`, `ProcessPanel.css` | **40% Read-Only Table** | List processes, sort by CPU/RAM, search filter by PID or name, refresh button, highlights high-CPU processes (>80%). | **No category filters** (All, Applications, Background Services, System, Browser Engines). **No process termination action**. **No protected status badge** protecting vital OS processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`). **No termination confirmation dialog**. |
| **Offline Assistant (R4)** | `components/chat/ChatPanel.tsx`, `ChatPanel.css` | **90% Functional** | Live telemetry context strip (CPU %, RAM %, Drive %, findings count), message history bubbles, markdown parsing, copy button, quick prompt buttons, Enter-to-send textarea, calls `query_assistant` with `lang`. | Assistant doesn't offer direct one-click CTA actions into the storage cleaner or process manager when answering optimization questions. |
| **Companion Widget** | `components/companion/Companion.tsx`, `Companion.css` | **100% Functional** | Docked right sidebar, dynamic emotional state (`healthy`, `warning`, `critical`, `thinking`, `offline`, `idle`) driven by telemetry and rule findings, animated CSS face & body. | Fully functional. |
| **Settings Panel** | `components/settings/SettingsPanel.tsx`, `SettingsPanel.css` | **100% Functional** | Language toggle (VI / EN), Theme toggle (Light / Dark), Polling interval selector (1s, 2s, 5s), About card. | Fully functional. |

---

### 1.3 Localization Setup (Bilingual VI / EN)
- **Architecture**:
  - Context Provider: `I18nProvider` in `src/i18n/index.tsx`.
  - State: `lang: Language` ('vi' | 'en'), persisted in `localStorage.getItem('pc_doctor_lang')`.
  - Hook: `useI18n()` providing `{ lang, setLang, t }`.
  - Bundles: `src/i18n/vi.ts` and `src/i18n/en.ts`.
  - Type-safe dictionary: `Translations = typeof en`.
- **Switching Mechanism**:
  - Header has `.lang-toggle-btn` (`VI / EN`) for instant one-click toggle.
  - Settings panel offers explicit button choices (`🇻🇳 Tiếng Việt`, `🇬🇧 English`).
  - Switching `lang` immediately updates context `t`, causing all consuming components to re-render synchronously with zero page reload.
  - Chat panel sends current `lang` to Tauri command `query_assistant`, ensuring backend assistant responses match the selected language.
- **Deficiencies**:
  - `vi.ts` and `en.ts` lack translation keys for the Storage Cleaner suite (scanning, targets, dry-run, purge confirmation, freed bytes summary, skipped locked files) and Process Manager guardrails (categories, terminate button, protected warning).

---

### 1.4 Kernova Design System Styling & Theming
- Defined in `src/styles/theme.css`:
  - **Light Theme (Default)**:
    - Primary background: `#f8fafc` (slate-50)
    - Card background: `#ffffff`
    - Brand navy: `#0f2744` (used for titles, primary buttons, stethoscope logo)
    - Accent teal: `#0d9488` (health status indicator, pulse dot)
    - Accent cyan: `#0284c7` (sparklines, focus rings)
  - **Dark Theme (`[data-theme="dark"]`)**:
    - Primary background: `#0b1320`
    - Card background: `#152238`
    - Card hover: `#1c2d49`
    - Brand navy: `#38bdf8`
    - Text primary: `#f8fafc`
  - **Component Tokens**:
    - Border radii: `--radius-sm: 8px`, `--radius-md: 12px`, `--radius-lg: 18px`, `--radius-xl: 24px`, `--radius-full: 9999px`.
    - Box shadows: `--shadow-sm`, `--shadow-md`, `--shadow-lg`.
    - Severity badges: `.badge.critical` (red), `.badge.high`/`.moderate` (amber), `.badge.low` (sky), `.badge.informational` (slate).
    - Status progress bars: `.progress-bar-track` with `.progress-bar-fill.healthy`, `.warning`, `.critical`.
  - **Accessibility**: Includes `@media (prefers-reduced-motion: reduce)` rules that automatically disable animations and transitions for sensitive users.

---

### 1.5 IPC Invoke Wrappers & Backend Communication
- **Current Observation**:
  - `invoke` from `@tauri-apps/api/core` is called ad-hoc directly within component files:
    - `useMonitoring.ts:16-17`: `invoke<SystemSnapshot>('get_system_snapshot')`, `invoke<ProcessInfo[]>('get_process_list')`.
    - `ChatPanel.tsx:86`: `invoke<{ reply: string; references: string[] }>('query_assistant', { prompt, lang })`.
  - There is no unified API wrapper layer or centralized error handling.
  - Missing commands needed for R2 and R3:
    - `scan_cleanable_storage`
    - `clean_storage_targets`
    - `terminate_process`

---

## 2. Logic Chain

### 2.1 Storage Cleaner Integration (R2)
- **Observation**: Requirement R2 mandates:
  1. Safe scanning of Windows temporary directories (`%TEMP%`, crash dumps, application logs, thumbnails).
  2. Exact reclaimable byte sizes with itemized breakdown.
  3. Purge execution strictly upon user confirmation with dry-run/preview support.
  4. Reporting freed bytes and locked files safely skipped.
- **Deduction**:
  - The static `StorageAdvisorSection` in `App.tsx` must be replaced by a full-featured component `StorageCleanerPanel.tsx` in `src/components/storage/`.
  - The component must manage a deterministic state machine: `idle` -> `scanning` -> `scanned` -> `confirming_purge` -> `purging` -> `completed`.
  - To fulfill dry-run/preview, a prominent checkbox toggle must default to safe preview or allow toggling dry-run vs real purge, passing `dry_run: boolean` to the backend.
  - To prevent accidental deletion, a purge confirmation dialog must list the selected target categories and total bytes before dispatching the invoke call.
  - When the backend returns `StorageCleanSummary`, the UI must display a clear breakdown: `bytes_freed`, `files_deleted`, and `files_skipped_locked` (reassuring the user that locked files were handled safely without error).

### 2.2 Process Manager Guardrails & Categorization (R3)
- **Observation**: Requirement R3 mandates:
  1. Categorization (CPU/RAM consumers, app vs background service).
  2. Safe process termination with a hardcoded safety blacklist protecting vital Windows OS processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`).
- **Deduction**:
  - `ProcessPanel.tsx` must be upgraded to accept enhanced process items containing `category: ProcessCategory` and `is_protected: boolean`.
  - A category filter bar (`Tất cả / All`, `Ứng dụng / Apps`, `Dịch vụ / Services`, `Hệ thống / System`, `Trình duyệt / Browsers`) should be rendered above the table.
  - In each row:
    - If `is_protected == true`: render a `🔒 Hệ thống` / `🔒 System` badge and disable or hide the termination button, with tooltip explaining it is protected by TBK Core safety guardrails.
    - If `is_protected == false`: render a `Dừng / Terminate` button.
  - Clicking `Terminate` opens a confirmation modal displaying the process name, PID, and memory usage.
  - On confirm, it invokes `terminate_process(pid)`.
  - If the backend returns success, it triggers `refetch()`. If the backend returns a safety rejection error, the UI displays the error in an alert banner.

### 2.3 Centralized IPC Client Module (`src/api/ipc.ts`)
- **Observation**: Components currently call raw `invoke('command_name', ...)` strings.
- **Deduction**:
  - Centralizing all IPC calls into `src/api/ipc.ts` enforces type safety, prevents string typos, provides a single place for IPC logging/error transformation, and allows graceful mock fallbacks when developing in browser mode (`npm run dev` outside Tauri).

---

## 3. Caveats

1. **Browser vs Tauri Runtime Context**:
   - Running `npm run dev` in a standard browser environment outside Tauri will throw `window.__TAURI_INTERNALS__ is undefined` when calling `invoke`. The IPC wrapper should catch this error or provide mock development fallbacks so UI developers can test components in browser mode without launching the full Rust binary.
2. **File Size Formatting**:
   - Temporary file sizes range from kilobytes (logs) to gigabytes (crash dumps). Size formatters must dynamically format bytes into `B`, `KB`, `MB`, and `GB` with consistent decimal precision.
3. **Vietnamese Diacritics & Accessibility**:
   - All Vietnamese UI strings must use standard Unicode precomposed forms (`NFC`) to avoid rendering artifacts on Windows font fallbacks (Segoe UI).

---

## 4. Conclusion & Proposed Frontend Architecture

### 4.1 Target Frontend Directory Structure

```
apps/desktop/src/
├── main.tsx
├── App.tsx                              [Integrated with StorageCleanerPanel, updated nav]
├── App.css
├── types.ts                             [Added CleanableTarget, StorageScanReport, StorageCleanSummary, ProcessCategory, TerminateResult]
├── api/
│   └── ipc.ts                          [NEW: Typed Tauri IPC invoke wrappers]
├── hooks/
│   ├── useMonitoring.ts
│   └── useStorageCleaner.ts            [NEW: Scanning, dry-run state, purge execution hook]
├── i18n/
│   ├── index.tsx
│   ├── vi.ts                           [Added storage cleaner & process guardrail strings]
│   └── en.ts                           [Added storage cleaner & process guardrail strings]
├── styles/
│   └── theme.css
└── components/
    ├── companion/
    │   ├── Companion.tsx
    │   └── Companion.css
    ├── dashboard/
    │   ├── Dashboard.tsx
    │   └── Dashboard.css
    ├── storage/                         [NEW: Storage Cleaner & Cache Purge Suite]
    │   ├── StorageCleanerPanel.tsx      [Scan trigger, itemized breakdown, dry-run toggle, purge modal, results summary]
    │   └── StorageCleanerPanel.css
    ├── processes/
    │   ├── ProcessPanel.tsx             [Enhanced with category tabs, protected status badges, terminate modal]
    │   └── ProcessPanel.css
    ├── chat/
    │   ├── ChatPanel.tsx
    │   └── ChatPanel.css
    └── settings/
        ├── SettingsPanel.tsx
        └── SettingsPanel.css
```

---

### 4.2 Data Models & TypeScript Contracts (`src/types.ts`)

Add the following interfaces to `apps/desktop/src/types.ts`:

```typescript
// Process Management Types (R3)
export type ProcessCategory = 'system' | 'backgroundservice' | 'application' | 'browserengine';

export interface ProcessItem {
  pid: number;
  name: string;
  cpu_percent: number;
  memory_bytes: number;
  status: string;
  category: ProcessCategory;
  is_protected: boolean;
}

export interface TerminateResult {
  pid: number;
  process_name: string;
  success: boolean;
  message: string;
}

// Storage Cleaner Types (R2)
export interface CleanableTarget {
  id: string;
  name_en: string;
  name_vi: string;
  description_en: string;
  description_vi: string;
  root_path: string;
  file_count: number;
  total_bytes: number;
  is_safe: boolean;
}

export interface StorageScanReport {
  targets: CleanableTarget[];
  total_reclaimable_bytes: number;
  total_files: number;
  scanned_at: string;
}

export interface CleanRequest {
  target_ids: string[];
  dry_run: boolean;
}

export interface StorageCleanSummary {
  dry_run: boolean;
  bytes_freed: number;
  files_deleted: number;
  files_skipped_locked: number;
  targets_processed: string[];
  errors: string[];
}

export interface AssistantResponse {
  reply: string;
  references: string[];
}
```

---

### 4.3 Typed IPC Client Module (`src/api/ipc.ts`)

```typescript
import { invoke } from '@tauri-apps/api/core';
import type {
  SystemSnapshot,
  ProcessItem,
  TerminateResult,
  StorageScanReport,
  CleanRequest,
  StorageCleanSummary,
  AssistantResponse,
} from '../types';

export const ipcApi = {
  // R1: Telemetry & Diagnostics
  getSystemSnapshot: async (): Promise<SystemSnapshot> => {
    return await invoke<SystemSnapshot>('get_system_snapshot');
  },

  // R1 & R3: Process Management
  getProcessList: async (): Promise<ProcessItem[]> => {
    return await invoke<ProcessItem[]>('get_process_list');
  },

  terminateProcess: async (pid: number): Promise<TerminateResult> => {
    return await invoke<TerminateResult>('terminate_process', { pid });
  },

  // R2: Storage & Temp Cache Cleaner
  scanCleanableStorage: async (): Promise<StorageScanReport> => {
    return await invoke<StorageScanReport>('scan_cleanable_storage');
  },

  cleanStorageTargets: async (request: CleanRequest): Promise<StorageCleanSummary> => {
    return await invoke<StorageCleanSummary>('clean_storage_targets', { request });
  },

  // R4: Offline Assistant
  queryAssistant: async (prompt: string, lang: string): Promise<AssistantResponse> => {
    return await invoke<AssistantResponse>('query_assistant', { prompt, lang });
  },
};
```

---

### 4.4 Detailed Component Specifications

#### A. Storage Cleaner Panel (`src/components/storage/StorageCleanerPanel.tsx`)
1. **Header & Scan Trigger**:
   - Page title: `{t.storage_cleaner_title}`
   - Action button: `{scanning ? t.storage_scanning : t.storage_scan_btn}`
   - Live summary banner displaying total reclaimable space in GB/MB and total cleanable files.
2. **Itemized Breakdown List**:
   - Checkbox header: Select all / Deselect all.
   - List of `CleanableTarget`:
     - Checkbox for individual selection.
     - Target icon (e.g. 🗑️ User Temp, 📂 Crash Dumps, 🖼️ Thumbnails).
     - Target name & description in user's active language (`target.name_vi` vs `target.name_en`).
     - Root path chip with mono font.
     - Badge: `An toàn / Safe`.
     - File count and formatted size.
3. **Execution Controls**:
   - Dry-Run / Preview toggle checkbox:
     - Checked by default for initial safety.
     - Label: `{t.storage_dry_run_label}`
     - Hint: `{t.storage_dry_run_hint}`
   - Clean Button:
     - Styled with danger/warning theme.
     - Disabled if `selectedTargets.length === 0` or while running.
     - Label: `dryRun ? t.storage_preview_btn : t.storage_clean_btn`
4. **Purge Confirmation Modal**:
   - Triggered when clicking Clean in real (non-dry-run) mode.
   - Summarizes: Number of targets, total bytes scheduled for purge.
   - Explains: Open/locked files will be safely skipped.
   - Two buttons: `Hủy / Cancel` and `Xác nhận Dọn dẹp / Confirm Purge`.
5. **Results Summary Banner**:
   - Appears after execution.
   - Shows:
     - Mode: `Xem trước (Dry-run)` vs `Đã hoàn tất dọn dẹp (Completed)`.
     - `Dung lượng đã giải phóng`: `{fmtBytes(summary.bytes_freed)}`.
     - `Số tệp đã xóa`: `{summary.files_deleted}`.
     - `Tệp được bảo vệ / đang mở đã bỏ qua an toàn`: `{summary.files_skipped_locked}`.

#### B. Process Manager Enhancements (`src/components/processes/ProcessPanel.tsx`)
1. **Category Filter Tabs**:
   - Tabs: `[Tất cả]`, `[Ứng dụng]`, `[Dịch vụ]`, `[Hệ thống]`, `[Trình duyệt]`.
   - Filters processes according to `p.category`.
2. **Safety Status & Termination Column**:
   - Added column: `Thao tác / Actions`.
   - If `p.is_protected`:
     - Renders a protected badge: `🛡️ Hệ thống` (subtle slate/blue).
     - Tooltip: `Tiến trình hệ thống Windows được bảo vệ bởi TBK Core`.
     - No terminate button rendered, eliminating accidental termination.
   - If not protected:
     - Renders a `Dừng / Terminate` button (`btn-terminate`).
3. **Termination Modal & Feedback**:
   - Shows confirmation modal: `"Bạn có chắc chắn muốn dừng tiến trình {proc.name} (PID: {proc.pid})?"`
   - Invokes `ipcApi.terminateProcess(proc.pid)`.
   - On error: Displays warning notification banner.
   - On success: Dispatches `refetch()`.

---

### 4.5 Localization Additions (`vi.ts` and `en.ts`)

#### Vietnamese (`vi.ts`):
```typescript
// Storage Cleaner Suite (R2)
storage_cleaner_title: 'Dọn dẹp Bộ nhớ Tạm & Tệp Rác',
storage_cleaner_sub: 'Quét và dọn dẹp an toàn các thư mục tạm hệ thống Windows',
storage_scan_btn: 'Quét tệp rác ngay',
storage_scanning: 'Đang quét hệ thống...',
storage_total_reclaimable: 'Có thể giải phóng',
storage_files_count: 'tệp rác tìm thấy',
storage_targets_heading: 'Danh mục tệp có thể dọn dẹp',
storage_select_all: 'Chọn tất cả',
storage_dry_run_label: 'Chế độ xem trước (Dry-run)',
storage_dry_run_hint: 'Chỉ tính toán và mô phỏng dọn dẹp, không xóa tệp thật trên ổ đĩa',
storage_clean_btn: 'Dọn dẹp các mục đã chọn',
storage_preview_btn: 'Xem trước kết quả dọn dẹp',
storage_confirm_title: 'Xác nhận Dọn dẹp Tệp Rác',
storage_confirm_msg: 'Bạn có chắc chắn muốn dọn dẹp các mục đã chọn? Các tệp đang được ứng dụng khác mở sẽ được tự động bỏ qua an toàn.',
storage_confirm_btn: 'Xác nhận Dọn dẹp',
storage_cancel_btn: 'Hủy bỏ',
storage_result_title: 'Kết quả Dọn dẹp',
storage_freed_bytes: 'Dung lượng đã giải phóng',
storage_files_deleted: 'Số tệp đã dọn dẹp',
storage_skipped_locked: 'Tệp đang mở đã bỏ qua an toàn',
storage_clean_success: 'Dọn dẹp hoàn tất thành công!',
storage_no_targets_selected: 'Vui lòng chọn ít nhất một mục để dọn dẹp.',

// Process Manager Enhancements (R3)
proc_cat_all: 'Tất cả',
proc_cat_app: 'Ứng dụng',
proc_cat_service: 'Dịch vụ nền',
proc_cat_system: 'Hệ thống OS',
proc_cat_browser: 'Trình duyệt',
proc_action: 'Thao tác',
proc_terminate: 'Dừng',
proc_protected: 'Được bảo vệ',
proc_protected_tooltip: 'Tiến trình cốt lõi của Windows được bảo vệ, không thể dừng để tránh lỗi hệ thống.',
proc_terminate_confirm_title: 'Dừng tiến trình',
proc_terminate_confirm_msg: 'Bạn có chắc chắn muốn dừng tiến trình {name} (PID: {pid})?',
proc_terminate_success: 'Đã dừng tiến trình thành công.',
proc_terminate_error: 'Không thể dừng tiến trình.',
```

#### English (`en.ts`):
```typescript
// Storage Cleaner Suite (R2)
storage_cleaner_title: 'Temporary Files & Cache Cleaner',
storage_cleaner_sub: 'Safely scan and purge temporary Windows cache directories',
storage_scan_btn: 'Scan Cleanable Storage',
storage_scanning: 'Scanning system...',
storage_total_reclaimable: 'Reclaimable Space',
storage_files_count: 'cleanable files found',
storage_targets_heading: 'Cleanable Categories',
storage_select_all: 'Select All',
storage_dry_run_label: 'Dry-run preview mode',
storage_dry_run_hint: 'Simulate cleanup without actually deleting any files on disk',
storage_clean_btn: 'Purge Selected Items',
storage_preview_btn: 'Preview Cleanup Results',
storage_confirm_title: 'Confirm Storage Purge',
storage_confirm_msg: 'Are you sure you want to clean the selected targets? Locked or open files will be safely skipped.',
storage_confirm_btn: 'Confirm Purge',
storage_cancel_btn: 'Cancel',
storage_result_title: 'Cleanup Summary',
storage_freed_bytes: 'Storage Freed',
storage_files_deleted: 'Files Deleted',
storage_skipped_locked: 'Locked files safely skipped',
storage_clean_success: 'Cleanup completed successfully!',
storage_no_targets_selected: 'Please select at least one category to clean.',

// Process Manager Enhancements (R3)
proc_cat_all: 'All',
proc_cat_app: 'Applications',
proc_cat_service: 'Background Services',
proc_cat_system: 'OS System',
proc_cat_browser: 'Web Browsers',
proc_action: 'Action',
proc_terminate: 'End Process',
proc_protected: 'Protected',
proc_protected_tooltip: 'Core Windows OS process protected by TBK Core safety guardrails.',
proc_terminate_confirm_title: 'End Process Confirmation',
proc_terminate_confirm_msg: 'Are you sure you want to terminate process {name} (PID: {pid})?',
proc_terminate_success: 'Process terminated successfully.',
proc_terminate_error: 'Failed to terminate process.',
```

---

## 5. Verification Method

### 5.1 Commands to Verify Frontend Integrity
Run from `d:/HOCC++/pc-doctor/apps/desktop`:
```bash
# Verify TypeScript type check and production bundle creation
npm run build
```
- Expected Result: Exit code `0`, `tsc` passes with 0 type errors, Vite transforms and generates assets in `dist/`.

### 5.2 End-to-End Workspace Verification
Run from workspace root `d:/HOCC++/pc-doctor`:
```bash
# 1. Check workspace compilation
cargo check --workspace

# 2. Run backend unit tests
cargo test --workspace

# 3. Check frontend bundle
cd apps/desktop
npm run build
```

### 5.3 Invalidation Conditions
This survey and proposal will be invalidated if:
1. `npm run build` fails with TypeScript compile errors or missing modules.
2. The Storage Cleaner UI permits executing real purges without explicit user confirmation or dry-run option.
3. The Process Manager UI permits sending termination requests for protected Windows OS processes (`csrss.exe`, `lsass.exe`, `services.exe`, `explorer.exe`, `smss.exe`).
4. Language switching between `vi` and `en` leaves untranslated placeholders or fails to persist across reloads.
