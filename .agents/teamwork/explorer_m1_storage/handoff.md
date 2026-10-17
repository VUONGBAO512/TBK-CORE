# Handoff Report: Milestone 1 Storage Engine (`pc-doctor-core::storage`)

## 1. Observation

### Codebase State & Current Baseline
- **Original Request (`d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md`)**:
  - Requirement R2 (lines 15–17): "The system must provide safe scanning and calculation of reclaimable storage across safe Windows temporary directories (such as `%TEMP%`, user crash dumps, application log files, and Windows thumbnail/prefetch caches where accessible). The scanner must compute exact reclaimable byte sizes, display an itemized breakdown, and execute safe purges only upon user confirmation with dry-run/preview support and protection against open/locked files."
  - Acceptance Criteria (lines 30–33):
    - "Scanning temporary directories returns a structured list of cleanable targets with verified non-negative byte sizes."
    - "Cleaning actions safely skip locked or in-use files without crashing or panicking, returning a summary of freed bytes."
    - "Core operating system files and directories outside designated safe temp locations are strictly untouched."
- **Project Plan (`d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md`)**:
  - Architecture (line 4): "`crates/pc-doctor-core`: Pure domain models, diagnostic rules, safe storage engine (targets, scanning, dry-run, locked-file handling, protected boundary validation)..."
  - Code Layout (line 11): "`crates/pc-doctor-core/src/storage/` — Storage cleanable targets, safe root whitelist, protected boundary guard, scanning and purging engine"
  - Interface Contract #1 (lines 53–87):
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
- **Current Core Crate (`crates/pc-doctor-core`)**:
  - Currently has modules: `diagnostics`, `knowledge`, `models.rs`, `lib.rs`.
  - Directory `crates/pc-doctor-core/src/storage/` does not exist yet.
  - Existing test suite passes with 3 tests in `DiagnosticEngine` and `KnowledgeBase` via `cargo test --workspace`.
- **Windows System Characteristics Directly Observed**:
  - In Windows, temporary directories frequently contain active files locked by running applications (e.g., browser sessions, Windows services, editors). Calling `std::fs::remove_file` on such files produces OS error 32 (`ERROR_SHARING_VIOLATION`) or OS error 5 (`ERROR_ACCESS_DENIED`).
  - Path canonicalization via `std::fs::canonicalize` on Windows prepends the verbatim extended-length prefix `\\?\`. Comparing `\\?\C:\Temp` with `C:\Temp` fails string equality and standard prefix checks unless normalized.
  - NTFS directory junctions and symlinks can point outside parent containers, creating risk of directory traversal or cyclic recursion.

---

## 2. Logic Chain

1. **Safety Boundary Priority**:
   - Because cleaning involves file deletion on the user's host machine, safety boundary enforcement must be proactive, strict, and defense-in-depth (combining both a safe root whitelist and a hardcoded forbidden blacklist).
   - A whitelist alone is vulnerable if an environment variable is forged or misconfigured to point to `C:\` or `C:\Windows`. A blacklist alone is vulnerable if an unexpected path is targeted. Therefore, both MUST be evaluated for every operation.
   - Component-aware prefix checks (`format!("{}\\", root)`) must be used instead of simple substring matching, preventing prefix collisions such as `C:\Temp` matching `C:\TemporaryFiles`.

2. **Decoupled Architecture & Testability**:
   - Following `codebase-design` principles, `StorageEngine` must be a **deep module**: small public interface (`scan`, `clean`) with rich internal logic.
   - Dynamic path resolution must be backed by a trait (`StoragePathResolver`), with a production implementation querying Windows environment variables (`%TEMP%`, `%LOCALAPPDATA%`, etc.) and a mock implementation allowing unit tests to create hermetic, temporary file trees. This guarantees 100% deterministic test coverage in CI/CD without touching user data.

3. **Symlink & Junction Protection**:
   - Directory recursion must use `symlink_metadata` rather than `metadata` to inspect file entries.
   - If an entry is a symlink or directory junction, the scanner and purger MUST NOT recurse into the target. In purge mode, only the link entry itself may be deleted; target directories outside the safe root remain strictly untouched.
   - A recursion depth limit (`MAX_SCAN_DEPTH = 32`) guards against stack overflow.

4. **Locked & In-Use File Handling**:
   - The purge engine must catch `std::io::Error` during `fs::remove_file`.
   - If the OS error is 32 (`ERROR_SHARING_VIOLATION`), 33 (`ERROR_LOCK_VIOLATION`), or 5 (`ERROR_ACCESS_DENIED`), the file is recorded as `files_skipped_locked += 1`, omitted from `bytes_freed`, and execution continues immediately to the next file without panicking.
   - For read-only temporary files, the purger attempts `perms.set_readonly(false)` before deletion. If it still fails, it is handled gracefully as a locked/skipped file.
   - The target root directory itself (e.g., `%TEMP%`) is NEVER deleted; only child files and empty subdirectories are purged.

5. **Exact Non-Negative Byte Sizing**:
   - Rust's `u64` data type for `total_bytes` and `bytes_freed` guarantees non-negative quantities.
   - In dry-run mode (`dry_run: true`), `remove_file` is never executed; scanned size and file count are returned as previewed reclaimable space.
   - In live mode (`dry_run: false`), `bytes_freed` only accumulates bytes from successfully removed files.

---

## 3. Caveats

1. **Administrator Privileges for System-Wide Logs/WER**:
   - Machine-level error reporting (`%PROGRAMDATA%\Microsoft\Windows\WER`) and system-level temp (`C:\Windows\Temp`) may require elevated administrative privileges (UAC) to delete.
   - *Design decision*: Focus primarily on user-accessible targets (`%LOCALAPPDATA%\Temp`, `%LOCALAPPDATA%\CrashDumps`, `%LOCALAPPDATA%\Microsoft\Windows\WER`, `%LOCALAPPDATA%\Microsoft\Windows\Explorer`). If an un-elevated process encounters permission denied in any target, it increments `files_skipped_locked` or appends to `errors` without panicking.
2. **Explorer Thumbnail Database Locking**:
   - `thumbcache_*.db` files inside `%LOCALAPPDATA%\Microsoft\Windows\Explorer` are frequently held open by `explorer.exe`. They will often be skipped as locked files when Explorer is running. This is normal and expected Windows behavior, fully handled by the skip logic.
3. **Assumptions Made**:
   - Target files are standard Windows filesystem entries.
   - Default environment variables (`LOCALAPPDATA`, `TEMP`, `USERPROFILE`) exist in standard Windows sessions; fallbacks are provided if any variable is missing.

---

## 4. Conclusion & Complete Implementation Blueprint

The storage engine must be structured under `crates/pc-doctor-core/src/storage/` with six coordinated submodules:
1. `models.rs`: Domain types (`CleanableTarget`, `StorageScanReport`, `CleanRequest`, `StorageCleanSummary`, `SafetyViolation`).
2. `safety.rs`: Strict path canonicalizer, safe root whitelist, forbidden folder blacklist, component-aware boundary guard.
3. `targets.rs`: Five cleanable target definitions with EN/VI localization, `StoragePathResolver` trait, and Windows dynamic path resolution.
4. `scanner.rs`: Recursive directory scanner with symlink protection, non-negative byte accumulation, and depth guards.
5. `purge.rs`: Purge execution engine with dry-run support, locked/in-use file skip handling (OS errors 32/5), and summary generation.
6. `mod.rs`: Facade `StorageEngine` coordinating scanning and purging.

### Detailed Code Blueprint

#### File 1: `crates/pc-doctor-core/src/storage/models.rs`
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageScanReport {
    pub targets: Vec<CleanableTarget>,
    pub total_reclaimable_bytes: u64,
    pub total_files: usize,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanRequest {
    pub target_ids: Vec<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageCleanSummary {
    pub dry_run: bool,
    pub bytes_freed: u64,
    pub files_deleted: usize,
    pub files_skipped_locked: usize,
    pub targets_processed: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyViolation {
    PathNotFound(String),
    PathCanonicalizationFailed(String),
    ForbiddenPathMatched { path: String, rule: String },
    PathOutsideSafeWhitelist { path: String },
    DriveRootProhibited(String),
    UserProfileRootProhibited(String),
    SystemDirectoryProhibited(String),
    SymlinkEscapeAttempt { path: String, target: String },
    ParentDirectoryTraversalDetected(String),
}
```

#### File 2: `crates/pc-doctor-core/src/storage/safety.rs`
```rust
use super::models::SafetyViolation;
use std::path::{Component, Path, PathBuf};

/// Strips the Windows verbatim extended-length prefix (`\\?\` or `\\?\UNC\`)
pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", stripped))
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Normalizes path by resolving lexical dots and canonicalizing if existing
pub fn normalize_path(path: &Path) -> PathBuf {
    if path.exists() {
        if let Ok(canon) = std::fs::canonicalize(path) {
            return strip_verbatim_prefix(&canon);
        }
    }
    // Lexical normalization for paths
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop();
            }
            c => components.push(c),
        }
    }
    let mut norm = PathBuf::new();
    for c in components {
        norm.push(c);
    }
    strip_verbatim_prefix(&norm)
}

/// Component-aware case-insensitive prefix check on Windows
pub fn path_starts_with_normalized(candidate: &Path, prefix: &Path) -> bool {
    let norm_candidate = normalize_path(candidate);
    let norm_prefix = normalize_path(prefix);

    #[cfg(windows)]
    {
        let c_str = norm_candidate.to_string_lossy().to_lowercase();
        let p_str = norm_prefix.to_string_lossy().to_lowercase();
        if c_str == p_str {
            return true;
        }
        let p_with_sep = if p_str.ends_with('\\') || p_str.ends_with('/') {
            p_str
        } else {
            format!("{}\\", p_str)
        };
        c_str.starts_with(&p_with_sep)
    }

    #[cfg(not(windows))]
    {
        norm_candidate.starts_with(&norm_prefix)
    }
}

/// Component-aware equality check
pub fn paths_equal_normalized(p1: &Path, p2: &Path) -> bool {
    let norm1 = normalize_path(p1);
    let norm2 = normalize_path(p2);

    #[cfg(windows)]
    {
        norm1.to_string_lossy().to_lowercase() == norm2.to_string_lossy().to_lowercase()
    }

    #[cfg(not(windows))]
    {
        norm1 == norm2
    }
}

#[derive(Debug, Clone)]
pub struct SafetyGuard {
    safe_roots: Vec<PathBuf>,
    forbidden_roots: Vec<PathBuf>,
}

impl SafetyGuard {
    pub fn new(safe_roots: Vec<PathBuf>, forbidden_roots: Vec<PathBuf>) -> Self {
        Self {
            safe_roots: safe_roots.into_iter().map(|p| normalize_path(&p)).collect(),
            forbidden_roots: forbidden_roots.into_iter().map(|p| normalize_path(&p)).collect(),
        }
    }

    /// Verifies that candidate_path is safely contained within a safe root and does not violate any forbidden rules.
    pub fn validate(&self, candidate_path: &Path) -> Result<(), SafetyViolation> {
        let norm_path = normalize_path(candidate_path);
        let path_str = norm_path.to_string_lossy().to_string();

        // 1. Prohibit drive root (e.g. C:\ or /)
        if norm_path.parent().is_none() || path_str.ends_with(":\\") || path_str == "/" {
            return Err(SafetyViolation::DriveRootProhibited(path_str));
        }

        // 2. Reject if candidate is an ancestor or exact match of any forbidden root
        for forbidden in &self.forbidden_roots {
            let forbidden_str = forbidden.to_string_lossy().to_string();
            // Candidate equals forbidden root
            if paths_equal_normalized(&norm_path, forbidden) {
                return Err(SafetyViolation::ForbiddenPathMatched {
                    path: path_str,
                    rule: format!("Exact match forbidden root: {}", forbidden_str),
                });
            }
            // Candidate is parent of forbidden root (e.g. candidate = C:\Windows, forbidden = C:\Windows\System32)
            if path_starts_with_normalized(forbidden, &norm_path) {
                return Err(SafetyViolation::ForbiddenPathMatched {
                    path: path_str,
                    rule: format!("Ancestor of forbidden root: {}", forbidden_str),
                });
            }
            // Candidate is child of forbidden root, unless explicitly covered by a more specific safe root
            if path_starts_with_normalized(&norm_path, forbidden) {
                // If within forbidden, check if safe_roots has a more specific authorized leaf
                let has_explicit_safe_root = self.safe_roots.iter().any(|safe| {
                    path_starts_with_normalized(&norm_path, safe)
                        && path_starts_with_normalized(safe, forbidden)
                });
                if !has_explicit_safe_root {
                    return Err(SafetyViolation::ForbiddenPathMatched {
                        path: path_str,
                        rule: format!("Contained inside forbidden system directory: {}", forbidden_str),
                    });
                }
            }
        }

        // 3. Must be contained within at least one authorized safe root
        let within_safe_root = self.safe_roots.iter().any(|safe| {
            path_starts_with_normalized(&norm_path, safe)
        });

        if !within_safe_root {
            return Err(SafetyViolation::PathOutsideSafeWhitelist {
                path: path_str,
            });
        }

        Ok(())
    }

    /// Verifies that an individual item to be purged is strictly a child of the target root
    pub fn validate_purge_child(&self, target_root: &Path, child_path: &Path) -> Result<(), SafetyViolation> {
        self.validate(child_path)?;
        let norm_root = normalize_path(target_root);
        let norm_child = normalize_path(child_path);

        // Child must not equal the target root itself (root must not be deleted)
        if paths_equal_normalized(&norm_root, &norm_child) {
            return Err(SafetyViolation::DriveRootProhibited(
                "Target root directory cannot delete itself".to_string(),
            ));
        }

        // Child must be inside target_root
        if !path_starts_with_normalized(&norm_child, &norm_root) {
            return Err(SafetyViolation::ParentDirectoryTraversalDetected(
                norm_child.to_string_lossy().to_string(),
            ));
        }

        Ok(())
    }
}
```

#### File 3: `crates/pc-doctor-core/src/storage/targets.rs`
```rust
use super::models::CleanableTarget;
use std::path::{Path, PathBuf};

pub trait StoragePathResolver: Send + Sync {
    fn resolve_target_root(&self, target_id: &str) -> Option<PathBuf>;
    fn get_safe_roots(&self) -> Vec<PathBuf>;
    fn get_forbidden_roots(&self) -> Vec<PathBuf>;
}

/// Production path resolver for Windows environments
#[derive(Debug, Default)]
pub struct DefaultWindowsPathResolver;

impl StoragePathResolver for DefaultWindowsPathResolver {
    fn resolve_target_root(&self, target_id: &str) -> Option<PathBuf> {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|p| PathBuf::from(p).join("AppData").join("Local"))
            })?;

        match target_id {
            "user_temp" => {
                std::env::var("TEMP")
                    .ok()
                    .map(PathBuf::from)
                    .or_else(|| Some(local_app_data.join("Temp")))
            }
            "crash_dumps" => Some(local_app_data.join("CrashDumps")),
            "windows_error_reporting" => Some(local_app_data.join("Microsoft").join("Windows").join("WER")),
            "explorer_thumbnails" => Some(local_app_data.join("Microsoft").join("Windows").join("Explorer")),
            "application_logs" => Some(local_app_data.join("Temp")),
            _ => None,
        }
    }

    fn get_safe_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        for id in &["user_temp", "crash_dumps", "windows_error_reporting", "explorer_thumbnails", "application_logs"] {
            if let Some(p) = self.resolve_target_root(id) {
                roots.push(p);
            }
        }
        roots
    }

    fn get_forbidden_roots(&self) -> Vec<PathBuf> {
        let mut forbidden = Vec::new();

        // 1. Windows directory and subfolders
        if let Ok(windir) = std::env::var("WINDIR").or_else(|_| std::env::var("SYSTEMROOT")) {
            let win_path = PathBuf::from(&windir);
            forbidden.push(win_path.clone());
            forbidden.push(win_path.join("System32"));
            forbidden.push(win_path.join("SysWOW64"));
            forbidden.push(win_path.join("WinSxS"));
        } else {
            forbidden.push(PathBuf::from(r"C:\Windows"));
            forbidden.push(PathBuf::from(r"C:\Windows\System32"));
        }

        // 2. User sensitive personal directories
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let u_path = PathBuf::from(&userprofile);
            forbidden.push(u_path.clone());
            forbidden.push(u_path.join("Desktop"));
            forbidden.push(u_path.join("Documents"));
            forbidden.push(u_path.join("Downloads"));
            forbidden.push(u_path.join("Pictures"));
            forbidden.push(u_path.join("Music"));
            forbidden.push(u_path.join("Videos"));
        }

        // 3. Program Files
        if let Ok(pf) = std::env::var("ProgramFiles") {
            forbidden.push(PathBuf::from(pf));
        }
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            forbidden.push(PathBuf::from(pf86));
        }

        forbidden
    }
}

pub struct TargetCatalog;

impl TargetCatalog {
    pub fn get_definitions(resolver: &dyn StoragePathResolver) -> Vec<CleanableTarget> {
        vec![
            CleanableTarget {
                id: "user_temp".to_string(),
                name_en: "User Temporary Files".to_string(),
                name_vi: "Tệp tạm người dùng (%TEMP%)".to_string(),
                description_en: "Temporary cache and scratch files generated by desktop applications and installers in %TEMP%.".to_string(),
                description_vi: "Các tệp bộ đệm tạm thời do ứng dụng và bộ cài đặt tạo ra trong thư mục %TEMP%.".to_string(),
                root_path: resolver.resolve_target_root("user_temp").map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                file_count: 0,
                total_bytes: 0,
                is_safe: true,
            },
            CleanableTarget {
                id: "crash_dumps".to_string(),
                name_en: "Windows Crash Dumps".to_string(),
                name_vi: "Tệp kết xuất sự cố ứng dụng (Crash Dumps)".to_string(),
                description_en: "Memory dump files (.dmp) recorded automatically by Windows Error Reporting when applications terminate unexpectedly.".to_string(),
                description_vi: "Các tệp kết xuất bộ nhớ (.dmp) ghi lại khi ứng dụng bị dừng đột ngột.".to_string(),
                root_path: resolver.resolve_target_root("crash_dumps").map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                file_count: 0,
                total_bytes: 0,
                is_safe: true,
            },
            CleanableTarget {
                id: "windows_error_reporting".to_string(),
                name_en: "Windows Error Reporting (WER) Queue".to_string(),
                name_vi: "Bộ nhớ đệm báo cáo lỗi Windows (WER)".to_string(),
                description_en: "Archived and queued diagnostic error reports generated by the Windows Error Reporting service.".to_string(),
                description_vi: "Báo cáo lỗi chẩn đoán chờ gửi hoặc lưu trữ của dịch vụ Windows Error Reporting.".to_string(),
                root_path: resolver.resolve_target_root("windows_error_reporting").map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                file_count: 0,
                total_bytes: 0,
                is_safe: true,
            },
            CleanableTarget {
                id: "explorer_thumbnails".to_string(),
                name_en: "Windows Explorer Thumbnail Cache".to_string(),
                name_vi: "Bộ đệm hình thu nhỏ File Explorer (Thumbnails)".to_string(),
                description_en: "Cached image and video thumbnail preview databases (thumbcache_*.db). File Explorer will rebuild them as needed.".to_string(),
                description_vi: "Cơ sở dữ liệu hình thu nhỏ xem trước của File Explorer (thumbcache_*.db). Hệ thống sẽ tự tạo lại khi cần.".to_string(),
                root_path: resolver.resolve_target_root("explorer_thumbnails").map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                file_count: 0,
                total_bytes: 0,
                is_safe: true,
            },
            CleanableTarget {
                id: "application_logs".to_string(),
                name_en: "Application Diagnostic Logs".to_string(),
                name_vi: "Nhật ký chẩn đoán ứng dụng cũ (*.log)".to_string(),
                description_en: "Outdated text log files (*.log, *.etl) accumulated in local application temporary folders.".to_string(),
                description_vi: "Các tệp văn bản ghi log sự kiện cũ (*.log, *.etl) tích lũy trong thư mục tạm.".to_string(),
                root_path: resolver.resolve_target_root("application_logs").map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                file_count: 0,
                total_bytes: 0,
                is_safe: true,
            },
        ]
    }
}
```

#### File 4: `crates/pc-doctor-core/src/storage/scanner.rs`
```rust
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
```

#### File 5: `crates/pc-doctor-core/src/storage/purge.rs`
```rust
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
                let mut perms = meta.permissions();
                if perms.readonly() {
                    perms.set_readonly(false);
                    let _ = fs::set_permissions(&path, perms);
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
```

#### File 6: `crates/pc-doctor-core/src/storage/mod.rs`
```rust
pub mod models;
pub mod purge;
pub mod safety;
pub mod scanner;
pub mod targets;

pub use models::*;
pub use purge::StoragePurger;
pub use safety::{SafetyGuard, normalize_path};
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
```

---

## 5. Verification Method

### Test Suite Execution
Execute the project test suite using `run_command`:
```powershell
cargo test --workspace
```
Expected output: All existing unit tests pass, plus all new storage engine unit tests pass with zero failures and zero unhandled panics.

### Concrete Unit Test Suite Implementation Specification

The following unit tests should be placed in `crates/pc-doctor-core/src/storage/mod.rs` (or `crates/pc-doctor-core/tests/storage_tests.rs`):

```rust
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

        let mut f_locked = File::create(&locked_path).unwrap();
        f_locked.write_all(&vec![0u8; 300]).unwrap();

        let mut f_norm = File::create(&normal_path).unwrap();
        f_norm.write_all(&vec![0u8; 700]).unwrap();

        // Lock `locked.tmp` by holding an open file handle with write mode (exclusive on Windows)
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
```

### Invalidation Conditions
- Any file path matching `%WINDIR%`, `System32`, `Desktop`, `Documents`, or `C:\` being accessed for deletion invalidates safety.
- Any unhandled panic or crash upon encountering a locked file (`ERROR_SHARING_VIOLATION` or `ERROR_ACCESS_DENIED`) invalidates the implementation.
- Any negative or inaccurate byte count or file count in `StorageScanReport` or `StorageCleanSummary` invalidates the engine.
