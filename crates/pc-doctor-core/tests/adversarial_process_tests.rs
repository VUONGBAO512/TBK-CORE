use pc_doctor_core::models::ProcessCategory;
use pc_doctor_core::processes::{
    categorize_process, detect_runaway, detect_runaway_with_thresholds, is_runaway,
    is_runaway_with_thresholds, normalize_proc_name, ProcessGuard, ProcessTerminationError,
    RunawayKind, RunawayThresholds, WINDOWS_PROTECTED_PROCESSES,
};

// ============================================================================
// SUITE 1: Mandatory Protected Processes Verification
// ============================================================================

#[test]
fn test_all_nine_mandatory_protected_processes() {
    let guard = ProcessGuard::with_self_pid(8888);

    let mandatory_targets = [
        ("csrss.exe", "Client Server Runtime Subsystem"),
        ("lsass.exe", "Local Security Authority Subsystem Service"),
        ("services.exe", "Service Control Manager"),
        ("explorer.exe", "Windows Explorer"),
        ("smss.exe", "Session Manager Subsystem"),
        ("wininit.exe", "Windows Initialization Process"),
        ("winlogon.exe", "Windows Logon Process"),
        ("svchost.exe", "Service Host"),
        ("dwm.exe", "Desktop Window Manager"),
    ];

    for (idx, (proc_name, expected_token)) in mandatory_targets.iter().enumerate() {
        let test_pid = 2000 + idx as u32;

        // 1. Direct validation check
        let res = guard.validate_termination(test_pid, proc_name);
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason }) => {
                assert_eq!(pid, test_pid, "PID mismatch for {}", proc_name);
                assert_eq!(name, *proc_name, "Name mismatch for {}", proc_name);
                assert!(
                    reason.contains(expected_token),
                    "Reason for '{}' did not contain '{}'. Full reason: {}",
                    proc_name,
                    expected_token,
                    reason
                );
            }
            Ok(_) => panic!("CRITICAL FAILURE: Protected process '{}' was approved for termination!", proc_name),
            Err(other) => panic!("Incorrect error variant returned for '{}': {:?}", proc_name, other),
        }

        // 2. Query helper functions
        assert!(
            guard.is_protected(test_pid, proc_name),
            "is_protected must be true for {}",
            proc_name
        );
        let reason_opt = guard.protection_reason(test_pid, proc_name);
        assert!(
            reason_opt.is_some(),
            "protection_reason must be Some for {}",
            proc_name
        );
        assert!(
            reason_opt.unwrap().contains(expected_token),
            "protection_reason content mismatch for {}",
            proc_name
        );
    }
}

#[test]
fn test_all_catalog_protected_processes_rejected() {
    let guard = ProcessGuard::with_self_pid(9999);

    assert!(
        WINDOWS_PROTECTED_PROCESSES.len() >= 9,
        "Protected processes list must contain at least 9 processes"
    );

    for (idx, &(proc_name, reason_token)) in WINDOWS_PROTECTED_PROCESSES.iter().enumerate() {
        let test_pid = 5000 + idx as u32;
        let res = guard.validate_termination(test_pid, proc_name);
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason }) => {
                assert_eq!(pid, test_pid);
                assert_eq!(name, proc_name);
                assert_eq!(reason, reason_token);
            }
            Ok(_) => panic!("Failed to protect process: {}", proc_name),
            Err(e) => panic!("Wrong error variant for {}: {:?}", proc_name, e),
        }
    }
}

// ============================================================================
// SUITE 2: Casing Variations & Extension Stripping
// ============================================================================

#[test]
fn test_casing_variations_and_extensions() {
    let guard = ProcessGuard::with_self_pid(7777);

    // Variations of mandatory protected processes
    let variations = [
        // CSRSS variations
        ("CSRSS.EXE", "csrss"),
        ("csrss", "csrss"),
        ("CsRss.Exe", "csrss"),
        ("CSRSS", "csrss"),
        ("  csrss.exe  ", "csrss"),
        // EXPLORER variations
        ("Explorer.exe", "explorer"),
        ("EXPLORER", "explorer"),
        ("ExPlOrEr.EXE", "explorer"),
        ("explorer", "explorer"),
        ("\texplorer.exe\n", "explorer"),
        // LSASS variations
        ("LSASS.EXE", "lsass"),
        ("lsass", "lsass"),
        ("Lsass.Exe", "lsass"),
        ("LSASS", "lsass"),
        // SERVICES variations
        ("Services.exe", "services"),
        ("SERVICES", "services"),
        ("SERVICES.EXE", "services"),
        ("services", "services"),
        // SMSS variations
        ("SMSS.EXE", "smss"),
        ("smss", "smss"),
        ("Smss.Exe", "smss"),
        // WININIT variations
        ("WININIT.EXE", "wininit"),
        ("wininit", "wininit"),
        ("WinInit.exe", "wininit"),
        // WINLOGON variations
        ("WINLOGON.EXE", "winlogon"),
        ("winlogon", "winlogon"),
        ("WinLogon.exe", "winlogon"),
        // SVCHOST variations
        ("SVCHOST.EXE", "svchost"),
        ("svchost", "svchost"),
        ("SvChOsT.eXe", "svchost"),
        // DWM variations
        ("DWM.EXE", "dwm"),
        ("dwm", "dwm"),
        ("Dwm.Exe", "dwm"),
    ];

    for (input_name, base_token) in variations {
        let res = guard.validate_termination(3456, input_name);
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason }) => {
                assert_eq!(pid, 3456);
                assert_eq!(name, input_name);
                assert!(
                    !reason.is_empty(),
                    "Reason for '{}' (base: '{}') should not be empty",
                    input_name,
                    base_token
                );
            }
            Ok(_) => panic!(
                "CRITICAL BYPASS: Process variation '{}' (base: '{}') was not blocked!",
                input_name, base_token
            ),
            Err(other) => panic!("Unexpected error type for '{}': {:?}", input_name, other),
        }
        assert!(
            guard.is_protected(3456, input_name),
            "is_protected returned false for '{}'",
            input_name
        );
    }
}

#[test]
fn test_unprotected_application_names_are_approved() {
    let guard = ProcessGuard::with_self_pid(1000);

    let allowed = [
        "notepad.exe",
        "NOTEPAD.EXE",
        "notepad",
        "chrome.exe",
        "firefox.exe",
        "vlc.exe",
        "code.exe",
        "steam.exe",
        "discord.exe",
        "csrss_simulator.exe", // Substring with different token
        "not_explorer.exe",
        "my_svchost_tool.exe",
        "csrss.exe.exe", // Double extension is treated as distinct process name
    ];

    for name in allowed {
        let res = guard.validate_termination(6000, name);
        assert!(
            res.is_ok(),
            "Application '{}' should be allowed to terminate, but got: {:?}",
            name,
            res
        );
        assert!(
            !guard.is_protected(6000, name),
            "is_protected should be false for '{}'",
            name
        );
        assert!(guard.protection_reason(6000, name).is_none());
    }
}

// ============================================================================
// SUITE 3: PID Boundaries Verification
// ============================================================================

#[test]
fn test_pid_boundaries_zero_to_four() {
    let guard = ProcessGuard::with_self_pid(9999);

    // Test PID 0 through 4 with various names, including empty strings and bogus names
    for pid in 0..=4 {
        // With generic name
        let res1 = guard.validate_termination(pid, "arbitrary_user_program.exe");
        match res1 {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid: p, name, reason }) => {
                assert_eq!(p, pid);
                assert_eq!(name, "arbitrary_user_program.exe");
                assert!(reason.contains("kernel process"));
            }
            _ => panic!("PID {} must be protected regardless of process name", pid),
        }

        // With empty string name - should auto-populate canonical system name
        let res2 = guard.validate_termination(pid, "");
        match res2 {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid: p, name, .. }) => {
                assert_eq!(p, pid);
                if pid == 0 {
                    assert_eq!(name, "System Idle Process");
                } else {
                    assert_eq!(name, "System");
                }
            }
            _ => panic!("PID {} with empty name must be protected", pid),
        }

        assert!(guard.is_protected(pid, "test"));
        assert!(guard.protection_reason(pid, "test").is_some());
    }
}

#[test]
fn test_pid_five_boundary_transitions_to_unprotected() {
    let guard = ProcessGuard::with_self_pid(9999);

    // PID 5 is right above the <= 4 kernel boundary
    let res = guard.validate_termination(5, "user_tool.exe");
    assert!(res.is_ok(), "PID 5 with non-blacklisted name should be approved");
    assert!(!guard.is_protected(5, "user_tool.exe"));

    // But PID 5 with blacklisted name must still be protected by name check
    let res_blacklisted = guard.validate_termination(5, "csrss.exe");
    assert!(
        matches!(res_blacklisted, Err(ProcessTerminationError::ProtectedSystemProcess { .. })),
        "PID 5 with blacklisted name csrss.exe must be protected"
    );
}

#[test]
fn test_self_pid_protection_boundary() {
    let test_self_pid = 43210;
    let guard = ProcessGuard::with_self_pid(test_self_pid);

    assert_eq!(guard.self_pid(), test_self_pid);

    // Attempt to terminate self PID with various process names
    let test_names = [
        "pc-doctor.exe",
        "TBK_Core.exe",
        "any_app.exe",
        "worker.exe",
        "",
    ];

    for name in test_names {
        let res = guard.validate_termination(test_self_pid, name);
        match res {
            Err(ProcessTerminationError::ProtectedSystemProcess { pid, name: proc_name, reason }) => {
                assert_eq!(pid, test_self_pid);
                assert_eq!(proc_name, name);
                assert!(
                    reason.contains("own application process"),
                    "Self PID error reason mismatch: {}",
                    reason
                );
            }
            Ok(_) => panic!("CRITICAL: Self PID {} was allowed to terminate itself!", test_self_pid),
            Err(other) => panic!("Wrong error for self PID: {:?}", other),
        }
        assert!(guard.is_protected(test_self_pid, name));
    }

    // Default guard using real OS PID
    let default_guard = ProcessGuard::default();
    let current_os_pid = std::process::id();
    assert_eq!(default_guard.self_pid(), current_os_pid);
    assert!(default_guard.is_protected(current_os_pid, "running_test.exe"));
}

#[test]
fn test_self_pid_at_kernel_boundary() {
    // Edge case: what if self_pid was configured to 0 or 4?
    // PID <= 4 check fires first and protects it.
    let guard_zero = ProcessGuard::with_self_pid(0);
    assert!(guard_zero.is_protected(0, "app.exe"));

    let guard_four = ProcessGuard::with_self_pid(4);
    assert!(guard_four.is_protected(4, "app.exe"));
}

#[test]
fn test_large_pid_boundary() {
    let guard = ProcessGuard::with_self_pid(500);

    // u32::MAX boundary
    let max_pid = u32::MAX;
    let res_ok = guard.validate_termination(max_pid, "safe_app.exe");
    assert!(res_ok.is_ok());

    let res_protected = guard.validate_termination(max_pid, "lsass.exe");
    assert!(matches!(res_protected, Err(ProcessTerminationError::ProtectedSystemProcess { .. })));
}

// ============================================================================
// SUITE 4: Runaway Criteria & Mathematical Boundaries
// ============================================================================

#[test]
fn test_runaway_cpu_boundary_precision() {
    let safe_mem = 500 * 1024 * 1024; // 500 MB (safely below 1.5GB / 2GB)

    // 0.0% CPU -> Healthy
    assert!(!is_runaway(0.0, safe_mem));
    assert_eq!(detect_runaway(0.0, safe_mem), None);

    // Negative CPU -> Healthy
    assert!(!is_runaway(-10.0, safe_mem));
    assert_eq!(detect_runaway(-10.0, safe_mem), None);

    // NaN CPU -> Not runaway (IEEE 754 NaN comparison is false)
    assert!(!is_runaway(f32::NAN, safe_mem));
    assert_eq!(detect_runaway(f32::NAN, safe_mem), None);

    // 50.0% CPU -> Healthy
    assert!(!is_runaway(50.0, safe_mem));
    assert_eq!(detect_runaway(50.0, safe_mem), None);

    // 89.89% CPU -> Healthy
    assert!(!is_runaway(89.89, safe_mem));
    assert_eq!(detect_runaway(89.89, safe_mem), None);

    // 89.9% CPU -> Healthy (STRICT REQUIREMENT: 89.9% is below 90% threshold)
    assert!(
        !is_runaway(89.9, safe_mem),
        "89.9% CPU must NOT trigger runaway detection"
    );
    assert_eq!(detect_runaway(89.9, safe_mem), None);

    // 89.999% CPU -> Healthy
    assert!(!is_runaway(89.999, safe_mem));
    assert_eq!(detect_runaway(89.999, safe_mem), None);

    // 90.0% CPU -> Runaway (STRICT REQUIREMENT: 90.0% triggers CPU runaway)
    assert!(
        is_runaway(90.0, safe_mem),
        "90.0% CPU MUST trigger runaway detection"
    );
    assert_eq!(
        detect_runaway(90.0, safe_mem),
        Some(RunawayKind::Cpu { cpu_percent: 90.0 })
    );

    // 90.001% CPU -> Runaway
    assert!(is_runaway(90.001, safe_mem));
    assert_eq!(
        detect_runaway(90.001, safe_mem),
        Some(RunawayKind::Cpu { cpu_percent: 90.001 })
    );

    // 100.0% CPU -> Runaway
    assert!(is_runaway(100.0, safe_mem));
    assert_eq!(
        detect_runaway(100.0, safe_mem),
        Some(RunawayKind::Cpu { cpu_percent: 100.0 })
    );

    // Multi-core aggregation > 100% (e.g. 8 cores pegged = 800%) -> Runaway
    assert!(is_runaway(800.0, safe_mem));
    assert_eq!(
        detect_runaway(800.0, safe_mem),
        Some(RunawayKind::Cpu { cpu_percent: 800.0 })
    );

    // Positive Infinity CPU -> Runaway
    assert!(is_runaway(f32::INFINITY, safe_mem));
    assert_eq!(
        detect_runaway(f32::INFINITY, safe_mem),
        Some(RunawayKind::Cpu { cpu_percent: f32::INFINITY })
    );
}

#[test]
fn test_runaway_ram_boundary_precision() {
    let safe_cpu = 10.0; // 10% CPU (safely below 70% dual / 90% single)
    let two_gb = 2 * 1024 * 1024 * 1024; // 2,147,483,648 bytes

    // 0 bytes -> Healthy
    assert!(!is_runaway(safe_cpu, 0));
    assert_eq!(detect_runaway(safe_cpu, 0), None);

    // 1 GB -> Healthy
    let one_gb = 1024 * 1024 * 1024;
    assert!(!is_runaway(safe_cpu, one_gb));
    assert_eq!(detect_runaway(safe_cpu, one_gb), None);

    // 2 GB - 1 byte -> Healthy
    let two_gb_minus_one = two_gb - 1;
    assert!(!is_runaway(safe_cpu, two_gb_minus_one));
    assert_eq!(detect_runaway(safe_cpu, two_gb_minus_one), None);

    // Exactly 2 GB (2,147,483,648 bytes) -> Healthy (threshold is strictly > 2GB)
    assert!(
        !is_runaway(safe_cpu, two_gb),
        "Exactly 2GB (2,147,483,648 bytes) must NOT exceed threshold"
    );
    assert_eq!(detect_runaway(safe_cpu, two_gb), None);

    // 2 GB + 1 byte (2,147,483,649 bytes) -> Runaway Memory
    let two_gb_plus_one = two_gb + 1;
    assert!(
        is_runaway(safe_cpu, two_gb_plus_one),
        "2GB + 1 byte (2,147,483,649 bytes) MUST trigger memory runaway"
    );
    assert_eq!(
        detect_runaway(safe_cpu, two_gb_plus_one),
        Some(RunawayKind::Memory { memory_bytes: two_gb_plus_one })
    );

    // 4 GB -> Runaway Memory
    let four_gb = 4 * 1024 * 1024 * 1024;
    assert!(is_runaway(safe_cpu, four_gb));
    assert_eq!(
        detect_runaway(safe_cpu, four_gb),
        Some(RunawayKind::Memory { memory_bytes: four_gb })
    );

    // u64::MAX -> Runaway Memory
    assert!(is_runaway(safe_cpu, u64::MAX));
    assert_eq!(
        detect_runaway(safe_cpu, u64::MAX),
        Some(RunawayKind::Memory { memory_bytes: u64::MAX })
    );
}

#[test]
fn test_runaway_dual_pressure_matrix() {
    let dual_cpu_threshold = 70.0;
    let dual_mem_threshold = 1_610_612_736; // 1.5 GB

    // Point 1: Below both (CPU 69.9%, RAM 1.5GB - 1 byte) -> Healthy
    assert!(!is_runaway(69.9, dual_mem_threshold - 1));
    assert_eq!(detect_runaway(69.9, dual_mem_threshold - 1), None);

    // Point 2: CPU below (69.9%), RAM at 1.5 GB -> Healthy
    assert!(!is_runaway(69.9, dual_mem_threshold));
    assert_eq!(detect_runaway(69.9, dual_mem_threshold), None);

    // Point 3: CPU at 70.0%, RAM below (1.5 GB - 1 byte) -> Healthy
    assert!(!is_runaway(70.0, dual_mem_threshold - 1));
    assert_eq!(detect_runaway(70.0, dual_mem_threshold - 1), None);

    // Point 4: CPU at 70.0%, RAM at 1.5 GB -> RUNAWAY DUAL PRESSURE
    assert!(
        is_runaway(dual_cpu_threshold, dual_mem_threshold),
        "Exact dual boundary (70.0% CPU & 1.5GB RAM) must trigger DualPressure"
    );
    assert_eq!(
        detect_runaway(dual_cpu_threshold, dual_mem_threshold),
        Some(RunawayKind::DualPressure {
            cpu_percent: dual_cpu_threshold,
            memory_bytes: dual_mem_threshold,
        })
    );

    // Point 5: CPU 80.0%, RAM 1.8 GB -> RUNAWAY DUAL PRESSURE
    let high_ram = 1_800_000_000;
    assert!(is_runaway(80.0, high_ram));
    assert_eq!(
        detect_runaway(80.0, high_ram),
        Some(RunawayKind::DualPressure {
            cpu_percent: 80.0,
            memory_bytes: high_ram,
        })
    );

    // Point 6: Single threshold precedence:
    // If CPU >= 90.0%, it triggers RunawayKind::Cpu even if RAM >= 1.5GB
    assert_eq!(
        detect_runaway(95.0, dual_mem_threshold),
        Some(RunawayKind::Cpu { cpu_percent: 95.0 })
    );

    // If RAM > 2GB and CPU < 90.0% (e.g. 60.0%), it triggers RunawayKind::Memory
    let two_gb_plus = 2 * 1024 * 1024 * 1024 + 100;
    assert_eq!(
        detect_runaway(60.0, two_gb_plus),
        Some(RunawayKind::Memory { memory_bytes: two_gb_plus })
    );
}

#[test]
fn test_runaway_custom_thresholds_override() {
    let custom = RunawayThresholds {
        cpu_percent: 50.0,
        memory_bytes: 500 * 1024 * 1024,
        dual_cpu_percent: 30.0,
        dual_memory_bytes: 200 * 1024 * 1024,
    };

    // 40% CPU and 100MB RAM -> Healthy under custom
    assert!(!is_runaway_with_thresholds(40.0, 100 * 1024 * 1024, &custom));

    // 55% CPU -> Runaway Cpu under custom
    assert_eq!(
        detect_runaway_with_thresholds(55.0, 100 * 1024 * 1024, &custom),
        Some(RunawayKind::Cpu { cpu_percent: 55.0 })
    );

    // 35% CPU & 250MB RAM -> Runaway DualPressure under custom
    assert_eq!(
        detect_runaway_with_thresholds(35.0, 250 * 1024 * 1024, &custom),
        Some(RunawayKind::DualPressure {
            cpu_percent: 35.0,
            memory_bytes: 250 * 1024 * 1024,
        })
    );
}

// ============================================================================
// SUITE 5: Structured Error ProcessTerminationError Verification
// ============================================================================

#[test]
fn test_structured_error_variants_and_serialization() {
    // 1. ProtectedSystemProcess
    let err_protected = ProcessTerminationError::ProtectedSystemProcess {
        pid: 4,
        name: "System".to_string(),
        reason: "Core NT kernel process".to_string(),
    };

    let display_str = format!("{}", err_protected);
    assert!(display_str.contains("Process 'System' (PID 4) is protected"));
    assert!(display_str.contains("Core NT kernel process"));

    let json_protected = serde_json::to_string(&err_protected).expect("Serialize protected error");
    assert!(json_protected.contains(r#""error_type":"ProtectedSystemProcess""#));
    assert!(json_protected.contains(r#""pid":4"#));
    assert!(json_protected.contains(r#""name":"System""#));
    assert!(json_protected.contains(r#""reason":"Core NT kernel process""#));

    let deserialized_protected: ProcessTerminationError =
        serde_json::from_str(&json_protected).expect("Deserialize protected error");
    assert_eq!(err_protected, deserialized_protected);

    // 2. ProcessNotFound
    let err_not_found = ProcessTerminationError::ProcessNotFound { pid: 99999 };
    assert_eq!(
        format!("{}", err_not_found),
        "Process with PID 99999 was not found"
    );
    let json_not_found = serde_json::to_string(&err_not_found).unwrap();
    assert!(json_not_found.contains(r#""error_type":"ProcessNotFound""#));
    assert_eq!(
        serde_json::from_str::<ProcessTerminationError>(&json_not_found).unwrap(),
        err_not_found
    );

    // 3. AccessDenied
    let err_denied = ProcessTerminationError::AccessDenied {
        pid: 1200,
        name: "antivirus.exe".to_string(),
        os_error_code: 5,
        message: "Access is denied (5)".to_string(),
    };
    assert!(format!("{}", err_denied).contains("Access denied"));
    let json_denied = serde_json::to_string(&err_denied).unwrap();
    assert!(json_denied.contains(r#""error_type":"AccessDenied""#));
    assert!(json_denied.contains(r#""os_error_code":5"#));

    // 4. InternalError
    let err_internal = ProcessTerminationError::InternalError {
        message: "Failed to open process handle".to_string(),
    };
    let json_internal = serde_json::to_string(&err_internal).unwrap();
    assert!(json_internal.contains(r#""error_type":"InternalError""#));
}

// ============================================================================
// SUITE 6: Edge Cases & Additional Hardening Checks
// ============================================================================

#[test]
fn test_normalize_proc_name_robustness() {
    assert_eq!(normalize_proc_name("csrss.exe"), "csrss");
    assert_eq!(normalize_proc_name("CSRSS.EXE"), "csrss");
    assert_eq!(normalize_proc_name("csrss"), "csrss");
    assert_eq!(normalize_proc_name("  CSRSS.exe  "), "csrss");
    assert_eq!(normalize_proc_name("\tExplorer.EXE\r\n"), "explorer");
    assert_eq!(normalize_proc_name("dwm"), "dwm");
    assert_eq!(normalize_proc_name("System"), "system");
    assert_eq!(normalize_proc_name("System Idle Process"), "system idle process");
}

#[test]
fn test_categorize_process_robustness() {
    // System processes
    assert_eq!(categorize_process(0, "System Idle Process", None), ProcessCategory::System);
    assert_eq!(categorize_process(4, "System", None), ProcessCategory::System);
    assert_eq!(categorize_process(100, "CSRSS.EXE", None), ProcessCategory::System);
    assert_eq!(categorize_process(101, "explorer.exe", None), ProcessCategory::System);

    // Browser processes
    assert_eq!(categorize_process(200, "CHROME.EXE", None), ProcessCategory::BrowserEngine);
    assert_eq!(categorize_process(201, "msedge.exe", None), ProcessCategory::BrowserEngine);
    assert_eq!(categorize_process(202, "firefox.exe", None), ProcessCategory::BrowserEngine);

    // Background services
    assert_eq!(categorize_process(300, "svchost.exe", None), ProcessCategory::BackgroundService);
    assert_eq!(categorize_process(301, "spoolsv.exe", None), ProcessCategory::BackgroundService);

    // General applications
    assert_eq!(categorize_process(400, "notepad.exe", None), ProcessCategory::Application);
    assert_eq!(categorize_process(401, "code.exe", None), ProcessCategory::Application);
}
