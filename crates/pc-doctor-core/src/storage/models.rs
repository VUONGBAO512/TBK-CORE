pub use crate::models::{CleanRequest, CleanableTarget, StorageCleanSummary, StorageScanReport};
use serde::{Deserialize, Serialize};

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
