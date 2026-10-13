use chrono::Utc;
use crate::models::{CpuSnapshot, DiagnosticFinding, DiskSnapshot, MemorySnapshot, ProcessInfo, SeverityLevel, TemperatureSensor};

/// Rolling history tracker for sustained metrics
#[derive(Debug, Default, Clone)]
pub struct DiagnosticHistory {
    pub cpu_usage_history: Vec<f32>,
    pub memory_usage_history: Vec<f32>,
    pub last_sample_time: Option<chrono::DateTime<Utc>>,
}

impl DiagnosticHistory {
    pub fn new() -> Self {
        Self {
            cpu_usage_history: Vec::new(),
            memory_usage_history: Vec::new(),
            last_sample_time: None,
        }
    }

    pub fn record_sample(&mut self, cpu_usage: f32, mem_usage: f32) {
        self.cpu_usage_history.push(cpu_usage);
        if self.cpu_usage_history.len() > 10 {
            self.cpu_usage_history.remove(0);
        }

        self.memory_usage_history.push(mem_usage);
        if self.memory_usage_history.len() > 10 {
            self.memory_usage_history.remove(0);
        }

        self.last_sample_time = Some(Utc::now());
    }
}

pub struct DiagnosticEngine {
    pub history: DiagnosticHistory,
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            history: DiagnosticHistory::new(),
        }
    }

    pub fn evaluate(
        &mut self,
        cpu: &CpuSnapshot,
        mem: &MemorySnapshot,
        disks: &[DiskSnapshot],
        temperatures: &[TemperatureSensor],
        processes: &[ProcessInfo],
    ) -> Vec<DiagnosticFinding> {
        let now = Utc::now();
        let mut findings = Vec::new();

        // Check stale data
        if let Some(last_time) = self.history.last_sample_time {
            let elapsed_sec = (now - last_time).num_seconds();
            if elapsed_sec > 10 {
                findings.push(DiagnosticFinding {
                    rule_id: "SENSOR_STALE_DATA".to_string(),
                    severity: SeverityLevel::Low,
                    title: "Monitoring Data Sample Lag".to_string(),
                    description: format!("No monitoring sample was captured for {} seconds, exceeding the 10s freshness threshold.", elapsed_sec),
                    evidence: vec![format!("Time difference: {} seconds", elapsed_sec)],
                    observed_at: now,
                    confidence: 0.95,
                    possible_causes: vec![
                        "Background thread throttling or system hibernation".to_string(),
                        "High load delaying timer dispatch".to_string(),
                    ],
                    next_steps: vec![
                        "Verify background service priority".to_string(),
                        "Check for heavy concurrent process interference".to_string(),
                    ],
                    limitations: vec!["Sample frequency relies on timer event loop precision".to_string()],
                });
            }
        }

        // Record current metrics into history
        self.history.record_sample(cpu.global_usage_percent, mem.usage_percent);

        // 1. PERF_CPU_HIGH_SUSTAINED: CPU > 90% for >= 3 consecutive samples -> High
        if self.history.cpu_usage_history.len() >= 3 {
            let recent_cpu = &self.history.cpu_usage_history[self.history.cpu_usage_history.len() - 3..];
            if recent_cpu.iter().all(|&val| val >= 90.0) {
                findings.push(DiagnosticFinding {
                    rule_id: "PERF_CPU_HIGH_SUSTAINED".to_string(),
                    severity: SeverityLevel::High,
                    title: "Sustained High CPU Utilization".to_string(),
                    description: "CPU utilization has remained above 90% across 3 or more consecutive samples.".to_string(),
                    evidence: recent_cpu.iter().enumerate().map(|(i, v)| format!("Sample -{}: {:.1}%", 3 - i, v)).collect(),
                    observed_at: now,
                    confidence: 0.98,
                    possible_causes: vec![
                        "Intensive compute task (compilation, rendering, video encoding)".to_string(),
                        "Runaway or unresponsive background thread".to_string(),
                    ],
                    next_steps: vec![
                        "Inspect the active process list for top CPU consumers".to_string(),
                        "Terminate non-essential heavy processes if unintended".to_string(),
                    ],
                    limitations: vec!["Short bursts between sample intervals are not reflected in discrete sampling".to_string()],
                });
            }
        }

        // 2. PERF_MEM_HIGH_SUSTAINED: RAM > 90% used for >= 3 samples -> High
        if self.history.memory_usage_history.len() >= 3 {
            let recent_mem = &self.history.memory_usage_history[self.history.memory_usage_history.len() - 3..];
            if recent_mem.iter().all(|&val| val >= 90.0) {
                findings.push(DiagnosticFinding {
                    rule_id: "PERF_MEM_HIGH_SUSTAINED".to_string(),
                    severity: SeverityLevel::High,
                    title: "Sustained High Memory Utilization".to_string(),
                    description: "Physical memory utilization has remained above 90% across 3 consecutive checks.".to_string(),
                    evidence: recent_mem.iter().enumerate().map(|(i, v)| format!("Sample -{}: {:.1}%", 3 - i, v)).collect(),
                    observed_at: now,
                    confidence: 0.95,
                    possible_causes: vec![
                        "High active working set across many applications".to_string(),
                        "Memory leak in one or more long-running programs".to_string(),
                    ],
                    next_steps: vec![
                        "Close unused applications or browser tabs".to_string(),
                        "Check process memory usage breakdown in Processes tab".to_string(),
                    ],
                    limitations: vec!["Cache memory may be reclaimed on demand by Windows memory manager".to_string()],
                });
            }
        }

        // 3. PERF_MEM_LOW_AVAILABLE: Available < 500MB -> Moderate
        let five_hundred_mb = 500 * 1024 * 1024;
        if mem.available_bytes < five_hundred_mb {
            let avail_mb = mem.available_bytes / (1024 * 1024);
            findings.push(DiagnosticFinding {
                rule_id: "PERF_MEM_LOW_AVAILABLE".to_string(),
                severity: SeverityLevel::Moderate,
                title: "Critically Low Available Physical Memory".to_string(),
                description: format!("Available system memory is below 500 MB (current: {} MB), which may trigger aggressive swapping.", avail_mb),
                evidence: vec![format!("Available memory: {} MB / {} MB total", avail_mb, mem.total_bytes / (1024 * 1024))],
                observed_at: now,
                confidence: 0.99,
                possible_causes: vec![
                    "Multiple memory-heavy applications open concurrently".to_string(),
                    "Insufficient physical RAM installed for current workload".to_string(),
                ],
                next_steps: vec![
                    "Close idle applications".to_string(),
                    "Verify paging file settings in Windows performance options".to_string(),
                ],
                limitations: vec!["Does not measure standby list pages available for fast repurposing".to_string()],
            });
        }

        // 4 & 5. Disk capacity checks
        for disk in disks {
            let free_pct = if disk.total_bytes > 0 {
                (disk.available_bytes as f64 / disk.total_bytes as f64) * 100.0
            } else {
                100.0
            };

            if free_pct < 2.0 {
                findings.push(DiagnosticFinding {
                    rule_id: "DISK_CRITICAL_CAPACITY".to_string(),
                    severity: SeverityLevel::Critical,
                    title: format!("Critical Free Space on Drive {}", disk.mount_point),
                    description: format!("Drive {} has only {:.1}% free space left (< 2%), posing immediate risk of OS failure or file corruption.", disk.mount_point, free_pct),
                    evidence: vec![
                        format!("Drive: {}", disk.mount_point),
                        format!("Available: {:.2} GB of {:.2} GB", disk.available_bytes as f64 / 1_073_741_824.0, disk.total_bytes as f64 / 1_073_741_824.0),
                    ],
                    observed_at: now,
                    confidence: 1.0,
                    possible_causes: vec![
                        "Accumulation of temporary files, crash dumps, or logs".to_string(),
                        "Large downloads, system backups, or unemptied Recycle Bin".to_string(),
                    ],
                    next_steps: vec![
                        "Run Windows Disk Cleanup (cleanmgr)".to_string(),
                        "Empty the Recycle Bin immediately".to_string(),
                        "Move large files or videos to secondary storage".to_string(),
                    ],
                    limitations: vec!["Shadow copies and reserved system restore space may not appear in free stats".to_string()],
                });
            } else if free_pct < 10.0 {
                findings.push(DiagnosticFinding {
                    rule_id: "DISK_LOW_CAPACITY".to_string(),
                    severity: SeverityLevel::Moderate,
                    title: format!("Low Free Space on Drive {}", disk.mount_point),
                    description: format!("Drive {} has less than 10% free space remaining ({:.1}% free).", disk.mount_point, free_pct),
                    evidence: vec![
                        format!("Drive: {}", disk.mount_point),
                        format!("Available: {:.2} GB / {:.2} GB", disk.available_bytes as f64 / 1_073_741_824.0, disk.total_bytes as f64 / 1_073_741_824.0),
                    ],
                    observed_at: now,
                    confidence: 1.0,
                    possible_causes: vec![
                        "Normal storage growth without routine cleanup".to_string(),
                        "Cache buildup from games or development tools".to_string(),
                    ],
                    next_steps: vec![
                        "Review installed applications and uninstall unneeded software".to_string(),
                        "Clean up temp directories (%TEMP%)".to_string(),
                    ],
                    limitations: vec!["Virtual disks or network shares might present dynamic quota limits".to_string()],
                });
            }
        }

        // 7. SENSOR_TEMP_UNAVAILABLE: No temperature -> Informational
        let valid_temp_count = temperatures.iter().filter(|t| t.celsius.is_some()).count();
        if temperatures.is_empty() || valid_temp_count == 0 {
            findings.push(DiagnosticFinding {
                rule_id: "SENSOR_TEMP_UNAVAILABLE".to_string(),
                severity: SeverityLevel::Informational,
                title: "Hardware Temperature Sensors Not Exposed".to_string(),
                description: "Hardware thermal sensors are not readable via standard OS APIs without elevated kernel drivers.".to_string(),
                evidence: vec!["0 active temperature readings returned by platform collector".to_string()],
                observed_at: now,
                confidence: 1.0,
                possible_causes: vec![
                    "Standard user privilege level on Windows restricts ACPI/WMI thermal zone access".to_string(),
                    "Motherboard proprietary controller requires vendor-signed ring-0 driver".to_string(),
                ],
                next_steps: vec![
                    "Use vendor-specific monitoring utilities (e.g., Ryzen Master, Intel XTU) if temperature tracking is needed".to_string(),
                ],
                limitations: vec!["Windows does not provide standard thermal counters to unprivileged user-mode applications".to_string()],
            });
        }

        // 8. PERF_UNUSUAL_CPU_PROCESS: Single process > 80% CPU -> Low
        for proc in processes {
            if proc.cpu_percent >= 80.0 {
                findings.push(DiagnosticFinding {
                    rule_id: "PERF_UNUSUAL_CPU_PROCESS".to_string(),
                    severity: SeverityLevel::Low,
                    title: format!("High CPU Consumption by Process: {}", proc.name),
                    description: format!("Process {} (PID: {}) is consuming {:.1}% of total CPU capacity.", proc.name, proc.pid, proc.cpu_percent),
                    evidence: vec![
                        format!("Process name: {}", proc.name),
                        format!("PID: {}", proc.pid),
                        format!("CPU usage: {:.1}%", proc.cpu_percent),
                    ],
                    observed_at: now,
                    confidence: 0.90,
                    possible_causes: vec![
                        "Legitimate heavy workload or compile task".to_string(),
                        "Infinite loop or thread synchronization lockup in application".to_string(),
                    ],
                    next_steps: vec![
                        format!("Check if {} is an intentional user activity", proc.name),
                        "Check task manager or terminate process if unresponsive".to_string(),
                    ],
                    limitations: vec!["Multi-core scaling may report instantaneous utilization exceeding 100% on some calculation methods".to_string()],
                });
            }
        }

        findings
    }
}
