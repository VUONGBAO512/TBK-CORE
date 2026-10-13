pub mod diagnostics;
pub mod knowledge;
pub mod models;

pub use diagnostics::{DiagnosticEngine, DiagnosticHistory};
pub use knowledge::{KnowledgeArticle, KnowledgeBase};
pub use models::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_engine_detects_low_disk() {
        let mut engine = DiagnosticEngine::new();
        let cpu = CpuSnapshot {
            model_name: "Test CPU".to_string(),
            logical_cores: 8,
            physical_cores: 4,
            global_usage_percent: 25.0,
            cores: vec![],
        };
        let mem = MemorySnapshot {
            total_bytes: 16 * 1024 * 1024 * 1024,
            used_bytes: 8 * 1024 * 1024 * 1024,
            available_bytes: 8 * 1024 * 1024 * 1024,
            usage_percent: 50.0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
        };
        let disks = vec![DiskSnapshot {
            name: "C:".to_string(),
            mount_point: "C:".to_string(),
            filesystem: "NTFS".to_string(),
            total_bytes: 100 * 1024 * 1024 * 1024,
            available_bytes: 1 * 1024 * 1024 * 1024, // 1% free -> critical
            used_bytes: 99 * 1024 * 1024 * 1024,
            usage_percent: 99.0,
            is_removable: false,
        }];
        let temperatures = vec![];
        let processes = vec![];

        let findings = engine.evaluate(&cpu, &mem, &disks, &temperatures, &processes);
        assert!(findings.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY"));
        assert!(findings.iter().any(|f| f.rule_id == "SENSOR_TEMP_UNAVAILABLE"));
    }

    #[test]
    fn test_diagnostic_sustained_cpu() {
        let mut engine = DiagnosticEngine::new();
        let cpu_high = CpuSnapshot {
            model_name: "Test CPU".to_string(),
            logical_cores: 8,
            physical_cores: 4,
            global_usage_percent: 95.0,
            cores: vec![],
        };
        let mem = MemorySnapshot {
            total_bytes: 16 * 1024 * 1024 * 1024,
            used_bytes: 4 * 1024 * 1024 * 1024,
            available_bytes: 12 * 1024 * 1024 * 1024,
            usage_percent: 25.0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
        };
        let disks = vec![DiskSnapshot {
            name: "C:".to_string(),
            mount_point: "C:".to_string(),
            filesystem: "NTFS".to_string(),
            total_bytes: 100 * 1024 * 1024 * 1024,
            available_bytes: 50 * 1024 * 1024 * 1024,
            used_bytes: 50 * 1024 * 1024 * 1024,
            usage_percent: 50.0,
            is_removable: false,
        }];
        let temps = vec![TemperatureSensor {
            label: "CPU Package".to_string(),
            celsius: Some(45.0),
            status: "available".to_string(),
        }];
        let procs = vec![];

        // Sample 1 & 2: not sustained yet
        let f1 = engine.evaluate(&cpu_high, &mem, &disks, &temps, &procs);
        assert!(!f1.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED"));

        let f2 = engine.evaluate(&cpu_high, &mem, &disks, &temps, &procs);
        assert!(!f2.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED"));

        // Sample 3: now sustained >= 3 samples
        let f3 = engine.evaluate(&cpu_high, &mem, &disks, &temps, &procs);
        assert!(f3.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED"));
    }

    #[test]
    fn test_knowledge_base_search() {
        let kb = KnowledgeBase::new();
        let results_vi = kb.search("máy bị đầy ổ đĩa");
        assert!(!results_vi.is_empty());
        assert_eq!(results_vi[0].id, "disk_troubleshooting");

        let results_en = kb.search("high cpu lag");
        assert!(!results_en.is_empty());
        assert_eq!(results_en[0].id, "cpu_troubleshooting");
    }
}
