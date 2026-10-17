use serde::{Deserialize, Serialize};

pub const DEFAULT_RUNAWAY_CPU_PERCENT: f32 = 90.0;
pub const DEFAULT_RUNAWAY_MEMORY_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GB (2,147,483,648 bytes)
pub const DEFAULT_DUAL_CPU_PERCENT: f32 = 70.0;
pub const DEFAULT_DUAL_MEMORY_BYTES: u64 = 1_610_612_736; // 1.5 GB (1,610,612,736 bytes)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunawayThresholds {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub dual_cpu_percent: f32,
    pub dual_memory_bytes: u64,
}

impl Default for RunawayThresholds {
    fn default() -> Self {
        Self {
            cpu_percent: DEFAULT_RUNAWAY_CPU_PERCENT,
            memory_bytes: DEFAULT_RUNAWAY_MEMORY_BYTES,
            dual_cpu_percent: DEFAULT_DUAL_CPU_PERCENT,
            dual_memory_bytes: DEFAULT_DUAL_MEMORY_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunawayKind {
    Cpu { cpu_percent: f32 },
    Memory { memory_bytes: u64 },
    DualPressure { cpu_percent: f32, memory_bytes: u64 },
}

/// Returns true if a process exceeds healthy resource thresholds.
pub fn is_runaway(cpu_percent: f32, memory_bytes: u64) -> bool {
    is_runaway_with_thresholds(cpu_percent, memory_bytes, &RunawayThresholds::default())
}

pub fn is_runaway_with_thresholds(
    cpu_percent: f32,
    memory_bytes: u64,
    thresholds: &RunawayThresholds,
) -> bool {
    detect_runaway_with_thresholds(cpu_percent, memory_bytes, thresholds).is_some()
}

/// Detects the runaway kind if thresholds are exceeded, or None if healthy.
pub fn detect_runaway(cpu_percent: f32, memory_bytes: u64) -> Option<RunawayKind> {
    detect_runaway_with_thresholds(cpu_percent, memory_bytes, &RunawayThresholds::default())
}

pub fn detect_runaway_with_thresholds(
    cpu_percent: f32,
    memory_bytes: u64,
    thresholds: &RunawayThresholds,
) -> Option<RunawayKind> {
    // 1. Single CPU threshold (CPU >= 90%)
    if cpu_percent >= thresholds.cpu_percent {
        return Some(RunawayKind::Cpu { cpu_percent });
    }

    // 2. Single RAM threshold (RAM > 2GB)
    if memory_bytes > thresholds.memory_bytes {
        return Some(RunawayKind::Memory { memory_bytes });
    }

    // 3. Dual Pressure threshold (CPU >= 70% AND RAM >= 1.5GB)
    if cpu_percent >= thresholds.dual_cpu_percent && memory_bytes >= thresholds.dual_memory_bytes {
        return Some(RunawayKind::DualPressure {
            cpu_percent,
            memory_bytes,
        });
    }

    None
}
