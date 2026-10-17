pub mod category;
pub mod error;
pub mod guard;
pub mod runaway;

pub use category::{categorize_process, normalize_proc_name, ProcessCategory};
pub use error::ProcessTerminationError;
pub use guard::{ProcessGuard, WINDOWS_PROTECTED_PROCESSES};
pub use runaway::{
    detect_runaway, detect_runaway_with_thresholds, is_runaway, is_runaway_with_thresholds,
    RunawayKind, RunawayThresholds, DEFAULT_DUAL_CPU_PERCENT, DEFAULT_DUAL_MEMORY_BYTES,
    DEFAULT_RUNAWAY_CPU_PERCENT, DEFAULT_RUNAWAY_MEMORY_BYTES,
};

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // 1. Blacklist & Safety Guard Tests
    // ==========================================

    #[test]
    fn test_guard_blocks_pid_zero() {
        let guard = ProcessGuard::with_self_pid(9999);
        let res = guard.validate_termination(0, "System Idle Process");
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, reason, .. }) => {
                assert_eq!(pid, 0);
                assert!(!reason.is_empty());
            }
            _ => panic!("Expected ProtectedSystemProcess error for PID 0"),
        }
    }

    #[test]
    fn test_guard_blocks_pid_four_system() {
        let guard = ProcessGuard::with_self_pid(9999);
        let res = guard.validate_termination(4, "System");
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, .. }) => {
                assert_eq!(pid, 4);
            }
            _ => panic!("Expected ProtectedSystemProcess error for PID 4"),
        }
    }

    #[test]
    fn test_guard_blocks_all_pids_less_than_or_equal_to_four() {
        let guard = ProcessGuard::with_self_pid(9999);
        for pid in 0..=4 {
            let res = guard.validate_termination(pid, "test_proc");
            assert!(
                matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })),
                "PID {} must be protected",
                pid
            );
            assert!(guard.is_protected(pid, "test_proc"));
        }
    }

    #[test]
    fn test_guard_blocks_self_pid() {
        let self_pid = 12345;
        let guard = ProcessGuard::with_self_pid(self_pid);
        let res = guard.validate_termination(self_pid, "custom_runner.exe");
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, reason, .. }) => {
                assert_eq!(pid, self_pid);
                assert!(reason.contains("own application process"));
            }
            _ => panic!("Expected ProtectedSystemProcess error for self PID"),
        }
        assert!(guard.is_protected(self_pid, "custom_runner.exe"));
    }

    #[test]
    fn test_guard_blocks_mandatory_windows_blacklist() {
        let guard = ProcessGuard::with_self_pid(9999);
        let mandatory_names = [
            "csrss.exe",
            "lsass.exe",
            "services.exe",
            "explorer.exe",
            "smss.exe",
            "wininit.exe",
            "winlogon.exe",
            "svchost.exe",
            "dwm.exe",
        ];

        for (i, name) in mandatory_names.iter().enumerate() {
            let test_pid = 1000 + i as u32;
            let res = guard.validate_termination(test_pid, name);
            match res {
                Err(ProcessTerminationError::ProtectedSystemProcess { pid, name: proc_name, reason }) => {
                    assert_eq!(pid, test_pid);
                    assert_eq!(proc_name, *name);
                    assert!(!reason.is_empty(), "Reason for {} must not be empty", name);
                }
                _ => panic!("Process {} must be protected by blacklist", name),
            }
            assert!(guard.is_protected(test_pid, name), "is_protected must be true for {}", name);
        }
    }

    #[test]
    fn test_guard_blocks_blacklist_case_insensitive_and_without_extension() {
        let guard = ProcessGuard::with_self_pid(9999);
        let variations = [
            "CSRSS.EXE",
            "csrss",
            "Explorer.exe",
            "EXPLORER",
            "Dwm.Exe",
            "DWM",
            "SvChOsT.eXe",
            "svchost",
            "LSASS.EXE",
            "Services.exe",
        ];

        for name in variations {
            let res = guard.validate_termination(8000, name);
            assert!(
                matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })),
                "Variation '{}' must be protected",
                name
            );
        }
    }

    #[test]
    fn test_guard_approves_unprotected_applications() {
        let guard = ProcessGuard::with_self_pid(9999);
        let safe_apps = [
            "notepad.exe",
            "chrome.exe",
            "code.exe",
            "spotify.exe",
            "discord.exe",
            "steam.exe",
            "game.exe",
            "photoshop.exe",
        ];

        for (i, name) in safe_apps.iter().enumerate() {
            let test_pid = 2000 + i as u32;
            let res = guard.validate_termination(test_pid, name);
            assert!(res.is_ok(), "Unprotected process '{}' should be approved", name);
            assert!(!guard.is_protected(test_pid, name), "is_protected should be false for '{}'", name);
            assert!(guard.protection_reason(test_pid, name).is_none());
        }
    }

    // ==========================================
    // 2. Process Categorization Tests
    // ==========================================

    #[test]
    fn test_categorize_system_processes() {
        assert_eq!(categorize_process(0, "System Idle Process", None), ProcessCategory::System);
        assert_eq!(categorize_process(4, "System", None), ProcessCategory::System);
        assert_eq!(categorize_process(100, "csrss.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(101, "CSRSS", None), ProcessCategory::System);
        assert_eq!(categorize_process(102, "smss.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(103, "wininit.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(104, "winlogon.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(105, "dwm.exe", None), ProcessCategory::System);
        assert_eq!(categorize_process(106, "explorer.exe", None), ProcessCategory::System);
    }

    #[test]
    fn test_categorize_browser_engines() {
        assert_eq!(categorize_process(200, "chrome.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(201, "CHROME.EXE", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(202, "msedge.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(203, "msedgewebview2.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(204, "firefox.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(205, "brave.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(206, "opera.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(207, "vivaldi.exe", None), ProcessCategory::BrowserEngine);
        assert_eq!(categorize_process(208, "arc.exe", None), ProcessCategory::BrowserEngine);

        // Path-based browser recognition
        assert_eq!(
            categorize_process(
                209,
                "browser_worker",
                Some(r"C:\Program Files\Google\Chrome\Application\chrome.exe")
            ),
            ProcessCategory::BrowserEngine
        );
    }

    #[test]
    fn test_categorize_background_services() {
        assert_eq!(categorize_process(300, "svchost.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(301, "spoolsv.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(302, "taskhostw.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(303, "sihost.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(304, "RuntimeBroker.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(305, "SearchIndexer.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(306, "SecurityHealthService.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(307, "conhost.exe", None), ProcessCategory::BackgroundService);

        // Suffix-based service recognition
        assert_eq!(categorize_process(308, "custom_agent.exe", None), ProcessCategory::BackgroundService);
        assert_eq!(categorize_process(309, "my_backup_daemon", None), ProcessCategory::BackgroundService);
    }

    #[test]
    fn test_categorize_applications() {
        assert_eq!(categorize_process(400, "code.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(401, "notepad.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(402, "discord.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(403, "spotify.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(404, "steam.exe", None), ProcessCategory::Application);
        assert_eq!(categorize_process(405, "slack.exe", None), ProcessCategory::Application);
    }

    // ==========================================
    // 3. Runaway Process Detection Tests
    // ==========================================

    #[test]
    fn test_runaway_healthy_process() {
        let cpu = 15.0;
        let mem = 250 * 1024 * 1024; // 250 MB
        assert!(!is_runaway(cpu, mem));
        assert_eq!(detect_runaway(cpu, mem), None);
    }

    #[test]
    fn test_runaway_cpu_boundary() {
        let mem = 100 * 1024 * 1024;
        // Below 90% is healthy
        assert!(!is_runaway(89.9, mem));
        assert_eq!(detect_runaway(89.9, mem), None);

        // >= 90% is runaway CPU
        assert!(is_runaway(90.0, mem));
        assert_eq!(detect_runaway(90.0, mem), Some(RunawayKind::Cpu { cpu_percent: 90.0 }));

        assert!(is_runaway(99.5, mem));
        assert_eq!(detect_runaway(99.5, mem), Some(RunawayKind::Cpu { cpu_percent: 99.5 }));
    }

    #[test]
    fn test_runaway_memory_boundary() {
        let cpu = 5.0;
        let two_gb = 2 * 1024 * 1024 * 1024; // 2,147,483,648 bytes

        // Exactly 2 GB is not > 2 GB
        assert!(!is_runaway(cpu, two_gb));
        assert_eq!(detect_runaway(cpu, two_gb), None);

        // > 2 GB is runaway Memory
        let two_gb_plus_one = two_gb + 1;
        assert!(is_runaway(cpu, two_gb_plus_one));
        assert_eq!(
            detect_runaway(cpu, two_gb_plus_one),
            Some(RunawayKind::Memory { memory_bytes: two_gb_plus_one })
        );

        let three_gb = 3 * 1024 * 1024 * 1024;
        assert!(is_runaway(cpu, three_gb));
    }

    #[test]
    fn test_runaway_dual_pressure_detection() {
        let dual_mem = 1_610_612_736; // 1.5 GB
        let dual_cpu = 70.0;

        // Both conditions met: Dual Pressure Runaway
        assert!(is_runaway(dual_cpu, dual_mem));
        assert_eq!(
            detect_runaway(dual_cpu, dual_mem),
            Some(RunawayKind::DualPressure {
                cpu_percent: dual_cpu,
                memory_bytes: dual_mem,
            })
        );

        // High CPU (75%) and High RAM (1.8 GB) -> Dual Pressure
        assert!(is_runaway(75.0, 1_800_000_000));
        assert_eq!(
            detect_runaway(75.0, 1_800_000_000),
            Some(RunawayKind::DualPressure {
                cpu_percent: 75.0,
                memory_bytes: 1_800_000_000,
            })
        );

        // Only CPU high (75%), RAM low (500 MB) -> Not runaway
        assert!(!is_runaway(75.0, 500 * 1024 * 1024));
        assert_eq!(detect_runaway(75.0, 500 * 1024 * 1024), None);

        // Only RAM high (1.8 GB), CPU low (30%) -> Not runaway (below 2GB single threshold)
        assert!(!is_runaway(30.0, 1_800_000_000));
        assert_eq!(detect_runaway(30.0, 1_800_000_000), None);
    }

    // ==========================================
    // 4. Serde & Serialization Tests
    // ==========================================

    #[test]
    fn test_error_json_serialization() {
        let err = ProcessTerminationError::ProtectedSystemProcess {
            pid: 4,
            name: "System".to_string(),
            reason: "Windows kernel".to_string(),
        };

        let json = serde_json::to_string(&err).expect("Serialization failed");
        assert!(json.contains(r#""error_type":"ProtectedSystemProcess""#));
        assert!(json.contains(r#""pid":4"#));
        assert!(json.contains(r#""name":"System""#));

        let deserialized: ProcessTerminationError =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(err, deserialized);
    }

    #[test]
    fn test_category_json_serialization() {
        let cat = ProcessCategory::BackgroundService;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, r#""background_service""#);

        let cat_browser = ProcessCategory::BrowserEngine;
        assert_eq!(serde_json::to_string(&cat_browser).unwrap(), r#""browser_engine""#);
    }
}
