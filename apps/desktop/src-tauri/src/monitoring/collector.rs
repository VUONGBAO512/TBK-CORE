use chrono::Utc;
use pc_doctor_core::models::{
    CpuCoreInfo, CpuSnapshot, DiskSnapshot, MemorySnapshot, OsSnapshot,
    ProcessInfo, SystemSnapshot, TemperatureSensor,
};
use pc_doctor_core::DiagnosticEngine;
use std::sync::Mutex;
use sysinfo::{Components, Disks, System};

pub struct SystemCollector {
    sys: System,
    disks: Disks,
    components: Components,
    engine: DiagnosticEngine,
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

        Self {
            sys,
            disks,
            components,
            engine,
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

    pub fn get_top_processes(&self, limit: usize) -> Vec<ProcessInfo> {
        let mut list: Vec<ProcessInfo> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, proc)| ProcessInfo {
                pid: pid.as_u32(),
                name: proc.name().to_string_lossy().to_string(),
                cpu_percent: proc.cpu_usage(),
                memory_bytes: proc.memory(),
                status: format!("{:?}", proc.status()),
            })
            .collect();

        // Sort descending by CPU percentage, fallback to memory
        list.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
        });

        if list.len() > limit {
            list.truncate(limit);
        }

        list
    }
}

pub struct CollectorState {
    pub collector: Mutex<SystemCollector>,
}
