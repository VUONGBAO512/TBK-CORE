//! Adversarial tests for Storage IPC Commands in PC Doctor (TBK Core).
//!
//! Stress-tests edge cases, boundary violations, dry-run guarantees,
//! locked file skips, directory preservation, and concurrent IPC calls.

use desktop_lib::commands::storage::{
    clean_storage_targets, clean_storage_targets_impl, scan_cleanable_storage,
    scan_cleanable_storage_impl,
};
use pc_doctor_core::storage::{
    CleanRequest, StorageEngine, StoragePathResolver,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

struct AdversarialMockResolver {
    temp_dir: PathBuf,
    forbidden_dir: PathBuf,
    nonexistent_dir: PathBuf,
    boundary_violation_dir: PathBuf,
}

impl StoragePathResolver for AdversarialMockResolver {
    fn resolve_target_root(&self, target_id: &str) -> Option<PathBuf> {
        match target_id {
            "user_temp" => Some(self.temp_dir.join("Temp")),
            "crash_dumps" => Some(self.nonexistent_dir.clone()),
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
            self.nonexistent_dir.clone(),
        ]
    }

    fn get_forbidden_roots(&self) -> Vec<PathBuf> {
        vec![
            self.forbidden_dir.clone(),
            self.boundary_violation_dir.clone(),
            self.temp_dir.join("Desktop"),
            self.temp_dir.join("Documents"),
        ]
    }
}

fn setup_adversarial_env() -> (tempfile::TempDir, Arc<AdversarialMockResolver>, StorageEngine) {
    let temp_workspace = tempfile::tempdir().expect("Failed to create tempdir");
    let base = temp_workspace.path().to_path_buf();
    let forbidden = base.join("Windows");
    let nonexistent = base.join("DoesNotActuallyExist_12345");
    let boundary_violation = forbidden.join("System32");

    fs::create_dir_all(&forbidden).unwrap();
    fs::create_dir_all(base.join("Temp")).unwrap();
    fs::create_dir_all(base.join("CrashDumps")).unwrap();
    fs::create_dir_all(base.join("WER")).unwrap();
    fs::create_dir_all(base.join("Explorer")).unwrap();
    fs::create_dir_all(base.join("Logs")).unwrap();
    fs::create_dir_all(base.join("Desktop")).unwrap();

    let resolver = Arc::new(AdversarialMockResolver {
        temp_dir: base,
        forbidden_dir: forbidden,
        nonexistent_dir: nonexistent,
        boundary_violation_dir: boundary_violation,
    });
    let engine = StorageEngine::with_resolver(resolver.clone());
    (temp_workspace, resolver, engine)
}

// -----------------------------------------------------------------------------
// 1. Host Async Execution Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_adv_host_async_scan_invariants() {
    let result = scan_cleanable_storage().await;
    assert!(result.is_ok(), "Real host scan_cleanable_storage must succeed");
    let report = result.unwrap();
    assert!(!report.targets.is_empty(), "Should return catalog targets");
    
    // Invariant: all targets safe, file count >= 0, byte sizes non-negative
    for target in &report.targets {
        assert!(target.is_safe, "Target '{}' marked unsafe on clean host", target.id);
        assert!(!target.id.is_empty());
        assert!(!target.name_en.is_empty());
        assert!(!target.name_vi.is_empty());
    }
    assert_eq!(
        report.total_files,
        report.targets.iter().map(|t| t.file_count).sum::<usize>()
    );
    assert_eq!(
        report.total_reclaimable_bytes,
        report.targets.iter().map(|t| t.total_bytes).sum::<u64>()
    );
}

#[tokio::test]
async fn test_adv_host_async_dry_run_zero_modification() {
    let request = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: true,
    };
    let result = clean_storage_targets(request).await;
    assert!(result.is_ok(), "Dry-run clean on host must succeed");
    let summary = result.unwrap();
    assert!(summary.dry_run, "Dry run flag must be true");
    assert_eq!(summary.files_skipped_locked, 0, "Dry run does not attempt file deletion locks");
}

// -----------------------------------------------------------------------------
// 2. Edge Case Tests: Empty, Unknown, Invalid, Duplicate Target IDs
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_adv_empty_target_ids_clean_request() {
    let request = CleanRequest {
        target_ids: vec![],
        dry_run: false,
    };
    let result = clean_storage_targets(request).await;
    assert!(result.is_ok(), "Empty request must not fail");
    let summary = result.unwrap();
    assert_eq!(summary.bytes_freed, 0);
    assert_eq!(summary.files_deleted, 0);
    assert_eq!(summary.files_skipped_locked, 0);
    assert!(summary.targets_processed.is_empty());
    assert!(summary.errors.is_empty());
}

#[test]
fn test_adv_unknown_and_malicious_target_ids() {
    let (_dir, _resolver, engine) = setup_adversarial_env();
    let malicious_ids = vec![
        "unknown_target_abc".to_string(),
        "".to_string(),
        "   ".to_string(),
        "../../etc/passwd".to_string(),
        "C:\\Windows\\System32".to_string(),
        "\0nullbyte".to_string(),
    ];

    let request = CleanRequest {
        target_ids: malicious_ids.clone(),
        dry_run: false,
    };
    let result = clean_storage_targets_impl(&engine, &request);
    assert!(result.is_ok(), "Unknown targets should return Ok with recorded errors");
    let summary = result.unwrap();
    assert_eq!(summary.bytes_freed, 0);
    assert_eq!(summary.files_deleted, 0);
    assert_eq!(summary.errors.len(), malicious_ids.len());
    for (i, target_id) in malicious_ids.iter().enumerate() {
        assert!(
            summary.errors[i].contains("is unknown"),
            "Error must identify unknown target id: {}",
            target_id
        );
    }
}

#[test]
fn test_adv_duplicate_target_ids_handling() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_folder = resolver.temp_dir.join("Temp");
    let test_file = temp_folder.join("single.tmp");
    let mut f = File::create(&test_file).unwrap();
    f.write_all(&vec![0u8; 1024]).unwrap();

    // Live clean with duplicate IDs
    let request = CleanRequest {
        target_ids: vec!["user_temp".to_string(), "user_temp".to_string()],
        dry_run: false,
    };
    let summary = clean_storage_targets_impl(&engine, &request).unwrap();
    // In live mode, 1st iteration purges file, 2nd iteration finds empty directory
    assert_eq!(summary.files_deleted, 1, "File should only be counted as deleted once");
    assert_eq!(summary.bytes_freed, 1024, "Bytes should only be freed once in live clean");
    assert!(!test_file.exists());
    assert!(temp_folder.exists());
}

#[test]
fn test_adv_nonexistent_directory_handling() {
    let (_dir, _resolver, engine) = setup_adversarial_env();
    // "crash_dumps" is a catalog target configured to point to nonexistent_dir
    let request = CleanRequest {
        target_ids: vec!["crash_dumps".to_string()],
        dry_run: false,
    };
    let summary = clean_storage_targets_impl(&engine, &request).unwrap();
    assert_eq!(summary.bytes_freed, 0);
    assert_eq!(summary.files_deleted, 0);
    assert_eq!(summary.files_skipped_locked, 0);
    assert!(summary.errors.is_empty(), "Missing folder should be skipped gracefully without error");
    assert!(summary.targets_processed.contains(&"crash_dumps".to_string()));
}

// -----------------------------------------------------------------------------
// 3. Dry-Run Guarantees: Oracle & Absolute Non-Mutation
// -----------------------------------------------------------------------------

#[test]
fn test_adv_dry_run_oracle_absolute_preservation() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_folder = resolver.temp_dir.join("Temp");

    // Create 30 files across nested subdirectories with specific byte sizes
    let mut expected_bytes: u64 = 0;
    let mut file_paths = Vec::new();

    for dir_idx in 0..5 {
        let sub = temp_folder.join(format!("sub_{}", dir_idx));
        fs::create_dir_all(&sub).unwrap();
        for file_idx in 0..6 {
            let file_path = sub.join(format!("file_{}.tmp", file_idx));
            let size = ((dir_idx + 1) * 100 + file_idx * 17) as usize;
            let data = vec![file_idx as u8; size];
            let mut f = File::create(&file_path).unwrap();
            f.write_all(&data).unwrap();
            expected_bytes += size as u64;
            file_paths.push((file_path, data));
        }
    }

    // Step 1: Scan target
    let scan_report = scan_cleanable_storage_impl(&engine).unwrap();
    let user_temp = scan_report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    assert_eq!(user_temp.file_count, 30);
    assert_eq!(user_temp.total_bytes, expected_bytes);

    // Step 2: Dry-run clean
    let request = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: true,
    };
    let summary = clean_storage_targets_impl(&engine, &request).unwrap();

    // Step 3: Assert summary exactly equals expected values
    assert!(summary.dry_run);
    assert_eq!(summary.files_deleted, 30);
    assert_eq!(summary.bytes_freed, expected_bytes);
    assert_eq!(summary.files_skipped_locked, 0);
    assert!(summary.errors.is_empty());

    // Step 4: Verify ALL files exist with EXACT unmutated contents
    for (path, expected_content) in file_paths {
        assert!(path.exists(), "File {:?} was modified or deleted during dry run!", path);
        let actual_content = fs::read(&path).unwrap();
        assert_eq!(
            actual_content, expected_content,
            "File content of {:?} was corrupted during dry run!", path
        );
    }
}

// -----------------------------------------------------------------------------
// 4. Live Clean: Root Preservation, Deletion, Subdir Cleanup
// -----------------------------------------------------------------------------

#[test]
fn test_adv_live_clean_deletes_files_and_cleans_subdirs_while_preserving_root() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_folder = resolver.temp_dir.join("Temp");
    let deep_nested = temp_folder.join("level1").join("level2").join("level3");
    fs::create_dir_all(&deep_nested).unwrap();

    let file1 = deep_nested.join("deep.tmp");
    let mut f1 = File::create(&file1).unwrap();
    f1.write_all(&vec![1u8; 512]).unwrap();

    let file2 = temp_folder.join("root_child.tmp");
    let mut f2 = File::create(&file2).unwrap();
    f2.write_all(&vec![2u8; 1024]).unwrap();

    let request = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: false,
    };
    let summary = clean_storage_targets_impl(&engine, &request).unwrap();

    assert!(!summary.dry_run);
    assert_eq!(summary.files_deleted, 2);
    assert_eq!(summary.bytes_freed, 1536);
    assert_eq!(summary.files_skipped_locked, 0);

    // Files must be deleted
    assert!(!file1.exists());
    assert!(!file2.exists());

    // Empty subdirectories must be pruned
    assert!(!deep_nested.exists());
    assert!(!temp_folder.join("level1").exists());

    // Root directory MUST STILL EXIST
    assert!(temp_folder.exists(), "Target root directory must NOT be deleted");
}

// -----------------------------------------------------------------------------
// 5. Locked Files & Sharing Violations
// -----------------------------------------------------------------------------

#[test]
fn test_adv_locked_files_and_open_handles() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_folder = resolver.temp_dir.join("Temp");
    let locked_sub = temp_folder.join("locked_subdir");
    fs::create_dir_all(&locked_sub).unwrap();

    let locked_file = locked_sub.join("locked_process.tmp");
    let normal_file = locked_sub.join("normal_unlocked.tmp");
    let root_normal = temp_folder.join("root_unlocked.tmp");

    {
        let mut f1 = File::create(&locked_file).unwrap();
        f1.write_all(&vec![0xAA; 1000]).unwrap();
        let mut f2 = File::create(&normal_file).unwrap();
        f2.write_all(&vec![0xBB; 2000]).unwrap();
        let mut f3 = File::create(&root_normal).unwrap();
        f3.write_all(&vec![0xCC; 3000]).unwrap();
    }

    // Lock `locked_file` with exclusive access (share_mode = 0 on Windows)
    #[cfg(windows)]
    let _exclusive_handle = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&locked_file)
            .expect("Failed to open file with exclusive share_mode(0)")
    };

    #[cfg(not(windows))]
    let _exclusive_handle = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&locked_file)
        .unwrap();

    let request = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: false,
    };
    let summary = clean_storage_targets_impl(&engine, &request).unwrap();

    // 2 files deleted (normal_file + root_normal = 2000 + 3000 = 5000 bytes)
    assert_eq!(summary.files_deleted, 2);
    assert_eq!(summary.bytes_freed, 5000);
    // 1 locked file skipped
    assert_eq!(summary.files_skipped_locked, 1);
    assert!(summary.errors.is_empty(), "Locked files must be skipped without pushing to errors");

    // Assert normal files deleted
    assert!(!normal_file.exists());
    assert!(!root_normal.exists());

    // Assert locked file still exists
    assert!(locked_file.exists(), "Locked file must survive purge");
    // Assert locked_subdir still exists because it contains the locked file
    assert!(locked_sub.exists(), "Directory with locked file must not be removed");
    // Assert target root still exists
    assert!(temp_folder.exists());
}

// -----------------------------------------------------------------------------
// 6. Target Filtering: Explorer Thumbnails & Application Logs
// -----------------------------------------------------------------------------

#[test]
fn test_adv_target_filtering_strictness() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let explorer_dir = resolver.temp_dir.join("Explorer");
    let logs_dir = resolver.temp_dir.join("Logs");

    // Explorer directory files
    let thumb_file = explorer_dir.join("thumbcache_128.db");
    let icon_file = explorer_dir.join("iconcache_idx.db");
    let personal_doc = explorer_dir.join("important_user_doc.pdf");
    let explorer_exe = explorer_dir.join("installer.exe");

    let mut f = File::create(&thumb_file).unwrap(); f.write_all(&vec![1; 100]).unwrap();
    let mut f = File::create(&icon_file).unwrap(); f.write_all(&vec![2; 200]).unwrap();
    let mut f = File::create(&personal_doc).unwrap(); f.write_all(&vec![3; 5000]).unwrap();
    let mut f = File::create(&explorer_exe).unwrap(); f.write_all(&vec![4; 10000]).unwrap();

    // Logs directory files
    let log_file = logs_dir.join("app_setup.log");
    let etl_file = logs_dir.join("trace.etl");
    let config_file = logs_dir.join("database.sqlite");

    let mut f = File::create(&log_file).unwrap(); f.write_all(&vec![5; 300]).unwrap();
    let mut f = File::create(&etl_file).unwrap(); f.write_all(&vec![6; 400]).unwrap();
    let mut f = File::create(&config_file).unwrap(); f.write_all(&vec![7; 20000]).unwrap();

    // 1. Scan check
    let scan = scan_cleanable_storage_impl(&engine).unwrap();
    let exp_target = scan.targets.iter().find(|t| t.id == "explorer_thumbnails").unwrap();
    assert_eq!(exp_target.file_count, 2, "Only thumbcache_ and iconcache_ counted");
    assert_eq!(exp_target.total_bytes, 300);

    let logs_target = scan.targets.iter().find(|t| t.id == "application_logs").unwrap();
    assert_eq!(logs_target.file_count, 2, "Only .log and .etl counted");
    assert_eq!(logs_target.total_bytes, 700);

    // 2. Live clean both targets
    let request = CleanRequest {
        target_ids: vec![
            "explorer_thumbnails".to_string(),
            "application_logs".to_string(),
        ],
        dry_run: false,
    };
    let summary = clean_storage_targets_impl(&engine, &request).unwrap();
    assert_eq!(summary.files_deleted, 4);
    assert_eq!(summary.bytes_freed, 1000);

    // Verified: filtered files deleted
    assert!(!thumb_file.exists());
    assert!(!icon_file.exists());
    assert!(!log_file.exists());
    assert!(!etl_file.exists());

    // CRITICAL: Unmatched files MUST BE UNTOUCHED
    assert!(personal_doc.exists(), "Non-thumbnail PDF was incorrectly deleted!");
    assert!(explorer_exe.exists(), "Non-thumbnail EXE was incorrectly deleted!");
    assert!(config_file.exists(), "Non-log SQLite database was incorrectly deleted!");
}

// -----------------------------------------------------------------------------
// 7. Safety Boundary Violation Interception
// -----------------------------------------------------------------------------

#[test]
fn test_adv_boundary_violation_in_clean_request_blocked() {
    let temp_workspace = tempfile::tempdir().expect("Failed to create tempdir");
    let base = temp_workspace.path().to_path_buf();
    let forbidden = base.join("Windows");
    let sys32 = forbidden.join("System32");
    fs::create_dir_all(&sys32).unwrap();

    struct BoundaryAttackResolver {
        attack_path: PathBuf,
        forbidden: PathBuf,
    }
    impl StoragePathResolver for BoundaryAttackResolver {
        fn resolve_target_root(&self, target_id: &str) -> Option<PathBuf> {
            if target_id == "user_temp" {
                Some(self.attack_path.clone())
            } else {
                None
            }
        }
        fn get_safe_roots(&self) -> Vec<PathBuf> {
            vec![]
        }
        fn get_forbidden_roots(&self) -> Vec<PathBuf> {
            vec![self.forbidden.clone()]
        }
    }

    let resolver = Arc::new(BoundaryAttackResolver {
        attack_path: sys32,
        forbidden: forbidden,
    });
    let engine = StorageEngine::with_resolver(resolver);
    let request = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: false,
    };
    let summary = clean_storage_targets_impl(&engine, &request).unwrap();
    assert_eq!(summary.bytes_freed, 0);
    assert_eq!(summary.files_deleted, 0);
    assert_eq!(summary.errors.len(), 1);
    assert!(summary.errors[0].contains("Safety boundary violation"));
}

// -----------------------------------------------------------------------------
// 8. High Concurrency IPC Stress
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_adv_concurrent_ipc_calls() {
    let mut scan_handles = Vec::new();
    let mut clean_handles = Vec::new();

    // Launch 10 concurrent async scans
    for _ in 0..10 {
        scan_handles.push(tokio::spawn(async {
            scan_cleanable_storage().await
        }));
    }

    // Launch 10 concurrent async dry-run cleans
    for _ in 0..10 {
        clean_handles.push(tokio::spawn(async {
            let req = CleanRequest {
                target_ids: vec!["user_temp".to_string()],
                dry_run: true,
            };
            clean_storage_targets(req).await
        }));
    }

    // Await all tasks
    for handle in scan_handles {
        let res = handle.await.expect("Tokio join failed");
        assert!(res.is_ok(), "Concurrent scan failed: {:?}", res);
    }
    for handle in clean_handles {
        let res = handle.await.expect("Tokio join failed");
        assert!(res.is_ok(), "Concurrent clean failed: {:?}", res);
    }
}
