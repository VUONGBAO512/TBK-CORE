use super::models::CleanableTarget;
use super::safety::SafetyGuard;
use std::fs;
use std::path::Path;

const MAX_SCAN_DEPTH: usize = 32;

pub struct StorageScanner;

impl StorageScanner {
    /// Recursively scans target directory with symlink escape prevention and non-negative byte accumulation
    pub fn scan_target(target: &mut CleanableTarget, safety_guard: &SafetyGuard) {
        let root = Path::new(&target.root_path);
        if target.root_path.is_empty() || !root.exists() {
            target.file_count = 0;
            target.total_bytes = 0;
            target.is_safe = true;
            return;
        }

        if let Err(_) = safety_guard.validate(root) {
            target.file_count = 0;
            target.total_bytes = 0;
            target.is_safe = false;
            return;
        }

        let mut count: usize = 0;
        let mut total_bytes: u64 = 0;
        Self::scan_recursive(root, &target.id, 0, &mut count, &mut total_bytes);

        target.file_count = count;
        target.total_bytes = total_bytes;
        target.is_safe = true;
    }

    fn scan_recursive(
        dir: &Path,
        target_id: &str,
        depth: usize,
        file_count: &mut usize,
        total_bytes: &mut u64,
    ) {
        if depth > MAX_SCAN_DEPTH {
            return;
        }

        let entries = match fs::read_dir(dir) {
            Ok(iter) => iter,
            Err(_) => return, // Gracefully tolerate permission denied or missing folders
        };

        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            // Inspect via symlink_metadata so reparse points/symlinks are never followed!
            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let file_type = meta.file_type();
            if file_type.is_symlink() {
                // Do NOT recurse into symlinks or junctions!
                continue;
            }

            if file_type.is_dir() {
                Self::scan_recursive(&path, target_id, depth + 1, file_count, total_bytes);
            } else if file_type.is_file() {
                // Apply optional target filters
                if Self::matches_filter(target_id, &path) {
                    *file_count += 1;
                    *total_bytes = total_bytes.saturating_add(meta.len());
                }
            }
        }
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
