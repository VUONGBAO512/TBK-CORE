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

/// Normalizes path by resolving lexical dots and canonicalizing existing components
pub fn normalize_path(path: &Path) -> PathBuf {
    if path.exists() {
        if let Ok(canon) = std::fs::canonicalize(path) {
            return strip_verbatim_prefix(&canon);
        }
    }

    // If path does not exist directly, try canonicalizing the deepest existing ancestor
    // to resolve 8.3 short names and symlinks consistently.
    let mut ancestor = path;
    let mut tail = Vec::new();
    while let Some(parent) = ancestor.parent() {
        if let Some(file_name) = ancestor.file_name() {
            tail.push(file_name);
        }
        if parent.exists() {
            if let Ok(canon_parent) = std::fs::canonicalize(parent) {
                let mut norm = strip_verbatim_prefix(&canon_parent);
                for component in tail.into_iter().rev() {
                    norm.push(component);
                }
                return norm;
            }
        }
        ancestor = parent;
    }

    // Lexical normalization fallback
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
        let c_str = norm_candidate.to_string_lossy().to_lowercase().replace('/', "\\");
        let p_str = norm_prefix.to_string_lossy().to_lowercase().replace('/', "\\");
        if c_str == p_str {
            return true;
        }
        let p_with_sep = if p_str.ends_with('\\') {
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
        norm1.to_string_lossy().to_lowercase().replace('/', "\\")
            == norm2.to_string_lossy().to_lowercase().replace('/', "\\")
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
        if norm_path.parent().is_none()
            || path_str.ends_with(":\\")
            || path_str.ends_with(":/")
            || path_str == "/"
            || path_str == "\\"
        {
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
