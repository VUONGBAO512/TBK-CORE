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
    use pc_doctor_core::storage::StoragePathResolver;
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
