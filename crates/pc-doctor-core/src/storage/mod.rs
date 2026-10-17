pub mod models;
pub mod purge;
pub mod safety;
pub mod scanner;
pub mod targets;

pub use models::*;
pub use purge::StoragePurger;
pub use safety::{normalize_path, SafetyGuard};
pub use scanner::StorageScanner;
pub use targets::{DefaultWindowsPathResolver, StoragePathResolver, TargetCatalog};

use chrono::Utc;
use std::sync::Arc;

pub struct StorageEngine {
    resolver: Arc<dyn StoragePathResolver>,
    safety_guard: SafetyGuard,
}

impl Default for StorageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageEngine {
    pub fn new() -> Self {
        let resolver = Arc::new(DefaultWindowsPathResolver::default());
        let safe_roots = resolver.get_safe_roots();
        let forbidden_roots = resolver.get_forbidden_roots();
        let safety_guard = SafetyGuard::new(safe_roots, forbidden_roots);

        Self {
            resolver,
            safety_guard,
        }
    }

    pub fn with_resolver(resolver: Arc<dyn StoragePathResolver>) -> Self {
        let safe_roots = resolver.get_safe_roots();
        let forbidden_roots = resolver.get_forbidden_roots();
        let safety_guard = SafetyGuard::new(safe_roots, forbidden_roots);

        Self {
            resolver,
            safety_guard,
        }
    }

    pub fn resolver(&self) -> &dyn StoragePathResolver {
        self.resolver.as_ref()
    }

    pub fn safety_guard(&self) -> &SafetyGuard {
        &self.safety_guard
    }

    pub fn scan(&self) -> StorageScanReport {
        let mut targets = TargetCatalog::get_definitions(self.resolver.as_ref());
        let mut total_bytes: u64 = 0;
        let mut total_files: usize = 0;

        for target in &mut targets {
            StorageScanner::scan_target(target, &self.safety_guard);
            total_bytes = total_bytes.saturating_add(target.total_bytes);
            total_files += target.file_count;
        }

        StorageScanReport {
            targets,
            total_reclaimable_bytes: total_bytes,
            total_files,
            scanned_at: Utc::now(),
        }
    }

    pub fn clean(&self, request: &CleanRequest) -> StorageCleanSummary {
        let mut targets = TargetCatalog::get_definitions(self.resolver.as_ref());
        for target in &mut targets {
            StorageScanner::scan_target(target, &self.safety_guard);
        }

        StoragePurger::purge(request, &targets, &self.safety_guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        fs::create_dir_all(base.join("Desktop")).unwrap();
        fs::create_dir_all(base.join("Documents")).unwrap();

        let resolver = Arc::new(MockResolver {
            temp_dir: base,
            forbidden_dir: forbidden,
        });
        let engine = StorageEngine::with_resolver(resolver.clone());
        (temp_workspace, resolver, engine)
    }

    #[test]
    fn test_safety_guard_rejects_forbidden_system_folders() {
        let (_dir, resolver, _engine) = setup_mock_env();
        let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

        // Reject Windows directory
        let res_win = guard.validate(&resolver.forbidden_dir);
        assert!(matches!(res_win, Err(SafetyViolation::ForbiddenPathMatched { .. })));

        // Reject Desktop
        let res_desktop = guard.validate(&resolver.temp_dir.join("Desktop"));
        assert!(matches!(res_desktop, Err(SafetyViolation::ForbiddenPathMatched { .. })));

        // Reject Documents
        let res_docs = guard.validate(&resolver.temp_dir.join("Documents"));
        assert!(matches!(res_docs, Err(SafetyViolation::ForbiddenPathMatched { .. })));

        // Reject Drive root (e.g. C:\)
        let res_root = guard.validate(&PathBuf::from("C:\\"));
        assert!(matches!(res_root, Err(SafetyViolation::DriveRootProhibited(_))));

        // Reject path outside safe whitelist
        let res_outside = guard.validate(&resolver.temp_dir.join("Unwhitelisted"));
        assert!(matches!(res_outside, Err(SafetyViolation::PathOutsideSafeWhitelist { .. })));

        // Allow valid safe temp path
        let res_safe = guard.validate(&resolver.temp_dir.join("Temp").join("subfile.tmp"));
        assert!(res_safe.is_ok());
    }

    #[test]
    fn test_scanning_engine_computes_exact_bytes_and_counts() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");

        // Write 3 files of exact known sizes: 100 bytes, 250 bytes, 650 bytes = 1000 bytes
        let mut f1 = File::create(temp_folder.join("file1.tmp")).unwrap();
        f1.write_all(&vec![0u8; 100]).unwrap();

        let nested = temp_folder.join("nested");
        fs::create_dir_all(&nested).unwrap();
        let mut f2 = File::create(nested.join("file2.tmp")).unwrap();
        f2.write_all(&vec![0u8; 250]).unwrap();

        let mut f3 = File::create(nested.join("file3.tmp")).unwrap();
        f3.write_all(&vec![0u8; 650]).unwrap();

        let report = engine.scan();
        let temp_target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();

        assert_eq!(temp_target.file_count, 3);
        assert_eq!(temp_target.total_bytes, 1000);
        assert!(temp_target.is_safe);
        assert_eq!(report.total_files, 3);
        assert_eq!(report.total_reclaimable_bytes, 1000);
    }

    #[test]
    fn test_purge_dry_run_leaves_files_intact() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");
        let file_path = temp_folder.join("sample.tmp");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(&vec![0u8; 500]).unwrap();

        let clean_req = CleanRequest {
            target_ids: vec!["user_temp".to_string()],
            dry_run: true,
        };

        let summary = engine.clean(&clean_req);
        assert!(summary.dry_run);
        assert_eq!(summary.files_deleted, 1);
        assert_eq!(summary.bytes_freed, 500);
        assert_eq!(summary.files_skipped_locked, 0);

        // Verification: File must still exist on disk!
        assert!(file_path.exists());
    }

    #[test]
    fn test_purge_live_deletes_files_and_preserves_root() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");
        let file_path = temp_folder.join("sample.tmp");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(&vec![0u8; 500]).unwrap();

        let clean_req = CleanRequest {
            target_ids: vec!["user_temp".to_string()],
            dry_run: false,
        };

        let summary = engine.clean(&clean_req);
        assert!(!summary.dry_run);
        assert_eq!(summary.files_deleted, 1);
        assert_eq!(summary.bytes_freed, 500);

        // Verification: File must be deleted, but Temp root directory MUST still exist!
        assert!(!file_path.exists());
        assert!(temp_folder.exists());
    }

    #[test]
    fn test_purge_skips_locked_file_without_panic() {
        let (_dir, resolver, engine) = setup_mock_env();
        let temp_folder = resolver.temp_dir.join("Temp");
        let locked_path = temp_folder.join("locked.tmp");
        let normal_path = temp_folder.join("normal.tmp");

        {
            let mut f_locked = File::create(&locked_path).unwrap();
            f_locked.write_all(&vec![0u8; 300]).unwrap();
        }

        {
            let mut f_norm = File::create(&normal_path).unwrap();
            f_norm.write_all(&vec![0u8; 700]).unwrap();
        }

        // Lock `locked.tmp` by holding an open file handle with exclusive access (no delete share on Windows)

        #[cfg(windows)]
        let _held_handle = {
            use std::os::windows::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&locked_path)
                .unwrap()
        };

        #[cfg(not(windows))]
        let _held_handle = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&locked_path)
            .unwrap();

        let clean_req = CleanRequest {
            target_ids: vec!["user_temp".to_string()],
            dry_run: false,
        };

        let summary = engine.clean(&clean_req);

        // Verification: Zero panics; normal file deleted, locked file skipped
        assert_eq!(summary.files_deleted, 1);
        assert_eq!(summary.bytes_freed, 700);
        assert_eq!(summary.files_skipped_locked, 1);
        assert!(!normal_path.exists());
        assert!(locked_path.exists());
    }
}
