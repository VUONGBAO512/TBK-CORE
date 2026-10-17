## 2026-10-16T08:21:10Z

You are the Implementation Worker for Milestone 3: Frontend UI & Bilingual Localization in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/worker_m3.

You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

File Ownership:
You have exclusive write ownership of:
- d:/HOCC++/pc-doctor/apps/desktop/src/types.ts
- d:/HOCC++/pc-doctor/apps/desktop/src/api/ipc.ts (create new)
- d:/HOCC++/pc-doctor/apps/desktop/src/hooks/useMonitoring.ts
- d:/HOCC++/pc-doctor/apps/desktop/src/components/storage/StorageCleanerPanel.tsx (create new)
- d:/HOCC++/pc-doctor/apps/desktop/src/components/storage/StorageCleanerPanel.css (create new)
- d:/HOCC++/pc-doctor/apps/desktop/src/components/processes/ProcessPanel.tsx
- d:/HOCC++/pc-doctor/apps/desktop/src/components/processes/ProcessPanel.css
- d:/HOCC++/pc-doctor/apps/desktop/src/i18n/vi.ts
- d:/HOCC++/pc-doctor/apps/desktop/src/i18n/en.ts
- d:/HOCC++/pc-doctor/apps/desktop/src/App.tsx

Implementation Requirements:
1. `apps/desktop/src/types.ts`:
   - Synchronize models with `PROJECT.md § Interface Contracts`:
     - `CleanableTarget` (id, name_en, name_vi, description_en, description_vi, root_path, file_count, total_bytes, is_safe)
     - `StorageScanReport` (targets: CleanableTarget[], total_reclaimable_bytes: number, total_files: number, scanned_at: string)
     - `CleanRequest` (target_ids: string[], dry_run: boolean)
     - `StorageCleanSummary` (dry_run: boolean, bytes_freed: number, files_deleted: number, files_skipped_locked: number, targets_processed: string[], errors: string[])
     - `ProcessCategory`: 'System' | 'BackgroundService' | 'Application' | 'BrowserEngine'
     - `ProcessItem`: (pid: number, name: string, cpu_percent: number, memory_bytes: number, status: string, category: ProcessCategory, is_protected: boolean)
     - `TerminateResult`: (pid: number, process_name: string, success: boolean, message: string)
     - `AssistantResponse`: (response: string, suggested_actions: string[], confidence: number, sources: string[])

2. `apps/desktop/src/api/ipc.ts`:
   - Typed Tauri 2 invoke wrappers using `@tauri-apps/api/core::invoke`:
     - `getSystemSnapshot(): Promise<SystemSnapshot>`
     - `getProcessList(): Promise<ProcessItem[]>`
     - `terminateProcess(pid: number, processName: string): Promise<TerminateResult>`
     - `scanCleanableStorage(): Promise<StorageScanReport>`
     - `cleanStorageTargets(request: CleanRequest): Promise<StorageCleanSummary>`
     - `queryAssistant(prompt: string, lang: string): Promise<AssistantResponse>`

3. `apps/desktop/src/hooks/useMonitoring.ts`:
   - Update `processes` state and return type to `ProcessItem[]` instead of `ProcessInfo[]`.

4. `apps/desktop/src/components/storage/StorageCleanerPanel.tsx` and `StorageCleanerPanel.css`:
   - Display Disk summary cards (mount point, total, used, available, percentage bar).
   - "Scan Storage" / "Quét bộ nhớ tạm" button with loading indicator.
   - Summary banner showing total reclaimable bytes and file count.
   - Itemized targets list/cards:
     - Checkbox for each target + "Select All" toggle.
     - Bilingual target title & description using current `lang` (e.g. `lang === 'vi' ? target.name_vi : target.name_en`).
     - Reclaimable size (formatted to MB/GB) and file count.
     - Safe badge ("An toàn" / "Safe").
   - Options bar:
     - Dry-run mode checkbox/toggle ("Dry-run / Preview" / "Mô phỏng an toàn, không xóa tệp").
     - "Clean Storage" / "Dọn dẹp bộ nhớ" button (disabled if 0 targets selected or cleaning in progress).
   - Confirmation Modal:
     - Clear warning dialog explaining the action (dry-run preview vs permanent purge).
     - Confirm / Cancel buttons.
   - Post-Clean Results Banner:
     - Displays `bytes_freed` (or simulated freed in dry-run), `files_deleted`, `files_skipped_locked` ("Đã bỏ qua tệp đang sử dụng"), and errors if any.

5. `apps/desktop/src/components/processes/ProcessPanel.tsx` and `ProcessPanel.css`:
   - Accept `processes: ProcessItem[]`.
   - Category filtering tabs: All, Application, BrowserEngine, BackgroundService, System with count badges.
   - Search filter (name / PID) and sort toggles (CPU / RAM).
   - Table columns: PID, Name, Category, CPU%, RAM (MB), Protection Status, Action.
   - Category badges with distinct styling for each category.
   - Protection badge: Shield icon with "Protected" / "Bảo vệ hệ thống" for `is_protected === true`.
   - Terminate Action:
     - If `p.is_protected`: button is disabled with tooltip explaining Windows system process cannot be terminated.
     - If unprotected: "Đóng" / "Terminate" button.
     - Clicking button opens confirmation modal ("Bạn có chắc muốn đóng tiến trình <name> (PID <pid>) không?").
     - On confirm, calls `terminateProcess(p.pid, p.name)`. On success, calls `refetch()`. On error, displays structured error alert.

6. `apps/desktop/src/i18n/vi.ts` and `en.ts`:
   - Add all necessary bilingual translation keys for storage cleaner, process manager categories, badges, modals, dry-run toggles, and status messages.

7. `apps/desktop/src/App.tsx`:
   - In the storage tab, render `<StorageCleanerPanel snapshot={snapshot} />`.
   - Ensure full bilingual switching between Vietnamese (`vi`) and English (`en`) across all views.
   - Maintain styling consistency with Kernova design tokens in `styles/theme.css`.

8. Verification:
   - Run `npm run build` in `apps/desktop` to ensure 0 TypeScript compilation errors and 0 Vite bundling errors.
   - Run `cargo test --workspace` to ensure all 111 backend tests continue passing.
   - Write your complete handoff report to `d:/HOCC++/pc-doctor/.agents/teamwork/worker_m3/handoff.md`.
   - Send a message to your caller when done.
