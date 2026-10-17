use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityLevel {
    Informational,
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompanionMood {
    Idle,
    Thinking,
    Healthy,
    Warning,
    Critical,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpuCoreInfo {
    pub core_id: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpuSnapshot {
    pub model_name: String,
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub global_usage_percent: f32,
    pub cores: Vec<CpuCoreInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskSnapshot {
    pub name: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f32,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsSnapshot {
    pub name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub host_name: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemperatureSensor {
    pub label: String,
    pub celsius: Option<f32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticFinding {
    pub rule_id: String,
    pub severity: SeverityLevel,
    pub title: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub observed_at: DateTime<Utc>,
    pub confidence: f32,
    pub possible_causes: Vec<String>,
    pub next_steps: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub os: OsSnapshot,
    pub temperatures: Vec<TemperatureSensor>,
    pub findings: Vec<DiagnosticFinding>,
}

// ==========================================
// Storage Engine Models (Interface Contract 1)
// ==========================================

/// Itemized cleanable target representing a known safe directory or cache type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl CleanableTarget {
    pub fn new(
        id: impl Into<String>,
        name_en: impl Into<String>,
        name_vi: impl Into<String>,
        description_en: impl Into<String>,
        description_vi: impl Into<String>,
        root_path: impl Into<String>,
        file_count: usize,
        total_bytes: u64,
        is_safe: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name_en: name_en.into(),
            name_vi: name_vi.into(),
            description_en: description_en.into(),
            description_vi: description_vi.into(),
            root_path: root_path.into(),
            file_count,
            total_bytes,
            is_safe,
        }
    }
}

/// Comprehensive scan report produced by the storage scanner
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageScanReport {
    pub targets: Vec<CleanableTarget>,
    pub total_reclaimable_bytes: u64,
    pub total_files: usize,
    pub scanned_at: DateTime<Utc>,
}

impl StorageScanReport {
    pub fn empty() -> Self {
        Self {
            targets: Vec::new(),
            total_reclaimable_bytes: 0,
            total_files: 0,
            scanned_at: Utc::now(),
        }
    }

    pub fn new(targets: Vec<CleanableTarget>) -> Self {
        let total_reclaimable_bytes = targets.iter().map(|t| t.total_bytes).sum();
        let total_files = targets.iter().map(|t| t.file_count).sum();
        Self {
            targets,
            total_reclaimable_bytes,
            total_files,
            scanned_at: Utc::now(),
        }
    }
}

/// Request to perform storage cleanup (or dry-run purge simulation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanRequest {
    pub target_ids: Vec<String>,
    pub dry_run: bool,
}

impl CleanRequest {
    pub fn new(target_ids: Vec<String>, dry_run: bool) -> Self {
        Self {
            target_ids,
            dry_run,
        }
    }
}

/// Outcome summary returned after executing a storage clean
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StorageCleanSummary {
    pub dry_run: bool,
    pub bytes_freed: u64,
    pub files_deleted: usize,
    pub files_skipped_locked: usize,
    pub targets_processed: Vec<String>,
    pub errors: Vec<String>,
}

// ==========================================
// Process Management Models (Interface Contract 2)
// ==========================================

/// Categorization classification for running processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessCategory {
    #[serde(alias = "system")]
    System,
    #[serde(alias = "backgroundservice", alias = "service")]
    BackgroundService,
    #[serde(alias = "application", alias = "app")]
    Application,
    #[serde(alias = "browserengine", alias = "browser")]
    BrowserEngine,
}

impl std::fmt::Display for ProcessCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "System"),
            Self::BackgroundService => write!(f, "Background Service"),
            Self::Application => write!(f, "Application"),
            Self::BrowserEngine => write!(f, "Browser Engine"),
        }
    }
}

/// Process item with telemetry, category, and system safety protection flag
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
    pub category: ProcessCategory,
    pub is_protected: bool,
}

impl ProcessItem {
    pub fn new(
        pid: u32,
        name: impl Into<String>,
        cpu_percent: f32,
        memory_bytes: u64,
        status: impl Into<String>,
        category: ProcessCategory,
        is_protected: bool,
    ) -> Self {
        Self {
            pid,
            name: name.into(),
            cpu_percent,
            memory_bytes,
            status: status.into(),
            category,
            is_protected,
        }
    }
}

impl From<ProcessItem> for ProcessInfo {
    fn from(item: ProcessItem) -> Self {
        Self {
            pid: item.pid,
            name: item.name,
            cpu_percent: item.cpu_percent,
            memory_bytes: item.memory_bytes,
            status: item.status,
        }
    }
}

/// Result returned from an attempted process termination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminateResult {
    pub pid: u32,
    pub process_name: String,
    pub success: bool,
    pub message: String,
}

impl TerminateResult {
    pub fn success(pid: u32, name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            pid,
            process_name: name.into(),
            success: true,
            message: message.into(),
        }
    }

    pub fn failure(pid: u32, name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            pid,
            process_name: name.into(),
            success: false,
            message: message.into(),
        }
    }
}

// ==========================================
// Offline Assistant Models
// ==========================================

/// Offline companion assistant response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantResponse {
    pub reply: String,
    pub references: Vec<String>,
}
