# Adversarial Challenge Report — Milestone 3: Bilingual Localization & i18n

**Challenger**: Challenger 2 (`challenger_m3_2`)  
**Verdict**: **APPROVE**  
**Date**: 2026-10-16  

---

## 1. Observation

### 1.1 Dictionary Key Parity (`apps/desktop/src/i18n/en.ts` vs `apps/desktop/src/i18n/vi.ts`)
- Script execution comparing keys between `en.ts` (175 lines) and `vi.ts` (175 lines):
  ```
  EN key count: 155
  VI key count: 155
  Missing in VI: []
  Missing in EN: []
  EN duplicates: []
  VI duplicates: []
  Empty EN entries: []
  Empty VI entries: []
  KEY PARITY SUCCESS: 100% match, 0 missing, 0 duplicates.
  ```
- All 155 keys match exactly with zero missing keys, zero duplicate declarations, and zero empty string values.
- Shared values between English and Vietnamese are restricted to standard technical hardware acronyms and brand identifiers (`app_name: 'TBK Core'`, `website_link: 'Website: https://kernova.click/'`, `proc_pid: 'PID'`, `proc_cpu: 'CPU (%)'`, `proc_ram: 'RAM (MB)'`).

### 1.2 Untranslated Text & Hardcoded String Scan (AST Inspection)
- Inspected TypeScript AST of `apps/desktop/src/components/storage/StorageCleanerPanel.tsx` (406 lines) and `apps/desktop/src/components/processes/ProcessPanel.tsx` (358 lines) using TypeScript 6.0.3 Compiler API (`ts.isJsxText`, `ts.isJsxAttribute`, `ts.isJsxExpression`).
- **`StorageCleanerPanel.tsx`**:
  - Raw JSX text nodes found: only emojis (`⏳` line 140, `🔍` line 145, `✕` line 199, `⚠️` lines 236/243, `🛡️` line 329) and unit symbol (`%` line 165).
  - Cleanable target bilingual property switching is implemented at lines 307-308:
    ```tsx
    const title = lang === 'vi' ? target.name_vi : target.name_en;
    const desc = lang === 'vi' ? target.description_vi : target.description_en;
    ```
  - All modal dialog headers, descriptions, statistic labels, action buttons, and error banners use `t.<key>` (`t.storage_cleaner_title`, `t.storage_scan_btn`, `t.storage_disk_overview`, `t.storage_results_title`, `t.storage_mode_dryrun`, `t.storage_mode_live`, `t.storage_dismiss`, `t.storage_bytes_freed`, `t.storage_simulated_freed`, `t.storage_files_deleted`, `t.storage_files_skipped`, `t.storage_errors_title`, `t.storage_scan_error`, `t.storage_clean_error`, `t.storage_confirm_dryrun_title`, `t.storage_confirm_purge_title`, `t.storage_confirm_dryrun_desc`, `t.storage_confirm_purge_desc`, `t.storage_modal_selected_targets`, `t.storage_modal_selected_bytes`, `t.storage_modal_selected_files`, `t.storage_btn_confirm`, `t.storage_btn_cancel`).
- **`ProcessPanel.tsx`**:
  - Raw JSX text nodes found: only hardware metrics and abbreviations (`CPU ↓` line 186, `RAM ↓` line 192, `MB` lines 253/332, `%` lines 250/328) and UI icons (`⏳`, `🔄`, `⚠️`, `✕`, `✅`, `🛡️`).
  - All category tab labels dynamically map through `getCategoryLabel`:
    - `All` → `t.proc_cat_all` ("Tất cả" / "All")
    - `Application` → `t.proc_cat_application` ("Ứng dụng" / "Application")
    - `BrowserEngine` → `t.proc_cat_browser` ("Trình duyệt" / "Browser")
    - `BackgroundService` → `t.proc_cat_service` ("Dịch vụ ngầm" / "Background Service")
    - `System` → `t.proc_cat_system` ("Hệ thống" / "System")
  - Protected status strings map to `t.proc_status_protected` ("Bảo vệ hệ thống" / "Protected") and `t.proc_status_unprotected` ("Bình thường" / "Unprotected") with tooltip `t.proc_protected_tooltip`.
  - Confirmation modal template replacement is verified at line 310:
    ```tsx
    {t.proc_confirm_desc.replace('{name}', targetToTerminate.name).replace('{pid}', String(targetToTerminate.pid))}
    ```
  - Error messages parse structured JSON errors and display localized guardrail messages (`t.proc_error_protected: ${parsed.details.reason}`, line 97).

### 1.3 Dynamic Language Switching Test Suite Execution
- Executed in-memory React 19 test harness rendering `App`, `StorageCleanerPanel`, `ProcessPanel`, and `Companion` under both `vi` and `en` locales:
  ```
  CHALLENGER 2: ADVERSARIAL i18n & LOCALIZATION SUITE
  --- TEST 1: Dictionary Key Parity & Integrity ---
    [PASS] Total key count matches (155 vs 155)
    [PASS] 0 keys missing in VI: []
    [PASS] 0 keys missing in EN: []
    [PASS] 0 empty values in EN
    [PASS] 0 empty values in VI
    [PASS] All language-specific strings are translated (untranslated: 0)

  --- TEST 2: In-Memory Component Transpilation & Module Singleton ---

  --- TEST 3: Dynamic Rendering & Translation Switching ---
  [3A] Testing StorageCleanerPanel Localization:
    [PASS] VI StorageCleaner contains: "Dọn dẹp Bộ nhớ Tạm An toàn"
    [PASS] VI StorageCleaner contains: "Quét bộ nhớ tạm"
    [PASS] VI StorageCleaner contains: "Tổng quan Ổ đĩa Hệ thống"
    [PASS] EN StorageCleaner contains: "Safe Storage Cache Cleaner"
    [PASS] EN StorageCleaner contains: "Scan Storage"
    [PASS] EN StorageCleaner contains: "System Disk Overview"
    [PASS] VI StorageCleaner does NOT leak English title
    [PASS] EN StorageCleaner does NOT leak Vietnamese title

  [3B] Testing ProcessPanel Localization:
    [PASS] VI ProcessPanel contains all category: "Tất cả"
    [PASS] VI ProcessPanel contains app category: "Ứng dụng"
    [PASS] VI ProcessPanel contains browser category: "Trình duyệt"
    [PASS] VI ProcessPanel contains service category: "Dịch vụ ngầm"
    [PASS] VI ProcessPanel contains system category: "Hệ thống"
    [PASS] VI ProcessPanel contains protected status: "Bảo vệ hệ thống"
    [PASS] VI ProcessPanel contains unprotected status: "Bình thường"
    [PASS] VI ProcessPanel contains terminate button: "Đóng"
    [PASS] EN ProcessPanel contains all category: "All"
    [PASS] EN ProcessPanel contains app category: "Application"
    [PASS] EN ProcessPanel contains browser category: "Browser"
    [PASS] EN ProcessPanel contains service category: "Background Service"
    [PASS] EN ProcessPanel contains system category: "System"
    [PASS] EN ProcessPanel contains protected status: "Protected"
    [PASS] EN ProcessPanel contains unprotected status: "Unprotected"
    [PASS] EN ProcessPanel contains terminate button: "Terminate"
    [PASS] VI ProcessPanel does NOT leak English title
    [PASS] EN ProcessPanel does NOT leak Vietnamese title

  [3C] Testing Companion View Localization:
    [PASS] VI Companion label contains: "Hệ thống ổn định"
    [PASS] EN Companion label contains: "System is healthy"
    [PASS] VI Companion does NOT leak English mood
    [PASS] EN Companion does NOT leak Vietnamese mood

  [3D] Testing Storage Cleanable Target Bilingual Property Switch:
    [PASS] VI target title correctly uses name_vi: "Tệp tạm người dùng (%TEMP%)"
    [PASS] VI target desc correctly uses description_vi: "Các tệp bộ đệm tạm thời do ứng dụng và bộ cài đặt tạo ra trong thư mục %TEMP%"
    [PASS] EN target title correctly uses name_en: "User Temporary Files (%TEMP%)"
    [PASS] EN target desc correctly uses description_en: "Temporary files created by running applications"

  --- TEST 4: App Shell Navigation & Header Switching ---
    [PASS] VI App header contains: "Trang chủ"
    [PASS] VI App header contains: "Bác sĩ AI"
    [PASS] VI App header contains: "Tiến trình"
    [PASS] VI App header contains: "Bảo mật"
    [PASS] VI App header contains: "Dọn dẹp lưu trữ"
    [PASS] VI App header contains: "Cài đặt"
    [PASS] VI App toggle button displays "VI / EN"
    [PASS] EN App header contains: "Home"
    [PASS] EN App header contains: "AI Doctor"
    [PASS] EN App header contains: "Processes"
    [PASS] EN App header contains: "Security"
    [PASS] EN App header contains: "Storage Clean"
    [PASS] EN App header contains: "Settings"
    [PASS] EN App toggle button displays "EN / VI"

  TOTAL SUITE RESULTS: 54 PASSED, 0 FAILED
  ```

### 1.4 Frontend Build Verification
- Command: `npm run build` in `d:/HOCC++/pc-doctor/apps/desktop`
- Result:
  ```
  > desktop@0.1.0 build
  > tsc && vite build

  vite v8.3.4 building client environment for production...
  transforming...
  ✓ 36 modules transformed.
  rendering chunks...
  computing gzip size...
  dist/index.html                   0.50 kB │ gzip:  0.34 kB
  dist/assets/index-DJhq2jMf.css   39.04 kB │ gzip:  7.08 kB
  dist/assets/index-DtsmJirR.js   273.95 kB │ gzip: 83.11 kB

  ✓ built in 179ms
  ```

### 1.5 Backend Regression Check
- Command: `cargo test --workspace` in `d:/HOCC++/pc-doctor`
- Result:
  - `desktop_lib`: 26 passed; 0 failed
  - `adversarial_process_ipc`: 10 passed; 0 failed
  - `adversarial_storage_ipc_tests`: 12 passed; 0 failed
  - `pc_doctor_core`: 27 passed; 0 failed
  - `adversarial_process_tests`: 16 passed; 0 failed
  - `adversarial_storage_tests`: 20 passed; 0 failed
  - Total: 111 passed; 0 failed.

---

## 2. Logic Chain

1. **Parity Check (Observation 1.1)**: Comparing the keys of `en.ts` and `vi.ts` revealed exact correspondence across all 155 entries with zero missing keys in either dictionary. This proves dictionary completeness and satisfies requirement 1.
2. **Static AST Analysis (Observation 1.2)**: Parsing `StorageCleanerPanel.tsx` and `ProcessPanel.tsx` through the TypeScript AST compiler demonstrated that zero natural language UI text is hardcoded. Emojis and standard acronyms (`CPU`, `RAM`, `MB`, `%`) remain universal across locales, while all strings, placeholders, dialogs, button labels, and status badges invoke `t.<key>` or dynamic model properties (`target.name_vi` / `target.name_en`). This satisfies requirement 2.
3. **Dynamic Reactivity Oracle (Observation 1.3)**: Simulating runtime language toggling via the `I18nProvider` React context demonstrated that state updates propagate to all mounted views:
   - Header navigation links switch dynamically between Vietnamese and English.
   - The Companion dock label updates dynamically with mood state.
   - Storage Cleaner panel re-renders all headers, overview cards, and itemized target descriptions using the active language without cross-locale leakage.
   - Process panel updates all category tab labels, table column headers, and termination dialog texts.
   This verifies requirement 3.
4. **Build & Backend Integrity (Observations 1.4 & 1.5)**: Running `npm run build` confirmed zero TypeScript type errors and zero Vite bundle issues. Running `cargo test --workspace` verified that all 111 Rust tests pass without regression. This satisfies requirement 4.
5. **Conclusion Derivation**: Since all five criteria are satisfied empirically without discrepancies or defects, the implementation is approved.

---

## 3. Caveats

- Hardware thermal sensor telemetry continues to report unavailable when ring-0 kernel drivers are not present in user-space Windows environments; this is expected behavior and properly handled by the localized error fallback message (`t.hardware_temps_unavailable`).

---

## 4. Conclusion

**Verdict: APPROVE**

Milestone 3 bilingual localization & i18n implementation meets 100% of specification requirements:
- 100% key parity across all 155 keys in `vi.ts` and `en.ts`.
- Zero untranslated hardcoded text in `StorageCleanerPanel.tsx` and `ProcessPanel.tsx`.
- Dynamic and instantaneous language switching across storage, process, header, and companion views.
- Clean production Vite/TypeScript build and 100% pass rate across workspace tests.

---

## 5. Verification Method

To independently reproduce the empirical findings:

1. **Run Vite Build**:
   ```powershell
   cd d:\HOCC++\pc-doctor\apps\desktop
   npm run build
   ```
   *Expected*: Exits with code 0 in <500ms, bundles `dist/index.html` and assets.

2. **Verify Key Parity**:
   ```powershell
   node -e "const fs = require('fs'); const e = Object.keys(eval('('+fs.readFileSync('apps/desktop/src/i18n/en.ts','utf8').replace('export const en = ','')+')')); const v = Object.keys(eval('('+fs.readFileSync('apps/desktop/src/i18n/vi.ts','utf8').replace('export const vi = ','')+')')); console.log('Parity:', e.length === v.length && e.every(k => v.includes(k)));"
   ```
   *Expected*: Outputs `Parity: true`.

3. **Run Workspace Cargo Tests**:
   ```powershell
   cd d:\HOCC++\pc-doctor
   cargo test --workspace
   ```
   *Expected*: 111 passed; 0 failed.
