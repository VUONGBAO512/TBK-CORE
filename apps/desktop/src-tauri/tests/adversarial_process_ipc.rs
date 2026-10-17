//! Adversarial empirical test probe suite for Process IPC commands in desktop_lib.
//! 
//! Tests:
//! 1. All 9 mandatory protected processes rejection (csrss, lsass, services, explorer, smss, wininit, winlogon, svchost, dwm)
//! 2. Case variations and extension stripping (CSRSS.EXE, csrss, Explorer.exe, EXPLORER, etc.)
//! 3. PID boundaries (PID 0, PID 4, PID <= 4, PID 5, u32::MAX)
//! 4. Self PID protection (current OS PID and mock self PID)
//! 5. Spoofing defense-in-depth:
//!    - PID 4 spoofed with benign name ("notepad.exe")
//!    - Self PID spoofed with benign name ("calc.exe")
//!    - Real running protected system process on host targeted with benign spoofed name ("harmless_app.exe")
//!    - Real running protected system process on host targeted with full path ("C:\\Windows\\explorer.exe")
//! 6. Whitespace and formatting tricks
//! 7. Path-containing inputs probe
//! 8. Process listing and sorting invariants

use desktop_lib::commands::process::{
    collect_process_items, terminate_process, terminate_process_with_guard,
};
use pc_doctor_core::models::ProcessCategory;
use pc_doctor_core::processes::{ProcessGuard, ProcessTerminationError};
use sysinfo::{ProcessesToUpdate, System};

// ============================================================================
// 1. Mandatory Windows Blacklist Rejection Tests (All 9)
// ============================================================================

#[test]
fn test_adversarial_all_nine_mandatory_protected_processes() {
    let mandatory = [
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

    for (i, proc_name) in mandatory.iter().enumerate() {
        let fake_pid = 20000 + i as u32;
        let res = terminate_process(fake_pid, proc_name.to_string());

        assert!(
            res.is_err(),
            "Mandatory protected process '{}' MUST be rejected by terminate_process",
            proc_name
        );

        let err_json = res.unwrap_err();
        let err: ProcessTerminationError = serde_json::from_str(&err_json)
            .unwrap_or_else(|e| panic!("Failed to parse error JSON for '{}': {}. JSON: {}", proc_name, e, err_json));

        match err {
            ProcessTerminationError::ProtectedSystemProcess { pid, name, reason } => {
                assert_eq!(pid, fake_pid);
                assert_eq!(name, *proc_name);
                assert!(!reason.is_empty(), "Reason must not be empty for {}", proc_name);
            }
            other => panic!("Unexpected error variant for '{}': {:?}", proc_name, other),
        }
    }
}

// ============================================================================
// 2. Case Variations, Extension Stripping & Whitespace Padding
// ============================================================================

#[test]
fn test_adversarial_casing_and_extension_tricks() {
    let variations = [
        // CSRSS
        ("CSRSS.EXE", 30001),
        ("csrss", 30002),
        ("CSRSS", 30003),
        ("CsRss.Exe", 30004),
        ("  csrss.exe  ", 30005),
        ("\tcsrss\n", 30006),
        // EXPLORER
        ("Explorer.exe", 30010),
        ("EXPLORER", 30011),
        ("EXPLORER.EXE", 30012),
        ("ExPlOrEr.EXE", 30013),
        ("explorer", 30014),
        ("  explorer.exe\t", 30015),
        // LSASS
        ("LSASS.EXE", 30020),
        ("lsass", 30021),
        ("LSASS", 30022),
        ("Lsass.Exe", 30023),
        // SERVICES
        ("SERVICES.EXE", 30030),
        ("services", 30031),
        ("SERVICES", 30032),
        ("Services.Exe", 30033),
        // SMSS
        ("SMSS.EXE", 30040),
        ("smss", 30041),
        ("SMSS", 30042),
        ("Smss.Exe", 30043),
        // WININIT
        ("WININIT.EXE", 30050),
        ("wininit", 30051),
        ("WinInit.exe", 30052),
        // WINLOGON
        ("WINLOGON.EXE", 30060),
        ("winlogon", 30061),
        ("WinLogon.exe", 30062),
        // SVCHOST
        ("SVCHOST.EXE", 30070),
        ("svchost", 30071),
        ("SvChOsT.eXe", 30072),
        // DWM
        ("DWM.EXE", 30080),
        ("dwm", 30081),
        ("Dwm.Exe", 30082),
    ];

    for (name, pid) in variations {
        let res = terminate_process(pid, name.to_string());
        assert!(
            res.is_err(),
            "Casing / whitespace trick '{}' (PID {}) must be blocked",
            name,
            pid
        );
        let err_json = res.unwrap_err();
        assert!(
            err_json.contains("ProtectedSystemProcess"),
            "Error for '{}' must be ProtectedSystemProcess, got: {}",
            name,
            err_json
        );
    }
}

// ============================================================================
// 3. PID Boundaries: PID 0, PID 4, PID <= 4, Self PID, Boundary Transitions
// ============================================================================

#[test]
fn test_adversarial_pid_boundaries_zero_to_four() {
    for pid in 0..=4 {
        // Test with arbitrary benign name
        let res = terminate_process(pid, "notepad.exe".to_string());
        assert!(
            res.is_err(),
            "PID {} with benign name 'notepad.exe' must be blocked",
            pid
        );
        let err_json = res.unwrap_err();
        assert!(
            err_json.contains("ProtectedSystemProcess"),
            "PID {} error must be ProtectedSystemProcess",
            pid
        );
        assert!(
            err_json.contains("kernel process"),
            "PID {} reason must mention kernel process",
            pid
        );

        // Test with empty string name
        let res_empty = terminate_process(pid, "".to_string());
        assert!(res_empty.is_err(), "PID {} with empty name must be blocked", pid);
        let err_empty = res_empty.unwrap_err();
        assert!(err_empty.contains("ProtectedSystemProcess"));
        if pid == 0 {
            assert!(err_empty.contains("System Idle Process"));
        } else {
            assert!(err_empty.contains("System"));
        }
    }
}

#[test]
fn test_adversarial_self_pid_protection() {
    let self_pid = std::process::id();

    // With own binary name
    let res1 = terminate_process(self_pid, "TBK_Core.exe".to_string());
    assert!(res1.is_err(), "Self PID {} must be blocked", self_pid);
    let err1 = res1.unwrap_err();
    assert!(err1.contains("own application process"));

    // With benign spoofed name
    let res2 = terminate_process(self_pid, "calculator.exe".to_string());
    assert!(res2.is_err(), "Self PID {} with spoofed name must be blocked", self_pid);
    let err2 = res2.unwrap_err();
    assert!(err2.contains("own application process"));

    // With empty name
    let res3 = terminate_process(self_pid, "".to_string());
    assert!(res3.is_err(), "Self PID {} with empty name must be blocked", self_pid);
    let err3 = res3.unwrap_err();
    assert!(err3.contains("own application process"));

    // Mock guard test
    let mock_guard = ProcessGuard::with_self_pid(98765);
    let res_mock = terminate_process_with_guard(&mock_guard, 98765, "harmless.exe");
    assert!(res_mock.is_err());
    let err_mock = res_mock.unwrap_err();
    assert!(err_mock.contains("98765"));
    assert!(err_mock.contains("own application process"));
}

#[test]
fn test_adversarial_pid_five_and_large_pid_transitions() {
    // PID 5 is right above the kernel threshold (pid <= 4)
    // Non-existent PID 5 with benign name should NOT trigger ProtectedSystemProcess
    let res_benign = terminate_process(5, "my_custom_worker.exe".to_string());
    assert!(
        res_benign.is_ok(),
        "PID 5 with benign name should pass guard and fail gracefully at OS level"
    );
    let term_res = res_benign.unwrap();
    assert_eq!(term_res.pid, 5);
    assert!(!term_res.success);

    // PID 5 with protected name "csrss.exe" MUST be blocked
    let res_protected = terminate_process(5, "csrss.exe".to_string());
    assert!(res_protected.is_err(), "PID 5 with csrss.exe must be blocked");
    assert!(res_protected.unwrap_err().contains("ProtectedSystemProcess"));

    // u32::MAX boundary
    let max_pid = u32::MAX;
    let res_max_benign = terminate_process(max_pid, "harmless.exe".to_string());
    assert!(res_max_benign.is_ok());
    let res_max_protected = terminate_process(max_pid, "dwm.exe".to_string());
    assert!(res_max_protected.is_err());
}

// ============================================================================
// 4. Spoofing Attack Probes (Defense-in-Depth)
// ============================================================================

#[test]
fn test_adversarial_spoofing_pid_four_with_benign_names() {
    let benign_names = [
        "notepad.exe",
        "calc.exe",
        "chrome.exe",
        "explorer_fake.exe",
        "my_game.exe",
        "",
        "   ",
    ];

    for name in benign_names {
        let res = terminate_process(4, name.to_string());
        assert!(
            res.is_err(),
            "PID 4 spoofed with name '{}' must be rejected",
            name
        );
        let err_json = res.unwrap_err();
        assert!(
            err_json.contains("ProtectedSystemProcess"),
            "Error for spoofed PID 4 '{}' must be ProtectedSystemProcess",
            name
        );
        assert!(
            err_json.contains("kernel process"),
            "Reason must state kernel process"
        );
    }
}

#[test]
fn test_adversarial_spoofing_live_host_protected_process() {
    // Find an actual running protected process on this Windows host (e.g. explorer.exe or dwm.exe or svchost.exe)
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let protected_names = ["explorer.exe", "dwm.exe", "svchost.exe", "csrss.exe", "services.exe", "lsass.exe"];

    let live_target = sys.processes().iter().find(|(pid, proc)| {
        let p_u32 = pid.as_u32();
        if p_u32 <= 4 || p_u32 == std::process::id() {
            return false;
        }
        let proc_name = proc.name().to_string_lossy().to_lowercase();
        protected_names.iter().any(|&prot| proc_name == prot || proc_name == prot.strip_suffix(".exe").unwrap())
    });

    if let Some((target_pid, target_proc)) = live_target {
        let real_pid = target_pid.as_u32();
        let real_name = target_proc.name().to_string_lossy().to_string();

        // SCENARIO 1: Spoof with benign name ("notepad.exe")
        let spoof_name = "notepad.exe".to_string();
        let res = terminate_process(real_pid, spoof_name);

        assert!(
            res.is_err(),
            "CRITICAL DEFENSE FAILURE: Stage 2 failed to block live protected process PID {} (real name '{}') when spoofed as 'notepad.exe'!",
            real_pid, real_name
        );

        let err_json = res.unwrap_err();
        assert!(
            err_json.contains("ProtectedSystemProcess"),
            "Error must be ProtectedSystemProcess, got: {}",
            err_json
        );

        // SCENARIO 2: Spoof with full path ("C:\\Windows\\System32\\..." or "C:\\Windows\\explorer.exe")
        let full_path_spoof = format!("C:\\Windows\\{}", real_name);
        let res_path = terminate_process(real_pid, full_path_spoof);

        assert!(
            res_path.is_err(),
            "CRITICAL DEFENSE FAILURE: Stage 2 failed to block live protected process PID {} when passed full path!",
            real_pid
        );

        // Verify the process is still running and was NOT killed
        let mut verify_sys = System::new();
        verify_sys.refresh_processes(ProcessesToUpdate::Some(&[*target_pid]), true);
        assert!(
            verify_sys.process(*target_pid).is_some(),
            "Process PID {} ({}) was killed despite protection!",
            real_pid, real_name
        );
    }
}

// ============================================================================
// 5. Unprotected Application Lifecycle Verification
// ============================================================================

#[test]
fn test_adversarial_unprotected_dummy_process_allowed() {
    let non_existent_pid = 987_654;
    let res = terminate_process(non_existent_pid, "valid_user_tool.exe".to_string());
    assert!(
        res.is_ok(),
        "Guard must approve termination request for non-protected process"
    );
    let result = res.unwrap();
    assert_eq!(result.pid, non_existent_pid);
    assert_eq!(result.process_name, "valid_user_tool.exe");
    assert!(!result.success);
    assert!(result.message.contains("not found"));
}

// ============================================================================
// 6. Path-containing Inputs Investigation Probe
// ============================================================================

#[test]
fn test_adversarial_path_containing_inputs() {
    let path_inputs = [
        ("C:\\Windows\\System32\\csrss.exe", 40001),
        ("C:/Windows/System32/csrss.exe", 40002),
        ("C:\\Windows\\explorer.exe", 40003),
        ("C:/Windows/explorer.exe", 40004),
        (r"System32\services.exe", 40005),
    ];

    for (path_name, pid) in path_inputs {
        let guard = ProcessGuard::new();
        let guard_res = guard.validate_termination(pid, path_name);
        // Documents that Stage 1 check without OS process lookup does not normalize directory prefixes
        assert!(
            guard_res.is_ok(),
            "Stage 1 guard checks base binary name against exact normalized token, directory paths do not match Stage 1 directly"
        );

        let term_res = terminate_process(pid, path_name.to_string());
        // For non-existent PID, sysinfo returns "not found"
        assert!(term_res.is_ok());
        let tr = term_res.unwrap();
        assert!(!tr.success);
        assert!(tr.message.contains("not found"));
    }
}

// ============================================================================
// 7. Process Listing and Sorting Invariants
// ============================================================================

#[test]
fn test_adversarial_process_listing_invariants() {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let guard = ProcessGuard::new();

    let items = collect_process_items(&sys, &guard);
    assert!(!items.is_empty(), "System process list must not be empty");

    // All PID <= 4 must be marked is_protected = true
    for item in items.iter().filter(|p| p.pid <= 4) {
        assert!(
            item.is_protected,
            "PID {} ({}) must have is_protected == true",
            item.pid, item.name
        );
        assert_eq!(
            item.category,
            ProcessCategory::System,
            "PID {} ({}) must have category System",
            item.pid,
            item.name
        );
    }

    // Verify sorting order: cpu_percent descending, then memory_bytes descending
    for i in 1..items.len() {
        let prev = &items[i - 1];
        let curr = &items[i];
        let ordered = prev.cpu_percent > curr.cpu_percent
            || ((prev.cpu_percent - curr.cpu_percent).abs() < f32::EPSILON
                && prev.memory_bytes >= curr.memory_bytes);
        assert!(
            ordered,
            "Ordering violated between #{}: (cpu: {}, mem: {}) and #{}: (cpu: {}, mem: {})",
            i - 1,
            prev.cpu_percent,
            prev.memory_bytes,
            i,
            curr.cpu_percent,
            curr.memory_bytes
        );
    }
}
