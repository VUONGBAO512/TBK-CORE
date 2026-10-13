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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCoreInfo {
    pub core_id: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub model_name: String,
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub global_usage_percent: f32,
    pub cores: Vec<CpuCoreInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsSnapshot {
    pub name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub host_name: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureSensor {
    pub label: String,
    pub celsius: Option<f32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub os: OsSnapshot,
    pub temperatures: Vec<TemperatureSensor>,
    pub findings: Vec<DiagnosticFinding>,
}
