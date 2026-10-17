use super::models::{CleanRequest, CleanableTarget, StorageCleanSummary};
use super::safety::SafetyGuard;
use std::fs;
use std::path::Path;

pub struct StoragePurger;

impl StoragePurger {
    pub fn purge(
        request: &CleanRequest,
        targets: &[CleanableTarget],
        safety_guard: &SafetyGuard,
    ) -> StorageCleanSummary {
        let mut summary = StorageCleanSummary {
            dry_run: request.dry_run,
            bytes_freed: 0,
            files_deleted: 0,
            files_skipped_locked: 0,
            targets_processed: Vec::new(),
            errors: Vec::new(),
        };

        for target_id in &request.target_ids {
            let target = match targets.iter().find(|t| &t.id == target_id) {
                Some(t) => t,
                None => {
                    summary.errors.push(format!("Target ID '{}' is unknown", target_id));
                    continue;
                }
            };

            let root = Path::new(&target.root_path);
            if target.root_path.is_empty() || !root.exists() {
                summary.targets_processed.push(target_id.clone());
                continue;
            }

            // Safety boundary enforcement
            if let Err(violation) = safety_guard.validate(root) {
                summary.errors.push(format!(
                    "Safety boundary violation for target '{}': {:?}",
                    target_id, violation
                ));
                continue;
            }

            if request.dry_run {
                // In dry run, simulate without deleting
                summary.files_deleted += target.file_count;
                summary.bytes_freed += target.total_bytes;
                summary.targets_processed.push(target_id.clone());
            } else {
                // Live purge: delete child items while skipping locked files
                Self::purge_directory_contents(
                    root,
                    root,
                    &target.id,
                    safety_guard,
                    &mut summary.bytes_freed,
                    &mut summary.files_deleted,
                    &mut summary.files_skipped_locked,
                    &mut summary.errors,
                );
                summary.targets_processed.push(target_id.clone());
            }
        }

        summary
    }

    fn purge_directory_contents(
        target_root: &Path,
        current_dir: &Path,
        target_id: &str,
        safety_guard: &SafetyGuard,
        bytes_freed: &mut u64,
        files_deleted: &mut usize,
        files_skipped_locked: &mut usize,
        errors: &mut Vec<String>,
    ) {
        let entries = match fs::read_dir(current_dir) {
            Ok(iter) => iter,
            Err(e) => {
                if Self::is_locked_error(&e) {
                    *files_skipped_locked += 1;
                } else {
                    errors.push(format!("Could not read directory {}: {}", current_dir.display(), e));
                }
                return;
            }
        };

        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if let Err(_) = safety_guard.validate_purge_child(target_root, &path) {
                continue; // Safety stop: do not touch anything outside target_root
            }

            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let file_type = meta.file_type();
            if file_type.is_symlink() {
                // Delete the link itself, never follow
                let len = meta.len();
                let res = if meta.is_dir() {
                    fs::remove_dir(&path)
                } else {
                    fs::remove_file(&path)
                };
                match res {
                    Ok(_) => {
                        *files_deleted += 1;
                        *bytes_freed = bytes_freed.saturating_add(len);
                    }
                    Err(e) => {
                        if Self::is_locked_error(&e) {
                            *files_skipped_locked += 1;
                        }
                    }
                }
                continue;
            }

            if file_type.is_dir() {
                // Recursively purge subdirectory contents first
                Self::purge_directory_contents(
                    target_root,
                    &path,
                    target_id,
                    safety_guard,
                    bytes_freed,
                    files_deleted,
                    files_skipped_locked,
                    errors,
                );
                // Attempt to remove the subdirectory now if empty (never removes target_root)
                let _ = fs::remove_dir(&path);
            } else if file_type.is_file() {
                if !Self::matches_filter(target_id, &path) {
                    continue;
                }

                let len = meta.len();

                // If read-only, attempt to clear read-only flag
                if let Ok(mut perms) = fs::metadata(&path).map(|m| m.permissions()) {
                    if perms.readonly() {
                        perms.set_readonly(false);
                        let _ = fs::set_permissions(&path, perms);
                    }
                }

                // Attempt deletion
                match fs::remove_file(&path) {
                    Ok(_) => {
                        *files_deleted += 1;
                        *bytes_freed = bytes_freed.saturating_add(len);
                    }
                    Err(e) => {
                        if Self::is_locked_error(&e) {
                            // File in use or locked: skip gracefully without panicking
                            *files_skipped_locked += 1;
                        } else {
                            errors.push(format!("Failed to delete {}: {}", path.display(), e));
                        }
                    }
                }
            }
        }
    }

    /// Identifies whether the error indicates a locked, open, or permission-denied file
    fn is_locked_error(err: &std::io::Error) -> bool {
        // Windows OS Error Codes:
        // 32 = ERROR_SHARING_VIOLATION (file in use by another process)
        // 33 = ERROR_LOCK_VIOLATION
        // 5  = ERROR_ACCESS_DENIED
        if let Some(code) = err.raw_os_error() {
            if code == 32 || code == 33 || code == 5 {
                return true;
            }
        }
        err.kind() == std::io::ErrorKind::PermissionDenied
    }

    fn matches_filter(target_id: &str, path: &Path) -> bool {
        match target_id {
            "explorer_thumbnails" => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                name.starts_with("thumbcache_") || name.starts_with("iconcache_")
            }
            "application_logs" => {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                ext == "log" || ext == "etl"
            }
            _ => true,
        }
    }
}
