//! Process inspection and management commands for PC Doctor (TBK Core).
//!
//! Provides Tauri IPC commands for listing running processes with categorization
//! and safety protection flags, and safely terminating processes with hardcoded
//! Windows safety guardrails.

use crate::monitoring::CollectorState;
use pc_doctor_core::models::{ProcessItem, TerminateResult};
use pc_doctor_core::processes::{categorize_process, ProcessGuard};
use std::sync::Arc;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::State;

/// Collects and categorizes all system processes, attaching safety guard protection status.
/// Sorted descending by CPU percentage, fallback to memory consumption.
pub fn collect_process_items(sys: &System, guard: &ProcessGuard) -> Vec<ProcessItem> {
    let mut list: Vec<ProcessItem> = sys
        .processes()
        .iter()
        .map(|(pid, proc)| {
            let pid_u32 = pid.as_u32();
            let name = proc.name().to_string_lossy().to_string();
            let exe_path = proc.exe().and_then(|p| p.to_str());
            let category = categorize_process(pid_u32, &name, exe_path);
            let is_protected = guard.is_protected(pid_u32, &name);

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

    list.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
    });

    list
}

/// Core safe process termination logic with injected ProcessGuard.
/// Validates process against hardcoded OS blacklist, PID <= 4, and self PID.
/// If blocked, returns structured JSON ProcessTerminationError without touching OS process.
/// If approved, refreshes target process in sysinfo and invokes Process::kill().
pub fn terminate_process_with_guard(
    guard: &ProcessGuard,
    pid: u32,
    process_name: &str,
) -> Result<TerminateResult, String> {
    // 1. Validate termination against ProcessGuard
    if let Err(err) = guard.validate_termination(pid, process_name) {
        let err_json = serde_json::to_string(&err)
            .unwrap_or_else(|_| format!(r#"{{"error_type":"ProtectedSystemProcess","details":{{"pid":{},"name":"{}","reason":"{}"}}}}"#, pid, process_name, err));
        return Err(err_json);
    }

    // 2. Query target process from OS via sysinfo
    let mut sys = System::new();
    let sys_pid = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);

    match sys.process(sys_pid) {
        Some(proc) => {
            // Defense-in-depth: check actual OS binary name against ProcessGuard
            let real_name = proc.name().to_string_lossy().to_string();
            if let Err(err) = guard.validate_termination(pid, &real_name) {
                let err_json = serde_json::to_string(&err)
                    .unwrap_or_else(|_| format!(r#"{{"error_type":"ProtectedSystemProcess","details":{{"pid":{},"name":"{}","reason":"{}"}}}}"#, pid, real_name, err));
                return Err(err_json);
            }

            let effective_name = if process_name.trim().is_empty() {
                real_name
            } else {
                process_name.to_string()
            };

            // 3. Invoke sysinfo::Process::kill()
            let killed = proc.kill();
            if killed {
                Ok(TerminateResult::success(
                    pid,
                    effective_name.clone(),
                    format!("Process '{}' (PID {}) was successfully terminated.", effective_name, pid),
                ))
            } else {
                Ok(TerminateResult::failure(
                    pid,
                    effective_name.clone(),
                    format!("Failed to terminate process '{}' (PID {}). Access denied or process is already exiting.", effective_name, pid),
                ))
            }
        }
        None => {
            let effective_name = if process_name.trim().is_empty() {
                format!("PID {}", pid)
            } else {
                process_name.to_string()
            };
            Ok(TerminateResult::failure(
                pid,
                effective_name,
                format!("Process with PID {} was not found.", pid),
            ))
        }
    }
}

/// Tauri IPC command: returns complete list of running processes with categorization and protection flags.
#[tauri::command]
pub fn get_process_list(
    state: State<'_, Arc<CollectorState>>,
) -> Result<Vec<ProcessItem>, String> {
    let mut collector = state
        .collector
        .lock()
        .map_err(|e| format!("Failed to acquire collector lock: {}", e))?;
    collector.refresh_processes();
    Ok(collector.get_process_list())
}

/// Tauri IPC command: terminates a process by PID and process_name after validating safety guardrails.
#[tauri::command]
pub fn terminate_process(pid: u32, process_name: String) -> Result<TerminateResult, String> {
    let guard = ProcessGuard::new();
    terminate_process_with_guard(&guard, pid, &process_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc_doctor_core::models::ProcessCategory;
    use pc_doctor_core::processes::ProcessTerminationError;

    // ==========================================
    // 1. Mandatory Windows Blacklist Rejection Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_csrss() {
        let res = terminate_process(1001, "csrss.exe".to_string());
        assert!(res.is_err(), "csrss.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("csrss.exe"));
        assert!(err_json.contains("CRITICAL_PROCESS_DIED"));

        let err: ProcessTerminationError = serde_json::from_str(&err_json).expect("Must deserialize into ProcessTerminationError");
        match err {
            ProcessTerminationError::ProtectedSystemProcess { pid, name, reason } => {
                assert_eq!(pid, 1001);
                assert_eq!(name, "csrss.exe");
                assert!(!reason.is_empty());
            }
            _ => panic!("Expected ProtectedSystemProcess variant"),
        }
    }

    #[test]
    fn test_terminate_process_rejects_lsass() {
        let res = terminate_process(1002, "lsass.exe".to_string());
        assert!(res.is_err(), "lsass.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("lsass.exe"));
    }

    #[test]
    fn test_terminate_process_rejects_services() {
        let res = terminate_process(1003, "services.exe".to_string());
        assert!(res.is_err(), "services.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("services.exe"));
    }

    #[test]
    fn test_terminate_process_rejects_explorer() {
        let res = terminate_process(1004, "explorer.exe".to_string());
        assert!(res.is_err(), "explorer.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("explorer.exe"));
    }

    #[test]
    fn test_terminate_process_rejects_smss() {
        let res = terminate_process(1005, "smss.exe".to_string());
        assert!(res.is_err(), "smss.exe must be rejected from termination");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("smss.exe"));
    }

    // ==========================================
    // 2. PID 4 and Kernel Boundary Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_pid_four_system() {
        let res = terminate_process(4, "System".to_string());
        assert!(res.is_err(), "PID 4 System must be rejected");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("\"pid\":4"));
        assert!(err_json.contains("kernel process"));
    }

    #[test]
    fn test_terminate_process_rejects_pid_four_with_arbitrary_name() {
        let res = terminate_process(4, "my_random_app.exe".to_string());
        assert!(res.is_err(), "PID 4 must be rejected regardless of passed name");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("\"pid\":4"));
    }

    #[test]
    fn test_terminate_process_rejects_pid_zero_idle() {
        let res = terminate_process(0, "System Idle Process".to_string());
        assert!(res.is_err(), "PID 0 must be rejected");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("\"pid\":0"));
    }

    // ==========================================
    // 3. Self PID Protection Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_self_pid() {
        let self_pid = std::process::id();
        let res = terminate_process(self_pid, "TBK_Core.exe".to_string());
        assert!(res.is_err(), "PC Doctor must not terminate its own PID");
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains(&self_pid.to_string()));
        assert!(err_json.contains("own application process"));
    }

    #[test]
    fn test_terminate_process_rejects_self_pid_with_mock_guard() {
        let mock_pid = 77777;
        let mock_guard = ProcessGuard::with_self_pid(mock_pid);
        let res = terminate_process_with_guard(&mock_guard, mock_pid, "anything.exe");
        assert!(res.is_err());
        let err_json = res.unwrap_err();
        assert!(err_json.contains("ProtectedSystemProcess"));
        assert!(err_json.contains("77777"));
        assert!(err_json.contains("own application process"));
    }

    // ==========================================
    // 4. Case Variations & Extensions Tests
    // ==========================================

    #[test]
    fn test_terminate_process_rejects_casing_variations() {
        let variations = [
            ("CSRSS.EXE", 2001),
            ("csrss", 2002),
            ("CsRss.Exe", 2003),
            ("EXPLORER.EXE", 2004),
            ("explorer", 2005),
            ("Explorer.exe", 2006),
            ("LSASS.EXE", 2007),
            ("lsass", 2008),
            ("SERVICES.EXE", 2009),
            ("services", 2010),
            ("SMSS.EXE", 2011),
            ("smss", 2012),
            ("dwm.exe", 2013),
            ("DWM.EXE", 2014),
            ("svchost.exe", 2015),
            ("SVCHOST.EXE", 2016),
            ("wininit.exe", 2017),
            ("winlogon.exe", 2018),
        ];

        for (name, pid) in variations {
            let res = terminate_process(pid, name.to_string());
            assert!(
                res.is_err(),
                "Process variation '{}' must be blocked from termination",
                name
            );
            let err_json = res.unwrap_err();
            assert!(
                err_json.contains("ProtectedSystemProcess"),
                "Error for '{}' must be ProtectedSystemProcess",
                name
            );
        }
    }

    // ==========================================
    // 5. Unprotected Application Approval
    // ==========================================

    #[test]
    fn test_terminate_process_allows_unprotected_application_attempt() {
        let non_existent_pid = 999_999;
        let res = terminate_process(non_existent_pid, "notepad.exe".to_string());
        assert!(res.is_ok(), "Guard should approve unprotected notepad.exe");
        let term_res = res.unwrap();
        assert_eq!(term_res.pid, non_existent_pid);
        assert_eq!(term_res.process_name, "notepad.exe");
        assert!(!term_res.success, "Non-existent process termination should report success = false");
        assert!(term_res.message.contains("not found"));
    }

    // ==========================================
    // 6. Process Listing & Categorization Tests
    // ==========================================

    #[test]
    fn test_collect_process_items_structure() {
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let guard = ProcessGuard::new();

        let items = collect_process_items(&sys, &guard);
        assert!(!items.is_empty(), "Process list must not be empty on running system");

        // Verify protected flag is present on core processes
        let has_protected = items.iter().any(|p| p.is_protected);
        assert!(has_protected, "At least one system process should be marked is_protected");

        // Verify categories exist
        let has_system_or_service = items.iter().any(|p| {
            p.category == ProcessCategory::System || p.category == ProcessCategory::BackgroundService
        });
        assert!(has_system_or_service, "Process list should include System or Service categories");

        // Verify descending sort order by CPU percentage
        for i in 1..items.len() {
            assert!(
                items[i - 1].cpu_percent >= items[i].cpu_percent
                    || (items[i - 1].cpu_percent == items[i].cpu_percent
                        && items[i - 1].memory_bytes >= items[i].memory_bytes),
                "Process items must be sorted descending by CPU percent then memory"
            );
        }
    }
}
