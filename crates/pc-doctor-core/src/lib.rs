pub mod diagnostics;
pub mod knowledge;
pub mod models;
pub mod processes;
pub mod storage;

pub use diagnostics::{DiagnosticEngine, DiagnosticHistory};
pub use knowledge::{KnowledgeArticle, KnowledgeBase};
pub use models::*;
pub use processes::{
    categorize_process, detect_runaway, detect_runaway_with_thresholds, is_runaway,
    is_runaway_with_thresholds, normalize_proc_name, ProcessGuard, ProcessTerminationError,
    RunawayKind, RunawayThresholds, WINDOWS_PROTECTED_PROCESSES,
};
pub use storage::{
    DefaultWindowsPathResolver, SafetyGuard, SafetyViolation, StorageEngine, StoragePathResolver,
    StoragePurger, StorageScanner, TargetCatalog,
};

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
    fn test_knowledge_base_search_and_retrieval() {
        let kb = KnowledgeBase::new();

        // Existing articles
        let results_vi = kb.search("máy bị đầy ổ đĩa");
        assert!(!results_vi.is_empty());
        assert_eq!(results_vi[0].id, "disk_troubleshooting");

        let results_en = kb.search("high cpu lag");
        assert!(!results_en.is_empty());
        assert_eq!(results_en[0].id, "cpu_troubleshooting");

        // New storage cleanup article
        let storage_vi = kb.search("dọn rác và thư mục tạm an toàn");
        assert!(!storage_vi.is_empty());
        assert!(storage_vi.iter().any(|a| a.id == "storage_cleanup_safety"));

        let storage_en = kb.search("safe storage temp cleanup dry-run");
        assert!(!storage_en.is_empty());
        assert!(storage_en.iter().any(|a| a.id == "storage_cleanup_safety"));

        // New process management safety article
        let proc_vi = kb.search("tắt tiến trình hệ thống danh sách đen csrss");
        assert!(!proc_vi.is_empty());
        assert!(proc_vi.iter().any(|a| a.id == "process_management_safety"));

        let proc_en = kb.search("kill process blacklist guardrails lsass");
        assert!(!proc_en.is_empty());
        assert!(proc_en.iter().any(|a| a.id == "process_management_safety"));

        // Direct get_by_id lookup
        assert!(kb.get_by_id("storage_cleanup_safety").is_some());
        assert!(kb.get_by_id("process_management_safety").is_some());
        assert!(kb.get_by_id("non_existent").is_none());
    }

    #[test]
    fn test_storage_models_serde() {
        let target = CleanableTarget::new(
            "temp_user",
            "User Temporary Files",
            "Tệp tạm người dùng",
            "Cached application files",
            "Tệp đệm ứng dụng",
            "C:\\Users\\User\\AppData\\Local\\Temp",
            120,
            104857600, // 100 MB
            true,
        );

        let report = StorageScanReport::new(vec![target.clone()]);
        assert_eq!(report.total_files, 120);
        assert_eq!(report.total_reclaimable_bytes, 104857600);

        let serialized_report = serde_json::to_string(&report).expect("Serialize report");
        let deserialized_report: StorageScanReport =
            serde_json::from_str(&serialized_report).expect("Deserialize report");
        assert_eq!(deserialized_report.total_files, 120);
        assert_eq!(deserialized_report.targets.len(), 1);
        assert_eq!(deserialized_report.targets[0].id, "temp_user");

        let clean_req = CleanRequest::new(vec!["temp_user".to_string()], true);
        let serialized_req = serde_json::to_string(&clean_req).expect("Serialize clean req");
        let deserialized_req: CleanRequest =
            serde_json::from_str(&serialized_req).expect("Deserialize clean req");
        assert!(deserialized_req.dry_run);
        assert_eq!(deserialized_req.target_ids, vec!["temp_user"]);

        let summary = StorageCleanSummary {
            dry_run: true,
            bytes_freed: 50000000,
            files_deleted: 60,
            files_skipped_locked: 2,
            targets_processed: vec!["temp_user".to_string()],
            errors: vec![],
        };
        let serialized_summary = serde_json::to_string(&summary).expect("Serialize summary");
        let deserialized_summary: StorageCleanSummary =
            serde_json::from_str(&serialized_summary).expect("Deserialize summary");
        assert_eq!(deserialized_summary.bytes_freed, 50000000);
        assert_eq!(deserialized_summary.files_skipped_locked, 2);
    }

    #[test]
    fn test_process_models_and_conversion() {
        let proc_item = ProcessItem::new(
            1234,
            "chrome.exe",
            12.5,
            250 * 1024 * 1024,
            "Run",
            ProcessCategory::BrowserEngine,
            false,
        );

        let serialized = serde_json::to_string(&proc_item).expect("Serialize proc item");
        assert!(serialized.contains("\"category\":\"browser_engine\""));

        let deserialized: ProcessItem =
            serde_json::from_str(&serialized).expect("Deserialize proc item");
        assert_eq!(deserialized.category, ProcessCategory::BrowserEngine);
        assert_eq!(deserialized.pid, 1234);

        // Test conversion into ProcessInfo for diagnostic evaluation
        let proc_info: ProcessInfo = proc_item.into();
        assert_eq!(proc_info.pid, 1234);
        assert_eq!(proc_info.name, "chrome.exe");
        assert_eq!(proc_info.cpu_percent, 12.5);

        // Test Category Deserialization Aliases
        let cat_app: ProcessCategory = serde_json::from_str("\"app\"").unwrap();
        assert_eq!(cat_app, ProcessCategory::Application);
        let cat_svc: ProcessCategory = serde_json::from_str("\"service\"").unwrap();
        assert_eq!(cat_svc, ProcessCategory::BackgroundService);
        let cat_sys: ProcessCategory = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(cat_sys, ProcessCategory::System);

        // Terminate result
        let term_ok = TerminateResult::success(1234, "chrome.exe", "Process terminated");
        assert!(term_ok.success);
        let term_fail = TerminateResult::failure(4, "csrss.exe", "Protected process cannot be terminated");
        assert!(!term_fail.success);
    }
}
