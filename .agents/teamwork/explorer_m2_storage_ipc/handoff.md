# Handoff Report: Milestone 2 Storage Tauri Commands (`apps/desktop/src-tauri`)

## 1. Observation

### Codebase State & Current Baseline
1. **Original Request (`d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md`)**:
   - Lines 15–17: Requirement R2 specifies safe scanning and calculation of reclaimable storage across Windows temporary directories (`%TEMP%`, user crash dumps, application log files, Windows thumbnail caches). Must compute exact reclaimable byte sizes, display an itemized breakdown, and execute safe purges only upon user confirmation with dry-run/preview support and protection against open/locked files.
   - Lines 30–33: Acceptance Criteria requires:
     - Scanning temporary directories returns a structured list of cleanable targets with verified non-negative byte sizes.
     - Cleaning actions safely skip locked or in-use files without crashing or panicking, returning a summary of freed bytes.
     - Operating system files and directories outside designated safe temp locations are strictly untouched.

2. **Project Plan (`d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md`)**:
   - Lines 53–87: Interface Contract #1 defines:
     ```rust
     pub struct CleanableTarget {
         pub id: String,
         pub name_en: String,
         pub name_vi: String,
         pub description_en: String,
         pub description_vi: String,
         pub root_path: String,
         pub file_count: usize,
         pub total_bytes: u64,
         pub is_safe: bool,
     }

     pub struct StorageScanReport {
         pub targets: Vec<CleanableTarget>,
         pub total_reclaimable_bytes: u64,
         pub total_files: usize,
         pub scanned_at: DateTime<Utc>,
     }

     pub struct CleanRequest {
         pub target_ids: Vec<String>,
         pub dry_run: bool,
     }

     pub struct StorageCleanSummary {
         pub dry_run: bool,
         pub bytes_freed: u64,
         pub files_deleted: usize,
         pub files_skipped_locked: usize,
         pub targets_processed: Vec<String>,
         pub errors: Vec<String>,
     }
     ```
   - Lines 120–121: IPC Contract:
     - `scan_cleanable_storage() -> Promise<StorageScanReport>`
     - `clean_storage_targets(request: CleanRequest) -> Promise<StorageCleanSummary>`

3. **Core Engine Readiness (`crates/pc-doctor-core/src/storage/`)**:
   - `crates/pc-doctor-core/src/storage/mod.rs` (lines 16–86): Implements `StorageEngine` with:
     - `StorageEngine::new()`: Instantiates `DefaultWindowsPathResolver` and `SafetyGuard`.
     - `StorageEngine::with_resolver(resolver: Arc<dyn StoragePathResolver>)`: Allows injecting hermetic mock resolvers for testing.
     - `StorageEngine::scan(&self) -> StorageScanReport`: Iterates targets, executes recursive scan with symlink avoidance, and computes totals.
     - `StorageEngine::clean(&self, request: &CleanRequest) -> StorageCleanSummary`: Executes dry-run simulation or live deletion with locked file skip.
   - `crates/pc-doctor-core/src/storage/models.rs` & `src/models.rs`: Defines and serializes `CleanableTarget`, `StorageScanReport`, `CleanRequest`, and `StorageCleanSummary` with `serde::{Serialize, Deserialize}`.
   - All 63 workspace tests pass (`cargo test --workspace`), including 20 adversarial storage tests in `tests/adversarial_storage_tests.rs`.

4. **Desktop Tauri Layer State (`apps/desktop/src-tauri/`)**:
   - `apps/desktop/src-tauri/Cargo.toml`:
     - Line 22: `tauri = { version = "2", features = [] }`
     - Line 28: `tokio = { version = "1", features = ["full"] }`
     - Line 31: `pc-doctor-core = { path = "../../../crates/pc-doctor-core" }`
     - Currently lacks `tempfile = "3"` in `[dev-dependencies]`.
   - `apps/desktop/src-tauri/src/commands/`:
     - Contains only `mod.rs` (lines 1–141) with `get_system_snapshot`, `get_process_list`, and `query_assistant`.
     - No storage commands or `storage.rs` module exists yet.
   - `apps/desktop/src-tauri/src/lib.rs`:
     - Lines 27–31: `invoke_handler` only registers the three existing commands.
     - Line 13–26: Starts an Axum HTTP local pairing server (`start_pairing_server`) on the Tokio runtime.
   - Unit tests in `apps/desktop/src-tauri`: Currently 0 tests run for `desktop_lib` during `cargo test`.

---

## 2. Logic Chain

### 2.1 Async vs. Sync Execution Design
- **Observation**: `StorageEngine::scan()` and `StorageEngine::clean()` perform synchronous, blocking filesystem I/O (`fs::read_dir`, `fs::symlink_metadata`, `fs::remove_file`).
- **Tauri 2 Runtime Constraint**: Tauri commands marked `async fn` execute on the shared Tokio async worker threads. If blocking I/O is run directly inside an `async fn` without offloading, it blocks the Tokio worker thread for hundreds of milliseconds to several seconds when scanning or purging large directories (e.g. 50,000+ files in `%TEMP%`).
- **Coexistence with Background Services**: In TBK Core, `apps/desktop/src-tauri/src/server.rs` runs an Axum HTTP pairing server (`start_pairing_server` on port 45120) on the Tokio runtime. Blocking async workers would cause latency spikes or dropped requests for companion ping and telemetry endpoints.
- **Deduction**: Storage commands must be defined as `async fn` and internally execute their blocking filesystem operations within `tokio::task::spawn_blocking(move || { ... })`.
- **Benefits**:
  1. Offloads blocking disk operations to Tokio's dedicated thread pool (up to 512 threads).
  2. Keeps the Tokio event loop and Axum server immediately responsive.
  3. Catches panics within the spawned thread as a `JoinError`, preventing unhandled crashes from taking down the desktop application.

### 2.2 Error Serialization & IPC Contract
- **Observation**: The contract specifies:
  - `scan_cleanable_storage() -> Result<StorageScanReport, String>`
  - `clean_storage_targets(request: CleanRequest) -> Result<StorageCleanSummary, String>`
- **Error Mapping Strategy**:
  1. **Background Task Failures**: If `tokio::task::spawn_blocking` encounters a task cancellation or panic, the `JoinError` is captured:
     ```rust
     .map_err(|e| format!("Storage scan background task failed: {}", e))?
     ```
  2. **Domain Operations**: `StorageEngine::scan` handles missing directories or permission errors gracefully without panicking (marking `total_bytes: 0, file_count: 0`).
  3. **Partial Clean Errors & Locked Files**: For `clean_storage_targets`, partial file failures (e.g., in-use browser cache files, sharing violations) are NOT treated as top-level IPC rejections. They are captured in `StorageCleanSummary.files_skipped_locked` and `StorageCleanSummary.errors`. This adheres to requirement R2 ("safely skip locked or in-use files without crashing or panicking, returning a summary of freed bytes").
  4. Top-level `Err(String)` is reserved for operational or fatal execution failures, producing a rejected Promise on the frontend.

### 2.3 Modularity & Decoupling for Testability
- **Observation**: Testing Tauri commands that accept `tauri::State` or `tauri::AppHandle` typically requires heavy Tauri test harnesses or mocks.
- **Architectural Solution**:
  1. Implement storage commands in a dedicated file `apps/desktop/src-tauri/src/commands/storage.rs`.
  2. Export pure execution helper functions:
     - `pub fn scan_cleanable_storage_impl(engine: &StorageEngine) -> Result<StorageScanReport, String>`
     - `pub fn clean_storage_targets_impl(engine: &StorageEngine, request: &CleanRequest) -> Result<StorageCleanSummary, String>`
  3. The `#[tauri::command]` functions delegate to these helpers inside `spawn_blocking`.
  4. Because the Tauri commands take no Tauri-specific parameters (`AppHandle` or `State`), they can be called directly in `#[tokio::test]` functions in `apps/desktop/src-tauri`.
  5. The `_impl` functions allow 100% hermetic unit testing using `StorageEngine::with_resolver` and `tempfile::tempdir()`, validating exact byte counting, dry-run safety, live purges, root directory preservation, and locked-file skipping without modifying host user files.

---

## 3. Caveats

1. **Host Environment Scans**: Unit tests that call `scan_cleanable_storage().await` directly test the real host filesystem (`%TEMP%`, etc.). While safe (scan is read-only), the number of files and byte counts vary across machines. Host-level tests must assert invariant properties (`is_safe == true`, non-negative byte count, valid targets) rather than hardcoded byte counts. Exact byte counts must be tested via mock environments.
2. **Elevated vs. Standard User Permissions**: Certain Windows cache locations (such as system-wide temp under `C:\Windows\Temp`) require administrator privileges. TBK Core explicitly restricts cleanable targets to user-level safe directories (`%TEMP%`, user crash dumps, WER, Explorer thumbnails), which are accessible under standard user tokens without requiring UAC elevation prompts.
3. **Frontend Argument Deserialization**: In Tauri 2, command parameters like `clean_storage_targets(request: CleanRequest)` expect the frontend invoke argument object to be keyed by parameter name: `invoke('clean_storage_targets', { request: { target_ids: [...], dry_run: ... } })`. The TypeScript wrapper in `apps/desktop/src/api/ipc.ts` must pass `{ request }`.

---

## 4. Conclusion & Actionable Blueprint

### 4.1 Summary of Changes
1. **Create `apps/desktop/src-tauri/src/commands/storage.rs`**:
   - Implements `scan_cleanable_storage` and `clean_storage_targets` with `spawn_blocking`.
   - Implements `scan_cleanable_storage_impl` and `clean_storage_targets_impl`.
   - Includes 8 comprehensive unit tests covering real system calls, mock environments, dry-run preservation, live purges, locked file skipping, unknown target handling, and JSON serialization.
2. **Update `apps/desktop/src-tauri/src/commands/mod.rs`**:
   - Add `pub mod storage;`
   - Re-export `pub use storage::{clean_storage_targets, scan_cleanable_storage};`
3. **Update `apps/desktop/src-tauri/src/lib.rs`**:
   - Import `clean_storage_targets` and `scan_cleanable_storage`.
   - Register both commands in `tauri::generate_handler![...]`.
4. **Update `apps/desktop/src-tauri/Cargo.toml`**:
   - Add `tempfile = "3"` to `[dev-dependencies]`.

### 4.2 Exact File Content: `apps/desktop/src-tauri/src/commands/storage.rs`
The complete implementation has been staged to `proposed_storage.rs` in this directory:
`d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc/proposed_storage.rs`

```rust
//! Storage management commands for PC Doctor (TBK Core).
//!
//! Provides Tauri IPC commands for scanning cleanable temporary targets
//! and safely purging temporary directories with dry-run support, locked-file
//! handling, and boundary protection.

use pc_doctor_core::storage::{
    CleanRequest, StorageCleanSummary, StorageEngine, StorageScanReport,
};

/// Scans cleanable temporary directories on the system.
///
/// Executes the filesystem scan on a Tokio blocking worker thread (`spawn_blocking`)
/// to prevent stalling the async runtime and desktop UI event loop.
#[tauri::command]
pub async fn scan_cleanable_storage() -> Result<StorageScanReport, String> {
    tokio::task::spawn_blocking(|| {
        let engine = StorageEngine::new();
        scan_cleanable_storage_impl(&engine)
    })
    .await
    .map_err(|e| format!("Storage scan background task failed: {}", e))?
}

/// Cleans or simulates cleaning for specified storage targets.
///
/// In dry-run mode (`request.dry_run == true`), calculates freed space without
/// modifying disk files. In live mode, purges files while safely skipping
/// locked/in-use files and preserving root directory boundaries.
#[tauri::command]
pub async fn clean_storage_targets(
    request: CleanRequest,
) -> Result<StorageCleanSummary, String> {
    tokio::task::spawn_blocking(move || {
        let engine = StorageEngine::new();
        clean_storage_targets_impl(&engine, &request)
    })
    .await
    .map_err(|e| format!("Storage clean background task failed: {}", e))?
}

/// Core synchronous scan logic decoupling execution from Tauri runtime for testing.
pub fn scan_cleanable_storage_impl(engine: &StorageEngine) -> Result<StorageScanReport, String> {
    Ok(engine.scan())
}

/// Core synchronous clean logic decoupling execution from Tauri runtime for testing.
pub fn clean_storage_targets_impl(
    engine: &StorageEngine,
    request: &CleanRequest,
) -> Result<StorageCleanSummary, String> {
    Ok(engine.clean(request))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_doctor_core::storage::{DefaultWindowsPathResolver, StoragePathResolver};
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Arc;

    struct MockResolver {
        temp_dir: PathBuf,
        forbidden_dir: PathBuf,
    }

    impl StoragePathResolver for MockResolver {
        fn resolve_target_root(&self, target_id: &str) -> Option<PathBuf> {
            match target_id {
                "user_temp" => Some(self.temp_dir.join("Temp")),
                "crash_dumps" => Some(self.temp_dir.join("CrashDumps")),
                "windows_error_reporting" => Some(self.temp_dir.join("WER")),
                "explorer_thumbnails" => Some(self.temp_dir.join("Explorer")),
                "application_logs" => Some(self.temp_dir.join("Logs")),
                _ => None,
            }
        }

        fn get_safe_roots(&self) -> Vec<PathBuf> {
            vec![
                self.temp_dir.join("Temp"),
                self.temp_dir.join("CrashDumps"),
                self.temp_dir.join("WER"),
                self.temp_dir.join("Explorer"),
                self.temp_dir.join("Logs"),
            ]
        }

        fn get_forbidden_roots(&self) -> Vec<PathBuf> {
            vec![
                self.forbidden_dir.clone(),
                self.forbidden_dir.join("System32"),
                self.temp_dir.join("Desktop"),
                self.temp_dir.join("Documents"),
            ]
        }
    }

    fn setup_mock_env() -> (tempfile::TempDir, Arc<MockResolver>, StorageEngine) {
        let temp_workspace = tempfile::tempdir().expect("Failed to create tempdir");
        let base = temp_workspace.path().to_path_buf();
        let forbidden = base.join("Windows");
        fs::create_dir_all(&forbidden).unwrap();
        fs::create_dir_all(base.join("Temp")).unwrap();
        fs::create_dir_all(base.join("CrashDumps")).unwrap();
        fs::create_dir_all(base.join("WER")).unwrap();
        fs::create_dir_all(base.join("Explorer")).unwrap();
        fs::create_dir_all(base.join("Logs")).unwrap();

        let resolver = Arc::new(MockResolver {
            temp_dir: base,
            forbidden_dir: forbidden,
        });
        let engine = StorageEngine::with_resolver(resolver.clone());
        (temp_workspace, resolver, engine)
    }

    #[tokio::test]
    async fn test_scan_cleanable_storage_real_system() {
        let result = scan_cleanable_storage().await;
        assert!(result.is_ok(), "scan_cleanable_storage should succeed on host");
        let report = result.unwrap();
        assert!(!report.targets.is_empty(), "Should return catalog targets");
        for target in &report.targets {
            assert!(target.is_safe, "All targets must pass safety guard");
            assert!(!target.id.is_empty(), "Target id must not be empty");
        }
    }

    #[tokio::test]
    async fn test_clean_storage_targets_dry_run_real_system() {
        let request = CleanRequest {
            target_ids: vec!["user_temp".to_string()],
            dry_run: true,
        };
        let result = clean_storage_targets(request).await;
        assert!(result.is_ok(), "Dry-run clean should succeed");
        let summary = result.unwrap();
        assert!(summary.dry_run, "Summary dry_run flag must be true");
        assert!(summary.targets_processed.contains(&"user_temp".to_string()));
        assert_eq!(summary.files_skipped_locked, 0);
    }

    #[tokio::test]
    async fn test_clean_storage_targets_empty_request() {
        let request = CleanRequest {
            target_ids: vec![],
            dry_run: true,
        };
        let result = clean_storage_targets(request).await;
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.bytes_freed, 0);
        assert_eq!(summary.files_deleted, 0);
        assert!(summary.targets_processed.is_empty());
    }

    #[tokio::test]
    async fn test_clean_storage_targets_unknown_target() {
        let request = CleanRequest {
            target_ids: vec!["unknown_target_xyz_123".to_string()],
            dry_run: true,
        };
        let result = clean_storage_targets(request).await;
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.errors.len(), 1);
        assert!(summary.errors[0].contains("unknown_target_xyz_123"));
    }

    #[test]
    fn test_mock_scan_computes_exact_bytes() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");

        let mut f1 = File::create(temp_folder.join("test1.tmp")).unwrap();
        f1.write_all(&vec![0u8; 400]).unwrap();

        let mut f2 = File::create(temp_folder.join("test2.tmp")).unwrap();
        f2.write_all(&vec![0u8; 600]).unwrap();

        let report = scan_cleanable_storage_impl(&engine).unwrap();
        let user_temp = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
        assert_eq!(user_temp.file_count, 2);
        assert_eq!(user_temp.total_bytes, 1000);
    }

    #[test]
    fn test_mock_dry_run_preserves_disk_files() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");
        let test_file = temp_folder.join("preserve.tmp");
        let mut f = File::create(&test_file).unwrap();
        f.write_all(&vec![0u8; 500]).unwrap();

        let request = CleanRequest {
            target_ids: vec!["user_temp".to_string()],
            dry_run: true,
        };
        let summary = clean_storage_targets_impl(&engine, &request).unwrap();
        assert!(summary.dry_run);
        assert_eq!(summary.files_deleted, 1);
        assert_eq!(summary.bytes_freed, 500);
        assert!(test_file.exists(), "File must NOT be deleted in dry run");
    }

    #[test]
    fn test_mock_live_clean_deletes_files_and_preserves_root() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");
        let test_file = temp_folder.join("delete_me.tmp");
        let mut f = File::create(&test_file).unwrap();
        f.write_all(&vec![0u8; 500]).unwrap();

        let request = CleanRequest {
            target_ids: vec!["user_temp".to_string()],
            dry_run: false,
        };
        let summary = clean_storage_targets_impl(&engine, &request).unwrap();
        assert!(!summary.dry_run);
        assert_eq!(summary.files_deleted, 1);
        assert_eq!(summary.bytes_freed, 500);
        assert!(!test_file.exists(), "File must be deleted in live mode");
        assert!(temp_folder.exists(), "Target root directory must be preserved");
    }

    #[test]
    fn test_mock_locked_file_is_skipped() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");
        let locked_file = temp_folder.join("locked.tmp");
        let normal_file = temp_folder.join("normal.tmp");

        {
            let mut f1 = File::create(&locked_file).unwrap();
            f1.write_all(&vec![0u8; 300]).unwrap();
            let mut f2 = File::create(&normal_file).unwrap();
            f2.write_all(&vec![0u8; 700]).unwrap();
        }

        #[cfg(windows)]
        let _held_lock = {
            use std::os::windows::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&locked_file)
                .unwrap()
        };

        #[cfg(not(windows))]
        let _held_lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&locked_file)
            .unwrap();

        let request = CleanRequest {
            target_ids: vec!["user_temp".to_string()],
            dry_run: false,
        };
        let summary = clean_storage_targets_impl(&engine, &request).unwrap();
        assert_eq!(summary.files_deleted, 1);
        assert_eq!(summary.bytes_freed, 700);
        assert_eq!(summary.files_skipped_locked, 1);
        assert!(!normal_file.exists());
        assert!(locked_file.exists());
    }

    #[test]
    fn test_ipc_json_serialization_contracts() {
        let request = CleanRequest {
            target_ids: vec!["user_temp".to_string(), "crash_dumps".to_string()],
            dry_run: true,
        };
        let req_json = serde_json::to_string(&request).unwrap();
        assert!(req_json.contains("\"target_ids\":[\"user_temp\",\"crash_dumps\"]"));
        assert!(req_json.contains("\"dry_run\":true"));

        let summary = StorageCleanSummary {
            dry_run: false,
            bytes_freed: 1048576,
            files_deleted: 42,
            files_skipped_locked: 3,
            targets_processed: vec!["user_temp".to_string()],
            errors: vec![],
        };
        let sum_json = serde_json::to_string(&summary).unwrap();
        assert!(sum_json.contains("\"bytes_freed\":1048576"));
        assert!(sum_json.contains("\"files_deleted\":42"));
        assert!(sum_json.contains("\"files_skipped_locked\":3"));
        assert!(sum_json.contains("\"targets_processed\":[\"user_temp\"]"));
        assert!(sum_json.contains("\"errors\":[]"));
    }
}
```

### 4.3 Proposed Diffs for Existing Files

#### Diff 1: `apps/desktop/src-tauri/Cargo.toml`
Staged at `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc/desktop_cargo.patch`:
```diff
--- a/apps/desktop/src-tauri/Cargo.toml
+++ b/apps/desktop/src-tauri/Cargo.toml
@@ -32,3 +32,6 @@ tower-http = { version = "0.6", features = ["cors"] }
 pc-doctor-core = { path = "../../../crates/pc-doctor-core" }
 
+[dev-dependencies]
+tempfile = "3"
+
```

#### Diff 2: `apps/desktop/src-tauri/src/commands/mod.rs`
Staged at `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc/commands_mod.patch`:
```diff
--- a/apps/desktop/src-tauri/src/commands/mod.rs
+++ b/apps/desktop/src-tauri/src/commands/mod.rs
@@ -1,4 +1,7 @@
+pub mod storage;
+
+pub use storage::{clean_storage_targets, scan_cleanable_storage};
+
 use crate::monitoring::CollectorState;
 use pc_doctor_core::knowledge::KnowledgeBase;
 use pc_doctor_core::models::{ProcessInfo, SystemSnapshot};
```

#### Diff 3: `apps/desktop/src-tauri/src/lib.rs`
Staged at `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc/src_lib.patch`:
```diff
--- a/apps/desktop/src-tauri/src/lib.rs
+++ b/apps/desktop/src-tauri/src/lib.rs
@@ -5,3 +5,4 @@
-use commands::{get_process_list, get_system_snapshot, query_assistant};
+use commands::{
+    clean_storage_targets, get_process_list, get_system_snapshot, query_assistant,
+    scan_cleanable_storage,
+};
@@ -28,3 +29,5 @@
             get_system_snapshot,
             get_process_list,
-            query_assistant
+            query_assistant,
+            scan_cleanable_storage,
+            clean_storage_targets
```

#### Companion: Frontend TypeScript Interface & Wrapper
Staged at `d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m2_storage_ipc/proposed_frontend_ipc.ts`:
```typescript
import { invoke } from '@tauri-apps/api/core';

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

export async function scanCleanableStorage(): Promise<StorageScanReport> {
  return await invoke<StorageScanReport>('scan_cleanable_storage');
}

export async function cleanStorageTargets(request: CleanRequest): Promise<StorageCleanSummary> {
  return await invoke<StorageCleanSummary>('clean_storage_targets', { request });
}
```

---

## 5. Verification Method

Once the implementer applies the patches:

1. **Compilation Check**:
   ```powershell
   cargo check --workspace
   ```
   *Expected Result*: Zero warnings or errors in both `crates/pc-doctor-core` and `apps/desktop/src-tauri`.

2. **Unit Test Execution**:
   ```powershell
   cargo test -p desktop_lib -- --nocapture
   ```
   *Expected Result*: All 8 storage unit tests in `desktop_lib::commands::storage::tests` pass:
   - `test_scan_cleanable_storage_real_system`: `ok`
   - `test_clean_storage_targets_dry_run_real_system`: `ok`
   - `test_clean_storage_targets_empty_request`: `ok`
   - `test_clean_storage_targets_unknown_target`: `ok`
   - `test_mock_scan_computes_exact_bytes`: `ok`
   - `test_mock_dry_run_preserves_disk_files`: `ok`
   - `test_mock_live_clean_deletes_files_and_preserves_root`: `ok`
   - `test_mock_locked_file_is_skipped`: `ok`
   - `test_ipc_json_serialization_contracts`: `ok`

3. **Workspace Full Regression Test**:
   ```powershell
   cargo test --workspace
   ```
   *Expected Result*: Zero failures across all packages (`desktop_lib`, `pc_doctor_core`, integration tests).

4. **Frontend TypeScript Typecheck (when integrated)**:
   ```powershell
   cd apps/desktop
   npm run build
   ```
   *Expected Result*: Zero TypeScript compilation errors.
