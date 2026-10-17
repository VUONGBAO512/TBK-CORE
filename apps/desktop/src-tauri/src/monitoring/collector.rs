use chrono::Utc;
use pc_doctor_core::models::{
    CpuCoreInfo, CpuSnapshot, DiskSnapshot, MemorySnapshot, OsSnapshot,
    ProcessInfo, ProcessItem, SystemSnapshot, TemperatureSensor,
};
use pc_doctor_core::processes::{categorize_process, ProcessGuard};
use pc_doctor_core::DiagnosticEngine;
use std::sync::Mutex;
use sysinfo::{Components, Disks, System};

pub struct SystemCollector {
    sys: System,
    disks: Disks,
    components: Components,
    engine: DiagnosticEngine,
    guard: ProcessGuard,
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let engine = DiagnosticEngine::new();
        let guard = ProcessGuard::new();

        Self {
            sys,
            disks,
            components,
            engine,
            guard,
        }
    }

    pub fn take_snapshot(&mut self) -> SystemSnapshot {
        // Refresh CPU, Memory, and Processes
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.disks.refresh(true);
        self.components.refresh(true);

        let now = Utc::now();

        // 1. CPU Metrics
        let cpus = self.sys.cpus();
        let global_usage = self.sys.global_cpu_usage();
        let model_name = cpus
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let logical_cores = cpus.len();
        let physical_cores = self.sys.physical_core_count().unwrap_or(logical_cores);

        let mut cores = Vec::with_capacity(logical_cores);
        for (i, c) in cpus.iter().enumerate() {
            cores.push(CpuCoreInfo {
                core_id: i,
                usage_percent: c.cpu_usage(),
                frequency_mhz: c.frequency(),
            });
        }

        let cpu = CpuSnapshot {
            model_name,
            logical_cores,
            physical_cores,
            global_usage_percent: global_usage,
            cores,
        };

        // 2. Memory Metrics
        let total_bytes = self.sys.total_memory();
        let used_bytes = self.sys.used_memory();
        let available_bytes = self.sys.available_memory();
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f32 / total_bytes as f32) * 100.0
        } else {
            0.0
        };

        let memory = MemorySnapshot {
            total_bytes,
            used_bytes,
            available_bytes,
            usage_percent,
            swap_total_bytes: self.sys.total_swap(),
            swap_used_bytes: self.sys.used_swap(),
        };

        // 3. Disks Metrics
        let mut disks = Vec::new();
        for d in self.disks.iter() {
            let total = d.total_space();
            let avail = d.available_space();
            let used = total.saturating_sub(avail);
            let usage_pct = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };

            disks.push(DiskSnapshot {
                name: d.name().to_string_lossy().to_string(),
                mount_point: d.mount_point().to_string_lossy().to_string(),
                filesystem: d.file_system().to_string_lossy().to_string(),
                total_bytes: total,
                available_bytes: avail,
                used_bytes: used,
                usage_percent: usage_pct,
                is_removable: d.is_removable(),
            });
        }

        // 4. OS Metrics
        let os = OsSnapshot {
            name: System::name().unwrap_or_else(|| "Windows".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            host_name: System::host_name().unwrap_or_else(|| "localhost".to_string()),
            uptime_seconds: System::uptime(),
        };

        // 5. Thermal Sensors
        let mut temperatures = Vec::new();
        for comp in self.components.iter() {
            temperatures.push(TemperatureSensor {
                label: comp.label().to_string(),
                celsius: comp.temperature(),
                status: if comp.temperature().is_some() {
                    "available".to_string()
                } else {
                    "unavailable".to_string()
                },
            });
        }

        // 6. Top Processes (for diagnostic analysis)
        let top_processes = self.get_top_processes(20);

        // 7. Evaluate Diagnostics
        let findings = self.engine.evaluate(&cpu, &memory, &disks, &temperatures, &top_processes);

        SystemSnapshot {
            timestamp: now,
            cpu,
            memory,
            disks,
            os,
            temperatures,
            findings,
        }
    }

    /// Refreshes process list and metrics from operating system.
    pub fn refresh_processes(&mut self) {
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    }

    /// Returns all running processes categorized and annotated with protection flags.
    /// Sorted descending by CPU percentage, with fallback to memory usage.
    pub fn get_process_list(&self) -> Vec<ProcessItem> {
        let mut list: Vec<ProcessItem> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, proc)| {
                let pid_u32 = pid.as_u32();
                let name = proc.name().to_string_lossy().to_string();
                let exe_path = proc.exe().and_then(|p| p.to_str());
                let category = categorize_process(pid_u32, &name, exe_path);
                let is_protected = self.guard.is_protected(pid_u32, &name);

                ProcessItem {
                    pid: pid_u32,
                    name,
                    cpu_percent: proc.cpu_usage(),
                    memory_bytes: proc.memory(),
                    status: format!("{:?}", proc.status()),
                    category,
                    is_protected,
                }
            })
            .collect();

        // Sort descending by CPU percentage, fallback to memory
        list.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
        });

        list
    }

    /// Returns top resource-consuming processes as `Vec<ProcessInfo>` for diagnostic evaluation.
    pub fn get_top_processes(&self, limit: usize) -> Vec<ProcessInfo> {
        let mut list = self.get_process_list();
        if list.len() > limit {
            list.truncate(limit);
        }
        list.into_iter().map(ProcessInfo::from).collect()
    }

    /// Accessor for underlying ProcessGuard
    pub fn guard(&self) -> &ProcessGuard {
        &self.guard
    }
}

pub struct CollectorState {
    pub collector: Mutex<SystemCollector>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_doctor_core::models::ProcessCategory;

    #[test]
    fn test_system_collector_initialization() {
        let collector = SystemCollector::new();
        assert!(!collector.guard().is_protected(999999, "harmless_app.exe"));
        assert!(collector.guard().is_protected(4, "System"));
    }

    #[test]
    fn test_system_collector_get_process_list_populates_category_and_protection() {
        let collector = SystemCollector::new();
        let processes = collector.get_process_list();
        
        assert!(!processes.is_empty(), "Process list should not be empty on host system");
        
        // Check that at least one process has a valid category
        let has_categorized = processes.iter().any(|p| {
            matches!(
                p.category,
                ProcessCategory::System
                    | ProcessCategory::BackgroundService
                    | ProcessCategory::Application
                    | ProcessCategory::BrowserEngine
            )
        });
        assert!(has_categorized, "All processes must be assigned valid categories");

        // Check that core system processes have is_protected = true
        let has_protected = processes.iter().any(|p| p.is_protected);
        assert!(has_protected, "Running system must contain protected processes");

        // Verify descending sort order
        for i in 1..processes.len() {
            assert!(
                processes[i - 1].cpu_percent >= processes[i].cpu_percent
                    || (processes[i - 1].cpu_percent == processes[i].cpu_percent
                        && processes[i - 1].memory_bytes >= processes[i].memory_bytes),
                "Processes must be sorted descending by CPU percent then memory"
            );
        }
    }

    #[test]
    fn test_system_collector_top_processes_conversion() {
        let collector = SystemCollector::new();
        let top5 = collector.get_top_processes(5);
        assert!(top5.len() <= 5);
        for proc in &top5 {
            assert!(!proc.name.is_empty());
        }
    }

    #[test]
    fn test_system_collector_take_snapshot() {
        let mut collector = SystemCollector::new();
        let snapshot = collector.take_snapshot();
        assert!(!snapshot.cpu.model_name.is_empty());
        assert!(snapshot.memory.total_bytes > 0);
        assert!(!snapshot.os.name.is_empty());
    }
}

