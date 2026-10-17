//! Adversarial Test Suite for Storage Engine in pc-doctor-core
//! Stress-testing boundary security, dry-run guarantees, locked file handling, and edge cases.

use pc_doctor_core::storage::{
    CleanRequest, CleanableTarget, DefaultWindowsPathResolver, SafetyGuard,
    SafetyViolation, StorageEngine, StoragePathResolver, StoragePurger,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct MockResolver {
    temp_dir: PathBuf,
    forbidden_windows: PathBuf,
    forbidden_desktop: PathBuf,
    forbidden_documents: PathBuf,
    forbidden_program_files: PathBuf,
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
            self.forbidden_windows.clone(),
            self.forbidden_windows.join("System32"),
            self.forbidden_windows.join("SysWOW64"),
            self.forbidden_desktop.clone(),
            self.forbidden_documents.clone(),
            self.forbidden_program_files.clone(),
        ]
    }
}

fn setup_adversarial_env() -> (tempfile::TempDir, Arc<MockResolver>, StorageEngine) {
    let temp_workspace = tempfile::tempdir().expect("Failed to create tempdir");
    let base = temp_workspace.path().to_path_buf();

    let win = base.join("Windows");
    let win32 = win.join("System32");
    let win_wow = win.join("SysWOW64");
    let desktop = base.join("Desktop");
    let documents = base.join("Documents");
    let prog_files = base.join("Program Files");

    fs::create_dir_all(&win32).unwrap();
    fs::create_dir_all(&win_wow).unwrap();
    fs::create_dir_all(&desktop).unwrap();
    fs::create_dir_all(&documents).unwrap();
    fs::create_dir_all(&prog_files).unwrap();

    fs::create_dir_all(base.join("Temp")).unwrap();
    fs::create_dir_all(base.join("CrashDumps")).unwrap();
    fs::create_dir_all(base.join("WER")).unwrap();
    fs::create_dir_all(base.join("Explorer")).unwrap();
    fs::create_dir_all(base.join("Logs")).unwrap();

    let resolver = Arc::new(MockResolver {
        temp_dir: base,
        forbidden_windows: win,
        forbidden_desktop: desktop,
        forbidden_documents: documents,
        forbidden_program_files: prog_files,
    });
    let engine = StorageEngine::with_resolver(resolver.clone());
    (temp_workspace, resolver, engine)
}

// =========================================================================
// Category 1: Boundary Security & Path Traversal Attempts
// =========================================================================

#[test]
fn test_adv_boundary_path_traversal_to_system_folders() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    // 1. Path traversal from inside Temp escaping into Windows/System32
    let traversal_to_win = resolver.temp_dir.join("Temp").join("..").join("Windows");
    let res = guard.validate(&traversal_to_win);
    assert!(
        matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })),
        "Path traversal into Windows must be rejected: {:?}",
        res
    );

    let traversal_to_sys32 = resolver
        .temp_dir
        .join("Temp")
        .join("..")
        .join("Windows")
        .join("System32");
    let res = guard.validate(&traversal_to_sys32);
    assert!(
        matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })),
        "Path traversal into System32 must be rejected: {:?}",
        res
    );

    // 2. Traversal into Desktop
    let traversal_to_desktop = resolver.temp_dir.join("Temp").join("..").join("Desktop");
    let res = guard.validate(&traversal_to_desktop);
    assert!(
        matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })),
        "Path traversal into Desktop must be rejected: {:?}",
        res
    );

    // 3. Traversal into Documents
    let traversal_to_docs = resolver.temp_dir.join("Temp").join("..").join("Documents");
    let res = guard.validate(&traversal_to_docs);
    assert!(
        matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })),
        "Path traversal into Documents must be rejected: {:?}",
        res
    );
}

#[test]
fn test_adv_boundary_drive_roots() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    // Windows drive roots
    assert!(matches!(
        guard.validate(Path::new("C:\\")),
        Err(SafetyViolation::DriveRootProhibited(_))
    ));
    assert!(matches!(
        guard.validate(Path::new("C:/")),
        Err(SafetyViolation::DriveRootProhibited(_))
    ));
    assert!(matches!(
        guard.validate(Path::new("D:\\")),
        Err(SafetyViolation::DriveRootProhibited(_))
    ));
    assert!(matches!(
        guard.validate(Path::new("\\")),
        Err(SafetyViolation::DriveRootProhibited(_))
    ));
    assert!(matches!(
        guard.validate(Path::new("/")),
        Err(SafetyViolation::DriveRootProhibited(_))
    ));
}

#[test]
fn test_adv_boundary_forbidden_directories_and_parents() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    // Exact forbidden matches
    assert!(matches!(
        guard.validate(&resolver.forbidden_windows),
        Err(SafetyViolation::ForbiddenPathMatched { .. })
    ));
    assert!(matches!(
        guard.validate(&resolver.forbidden_desktop),
        Err(SafetyViolation::ForbiddenPathMatched { .. })
    ));
    assert!(matches!(
        guard.validate(&resolver.forbidden_documents),
        Err(SafetyViolation::ForbiddenPathMatched { .. })
    ));
    assert!(matches!(
        guard.validate(&resolver.forbidden_program_files),
        Err(SafetyViolation::ForbiddenPathMatched { .. })
    ));

    // Child of forbidden directory
    let sys32_file = resolver.forbidden_windows.join("System32").join("cmd.exe");
    assert!(matches!(
        guard.validate(&sys32_file),
        Err(SafetyViolation::ForbiddenPathMatched { .. })
    ));

    let desktop_file = resolver.forbidden_desktop.join("secret_wallet.txt");
    assert!(matches!(
        guard.validate(&desktop_file),
        Err(SafetyViolation::ForbiddenPathMatched { .. })
    ));

    // Case insensitivity check on Windows forbidden paths
    let win_lower = PathBuf::from(resolver.forbidden_windows.to_string_lossy().to_lowercase());
    assert!(matches!(
        guard.validate(&win_lower),
        Err(SafetyViolation::ForbiddenPathMatched { .. })
    ));
}

#[test]
fn test_adv_boundary_validate_purge_child() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let temp_root = resolver.temp_dir.join("Temp");

    // 1. Child equals target_root itself -> prohibited! (Must never delete root)
    let res = guard.validate_purge_child(&temp_root, &temp_root);
    assert!(
        matches!(res, Err(SafetyViolation::DriveRootProhibited(_))),
        "Purging target root itself must be prohibited: {:?}",
        res
    );

    // 2. Child escapes outside target_root via ..
    let outside_child = temp_root.join("..").join("secret.txt");
    let res = guard.validate_purge_child(&temp_root, &outside_child);
    assert!(
        res.is_err(),
        "Purging item outside target_root must be rejected: {:?}",
        res
    );

    // 3. Sibling folder attempt
    let sibling = resolver.temp_dir.join("CrashDumps").join("dump.dmp");
    let res = guard.validate_purge_child(&temp_root, &sibling);
    assert!(
        matches!(res, Err(SafetyViolation::ParentDirectoryTraversalDetected(_))),
        "Purging sibling target file with wrong target_root must fail traversal check: {:?}",
        res
    );

    // 4. Legitimate child item inside target_root
    let valid_child = temp_root.join("subfolder").join("file.tmp");
    let res = guard.validate_purge_child(&temp_root, &valid_child);
    assert!(res.is_ok(), "Valid child must pass: {:?}", res);
}

#[test]
fn test_adv_production_windows_resolver_safety_guard() {
    let resolver = DefaultWindowsPathResolver::default();
    let safe_roots = resolver.get_safe_roots();
    let forbidden_roots = resolver.get_forbidden_roots();
    let guard = SafetyGuard::new(safe_roots, forbidden_roots);

    // Ensure Production resolver strictly protects Windows, System32, Desktop
    if let Ok(windir) = std::env::var("WINDIR").or_else(|_| std::env::var("SYSTEMROOT")) {
        let win_path = PathBuf::from(windir);
        assert!(guard.validate(&win_path).is_err());
        assert!(guard.validate(&win_path.join("System32")).is_err());
        assert!(guard.validate(&win_path.join("System32").join("drivers")).is_err());
    }

    if let Ok(profile) = std::env::var("USERPROFILE") {
        let prof_path = PathBuf::from(profile);
        assert!(guard.validate(&prof_path.join("Desktop")).is_err());
        assert!(guard.validate(&prof_path.join("Documents")).is_err());
    }

    // Drive root C:\
    assert!(guard.validate(Path::new("C:\\")).is_err());
}

// =========================================================================
// Category 2: Dry-Run Mode Zero-Deletions Guarantee
// =========================================================================

#[test]
fn test_adv_dry_run_zero_deletions_rigorous() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_root = resolver.temp_dir.join("Temp");

    // Populate a complex directory tree:
    // Temp/
    //   file1.tmp (1024 bytes)
    //   readonly.tmp (2048 bytes) [read-only flag set]
    //   nested1/
    //     sub1.tmp (512 bytes)
    //     nested2/
    //       sub2.tmp (256 bytes)
    //   empty_folder/

    let f1_path = temp_root.join("file1.tmp");
    let mut f1 = File::create(&f1_path).unwrap();
    f1.write_all(&vec![0xAAu8; 1024]).unwrap();

    let ro_path = temp_root.join("readonly.tmp");
    let mut ro = File::create(&ro_path).unwrap();
    ro.write_all(&vec![0xBBu8; 2048]).unwrap();
    let mut perms = fs::metadata(&ro_path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&ro_path, perms).unwrap();

    let nested2 = temp_root.join("nested1").join("nested2");
    fs::create_dir_all(&nested2).unwrap();

    let sub1_path = temp_root.join("nested1").join("sub1.tmp");
    let mut sub1 = File::create(&sub1_path).unwrap();
    sub1.write_all(&vec![0xCCu8; 512]).unwrap();

    let sub2_path = nested2.join("sub2.tmp");
    let mut sub2 = File::create(&sub2_path).unwrap();
    sub2.write_all(&vec![0xDDu8; 256]).unwrap();

    let empty_dir = temp_root.join("empty_folder");
    fs::create_dir_all(&empty_dir).unwrap();

    let total_expected_bytes = 1024 + 2048 + 512 + 256;
    let total_expected_files = 4;

    // Execute Dry-Run
    let req = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: true,
    };

    let summary = engine.clean(&req);

    // Verify summary values
    assert!(summary.dry_run, "Summary dry_run flag must be true");
    assert_eq!(summary.files_deleted, total_expected_files);
    assert_eq!(summary.bytes_freed, total_expected_bytes);
    assert_eq!(summary.files_skipped_locked, 0);
    assert!(summary.errors.is_empty(), "Errors: {:?}", summary.errors);

    // CRITICAL EMPIRICAL PROOF: ZERO DELETIONS ON DISK
    assert!(f1_path.exists(), "f1_path must still exist on disk!");
    assert!(ro_path.exists(), "ro_path must still exist on disk!");
    assert!(sub1_path.exists(), "sub1_path must still exist on disk!");
    assert!(sub2_path.exists(), "sub2_path must still exist on disk!");
    assert!(empty_dir.exists(), "empty_dir must still exist on disk!");
    assert!(nested2.exists(), "nested2 must still exist on disk!");
    assert!(temp_root.exists(), "temp_root must still exist on disk!");

    // Verify content integrity
    let f1_content = fs::read(&f1_path).unwrap();
    assert_eq!(f1_content.len(), 1024);
    assert_eq!(f1_content[0], 0xAA);

    let ro_content = fs::read(&ro_path).unwrap();
    assert_eq!(ro_content.len(), 2048);
    assert_eq!(ro_content[0], 0xBB);

    // Clean up read-only flag so tempfile cleanup can succeed when test completes
    let mut clean_perms = fs::metadata(&ro_path).unwrap().permissions();
    clean_perms.set_readonly(false);
    let _ = fs::set_permissions(&ro_path, clean_perms);
}

// =========================================================================
// Category 3: Locked File Handling & OS Error 32 / Sharing Violations
// =========================================================================

#[test]
fn test_adv_locked_files_and_sharing_violations() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_root = resolver.temp_dir.join("Temp");

    // Create 3 files:
    // 1. `normal_1.tmp` - free to delete
    // 2. `locked.tmp` - held open exclusively (sharing violation)
    // 3. `normal_2.tmp` - free to delete

    let normal1_path = temp_root.join("normal_1.tmp");
    {
        let mut f1 = File::create(&normal1_path).unwrap();
        f1.write_all(&vec![0x11u8; 400]).unwrap();
    }

    let locked_path = temp_root.join("locked.tmp");
    {
        let mut f_locked = File::create(&locked_path).unwrap();
        f_locked.write_all(&vec![0x22u8; 800]).unwrap();
    }

    let normal2_path = temp_root.join("normal_2.tmp");
    {
        let mut f2 = File::create(&normal2_path).unwrap();
        f2.write_all(&vec![0x33u8; 600]).unwrap();
    }

    // Lock `locked.tmp` exclusively (Windows share_mode(0))
    #[cfg(windows)]
    let held_file = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&locked_path)
            .expect("Must acquire exclusive lock on test file")
    };

    #[cfg(not(windows))]
    let held_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&locked_path)
        .expect("Open file");

    // Execute Live Purge (dry_run = false)
    let req = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: false,
    };

    let summary = engine.clean(&req);

    // EMPIRICAL VERIFICATION:
    // 1. No panics occurred.
    // 2. Exactly 1 file skipped due to lock.
    assert_eq!(
        summary.files_skipped_locked, 1,
        "Exactly 1 locked file must be reported skipped"
    );
    // 3. Two normal files deleted: 400 + 600 = 1000 bytes
    assert_eq!(summary.files_deleted, 2, "Two normal files must be deleted");
    assert_eq!(summary.bytes_freed, 1000, "1000 bytes must be freed");

    // 4. On disk: normal files are gone, locked file persists
    assert!(!normal1_path.exists(), "normal_1.tmp must be deleted");
    assert!(!normal2_path.exists(), "normal_2.tmp must be deleted");
    assert!(locked_path.exists(), "locked.tmp must still exist on disk!");

    // 5. Temp root directory is preserved
    assert!(temp_root.exists(), "Temp root directory must not be deleted");

    // Drop file lock
    drop(held_file);
}

#[test]
fn test_adv_readonly_file_deletion_live_mode() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_root = resolver.temp_dir.join("Temp");

    let ro_path = temp_root.join("readonly_test.tmp");
    let mut ro = File::create(&ro_path).unwrap();
    ro.write_all(&vec![0x55u8; 500]).unwrap();

    let mut perms = fs::metadata(&ro_path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&ro_path, perms).unwrap();

    // Verify it is indeed read-only
    assert!(fs::metadata(&ro_path).unwrap().permissions().readonly());

    // Execute live purge
    let req = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: false,
    };

    let summary = engine.clean(&req);
    assert_eq!(summary.files_deleted, 1);
    assert_eq!(summary.bytes_freed, 500);
    assert!(!ro_path.exists(), "Read-only file must have permissions reset and be deleted");
}

// =========================================================================
// Category 4: Edge Cases & Target Filtering
// =========================================================================

#[test]
fn test_adv_target_filtering_explorer_thumbnails() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let explorer_root = resolver.temp_dir.join("Explorer");

    // Explorer target only cleans thumbcache_* and iconcache_*
    let thumb = explorer_root.join("thumbcache_256.db");
    File::create(&thumb).unwrap().write_all(&vec![0u8; 100]).unwrap();

    let icon = explorer_root.join("iconcache_idx.db");
    File::create(&icon).unwrap().write_all(&vec![0u8; 200]).unwrap();

    let sensitive_other = explorer_root.join("other_document.docx");
    File::create(&sensitive_other).unwrap().write_all(&vec![0u8; 500]).unwrap();

    let report = engine.scan();
    let explorer_target = report.targets.iter().find(|t| t.id == "explorer_thumbnails").unwrap();

    // Scanner should ONLY count matching cache files: 2 files, 300 bytes
    assert_eq!(explorer_target.file_count, 2);
    assert_eq!(explorer_target.total_bytes, 300);

    // Live purge
    let req = CleanRequest {
        target_ids: vec!["explorer_thumbnails".to_string()],
        dry_run: false,
    };
    let summary = engine.clean(&req);

    assert_eq!(summary.files_deleted, 2);
    assert_eq!(summary.bytes_freed, 300);

    // CRITICAL: Unmatched file must NOT be deleted!
    assert!(sensitive_other.exists(), "Unmatched file must NOT be touched");
    assert!(!thumb.exists(), "thumbcache must be deleted");
    assert!(!icon.exists(), "iconcache must be deleted");
}

#[test]
fn test_adv_target_filtering_application_logs() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let logs_root = resolver.temp_dir.join("Logs");

    let log_file = logs_root.join("service.log");
    File::create(&log_file).unwrap().write_all(&vec![0u8; 150]).unwrap();

    let etl_file = logs_root.join("trace.etl");
    File::create(&etl_file).unwrap().write_all(&vec![0u8; 250]).unwrap();

    let config_file = logs_root.join("app.config");
    File::create(&config_file).unwrap().write_all(&vec![0u8; 999]).unwrap();

    let report = engine.scan();
    let logs_target = report.targets.iter().find(|t| t.id == "application_logs").unwrap();

    // Scanner should only match .log and .etl: 2 files, 400 bytes
    assert_eq!(logs_target.file_count, 2);
    assert_eq!(logs_target.total_bytes, 400);

    // Live purge
    let req = CleanRequest {
        target_ids: vec!["application_logs".to_string()],
        dry_run: false,
    };
    let summary = engine.clean(&req);

    assert_eq!(summary.files_deleted, 2);
    assert_eq!(summary.bytes_freed, 400);

    assert!(config_file.exists(), "Non-log file must be untouched");
    assert!(!log_file.exists());
    assert!(!etl_file.exists());
}

#[test]
fn test_adv_unknown_target_id_graceful_handling() {
    let (_dir, _resolver, engine) = setup_adversarial_env();

    let req = CleanRequest {
        target_ids: vec!["evil_nonexistent_target_id".to_string()],
        dry_run: false,
    };

    let summary = engine.clean(&req);
    assert_eq!(summary.files_deleted, 0);
    assert_eq!(summary.bytes_freed, 0);
    assert_eq!(summary.errors.len(), 1);
    assert!(summary.errors[0].contains("unknown"));
}

#[test]
fn test_adv_target_with_boundary_violation_in_root() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    // Malicious target crafted pointing to Windows directory
    let malicious_target = CleanableTarget {
        id: "evil_target".to_string(),
        name_en: "Evil".to_string(),
        name_vi: "Doc hai".to_string(),
        description_en: "".to_string(),
        description_vi: "".to_string(),
        root_path: resolver.forbidden_windows.to_string_lossy().to_string(),
        file_count: 5,
        total_bytes: 10000,
        is_safe: false,
    };

    let req = CleanRequest {
        target_ids: vec!["evil_target".to_string()],
        dry_run: false,
    };

    let summary = StoragePurger::purge(&req, &[malicious_target], &guard);

    assert_eq!(summary.files_deleted, 0, "No files must be deleted in forbidden target");
    assert_eq!(summary.bytes_freed, 0);
    assert_eq!(summary.errors.len(), 1);
    assert!(summary.errors[0].contains("Safety boundary violation"));
}

// =========================================================================
// Category 5: Deep Recursion, Prefix Collisions, Slashes, and Stress
// =========================================================================

#[test]
fn test_adv_prefix_collision_temp2_rejected() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    // Safe root is `Temp`. A directory named `Temp2` or `Temporary` is NOT inside `Temp`!
    let temp2 = resolver.temp_dir.join("Temp2");
    fs::create_dir_all(&temp2).unwrap();

    let res = guard.validate(&temp2);
    assert!(
        matches!(res, Err(SafetyViolation::PathOutsideSafeWhitelist { .. })),
        "Temp2 must not be accepted under Temp safe root: {:?}",
        res
    );

    let temp_child = temp2.join("sneaky.tmp");
    let res_child = guard.validate(&temp_child);
    assert!(
        matches!(res_child, Err(SafetyViolation::PathOutsideSafeWhitelist { .. })),
        "Files inside Temp2 must be rejected: {:?}",
        res_child
    );
}

#[test]
fn test_adv_deep_directory_recursion_limit() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_root = resolver.temp_dir.join("Temp");

    // Create a 36-level deep directory tree (exceeding MAX_SCAN_DEPTH = 32)
    let mut current = temp_root.clone();
    for i in 0..36 {
        current = current.join(format!("level_{}", i));
    }
    fs::create_dir_all(&current).unwrap();

    // Place a file at depth 36
    let deep_file = current.join("deep.tmp");
    File::create(&deep_file).unwrap().write_all(&vec![0u8; 100]).unwrap();

    // Place a file at depth 1 (within 32)
    let shallow_file = temp_root.join("shallow.tmp");
    File::create(&shallow_file).unwrap().write_all(&vec![0u8; 50]).unwrap();

    // Scan should complete cleanly without stack overflow
    let report = engine.scan();
    let temp_target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();

    // Shallow file is counted
    assert!(temp_target.file_count >= 1);
    assert!(temp_target.total_bytes >= 50);

    // Dry run clean should also complete cleanly without panic
    let req = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: true,
    };
    let summary = engine.clean(&req);
    assert!(summary.dry_run);
    assert!(deep_file.exists());
    assert!(shallow_file.exists());
}

#[test]
fn test_adv_dry_run_multi_targets_preservation() {
    let (_dir, resolver, engine) = setup_adversarial_env();

    // Setup files in Temp, CrashDumps, and WER
    let temp_file = resolver.temp_dir.join("Temp").join("t.tmp");
    File::create(&temp_file).unwrap().write_all(&vec![1u8; 100]).unwrap();

    let dump_file = resolver.temp_dir.join("CrashDumps").join("app.dmp");
    File::create(&dump_file).unwrap().write_all(&vec![2u8; 200]).unwrap();

    let wer_file = resolver.temp_dir.join("WER").join("report.wer");
    File::create(&wer_file).unwrap().write_all(&vec![3u8; 300]).unwrap();

    let req = CleanRequest {
        target_ids: vec![
            "user_temp".to_string(),
            "crash_dumps".to_string(),
            "windows_error_reporting".to_string(),
        ],
        dry_run: true,
    };

    let summary = engine.clean(&req);
    assert!(summary.dry_run);
    assert_eq!(summary.files_deleted, 3);
    assert_eq!(summary.bytes_freed, 600);
    assert_eq!(summary.targets_processed.len(), 3);
    assert!(summary.errors.is_empty());

    // EMPIRICAL CHECK: All files must still exist
    assert!(temp_file.exists());
    assert!(dump_file.exists());
    assert!(wer_file.exists());
}

#[test]
fn test_adv_purge_empty_subdirectories_cleaned_while_root_preserved() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_root = resolver.temp_dir.join("Temp");

    let sub = temp_root.join("sub_to_delete");
    fs::create_dir_all(&sub).unwrap();
    let sub_file = sub.join("temp.tmp");
    File::create(&sub_file).unwrap().write_all(&vec![0u8; 50]).unwrap();

    let req = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: false,
    };

    let summary = engine.clean(&req);
    assert_eq!(summary.files_deleted, 1);
    assert_eq!(summary.bytes_freed, 50);

    // sub_file deleted
    assert!(!sub_file.exists());
    // sub directory itself should be cleaned up because it became empty
    assert!(!sub.exists(), "Empty subdirectory should be removed");
    // CRITICAL: temp_root MUST remain intact
    assert!(temp_root.exists(), "Target root directory must NEVER be deleted");
}

#[test]
fn test_adv_purge_subdir_with_locked_file_not_deleted() {
    let (_dir, resolver, engine) = setup_adversarial_env();
    let temp_root = resolver.temp_dir.join("Temp");

    let sub = temp_root.join("sub_with_locked");
    fs::create_dir_all(&sub).unwrap();
    let locked_file = sub.join("locked_child.tmp");
    {
        let mut f = File::create(&locked_file).unwrap();
        f.write_all(&vec![0u8; 100]).unwrap();
    }

    #[cfg(windows)]
    let _held = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&locked_file)
            .expect("Acquire exclusive lock")
    };

    #[cfg(not(windows))]
    let _held = fs::OpenOptions::new().read(true).write(true).open(&locked_file).unwrap();

    let req = CleanRequest {
        target_ids: vec!["user_temp".to_string()],
        dry_run: false,
    };

    let summary = engine.clean(&req);
    assert_eq!(summary.files_skipped_locked, 1);
    assert!(locked_file.exists());
    // The subdirectory cannot be deleted because it still contains locked_child.tmp
    assert!(sub.exists(), "Subdirectory containing locked file must not be deleted");
    assert!(temp_root.exists(), "Temp root must not be deleted");
}

#[test]
fn test_adv_nonexistent_and_empty_root_paths() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    // 1. Target with completely non-existent path
    let non_existent_target = CleanableTarget {
        id: "missing_target".to_string(),
        name_en: "Missing".to_string(),
        name_vi: "Khong ton tai".to_string(),
        description_en: "".to_string(),
        description_vi: "".to_string(),
        root_path: "C:\\NonExistentPath_123456789".to_string(),
        file_count: 0,
        total_bytes: 0,
        is_safe: true,
    };

    // 2. Target with empty root_path
    let empty_target = CleanableTarget {
        id: "empty_target".to_string(),
        name_en: "Empty".to_string(),
        name_vi: "Rong".to_string(),
        description_en: "".to_string(),
        description_vi: "".to_string(),
        root_path: "".to_string(),
        file_count: 0,
        total_bytes: 0,
        is_safe: true,
    };

    let req = CleanRequest {
        target_ids: vec!["missing_target".to_string(), "empty_target".to_string()],
        dry_run: false,
    };

    let summary = StoragePurger::purge(&req, &[non_existent_target, empty_target], &guard);
    assert_eq!(summary.files_deleted, 0);
    assert_eq!(summary.bytes_freed, 0);
    assert_eq!(summary.files_skipped_locked, 0);
    // Neither should cause a panic
}

#[test]
fn test_adv_case_insensitivity_and_mixed_slashes() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    // Mixed slashes forbidden path
    let win_str = resolver.forbidden_windows.to_string_lossy().to_string();
    let win_forward = win_str.replace('\\', "/");
    let res = guard.validate(Path::new(&win_forward));
    assert!(
        matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })),
        "Forward slashes forbidden path must be rejected: {:?}",
        res
    );

    // Mixed case safe path
    let temp_str = resolver.temp_dir.join("Temp").join("test.tmp").to_string_lossy().to_string();
    let temp_upper = temp_str.to_uppercase();
    let res = guard.validate(Path::new(&temp_upper));
    assert!(
        res.is_ok(),
        "Uppercase valid safe path must be accepted on Windows: {:?}",
        res
    );
}

#[test]
fn test_adv_dot_traversal_variations() {
    let (_dir, resolver, _engine) = setup_adversarial_env();
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    let temp_root = resolver.temp_dir.join("Temp");

    // Current dir dots: Temp/././file.tmp
    let dot_path = temp_root.join(".").join(".").join("file.tmp");
    let res = guard.validate(&dot_path);
    assert!(res.is_ok(), "Current dir dots within safe root should be ok: {:?}", res);

    // Parent dir traversal escaping: Temp/sub/../../Windows
    let escape_path = temp_root.join("sub").join("..").join("..").join("Windows");
    let res = guard.validate(&escape_path);
    assert!(
        matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })),
        "Parent dir escape to Windows must be rejected: {:?}",
        res
    );
}
