use super::category::normalize_proc_name;
use super::error::ProcessTerminationError;

/// Hardcoded Windows OS Process Safety Blacklist with structured danger explanations
pub const WINDOWS_PROTECTED_PROCESSES: &[(&str, &str)] = &[
    (
        "csrss.exe",
        "Client Server Runtime Subsystem (csrss.exe) is vital for Windows operation; terminating it triggers an immediate Blue Screen of Death (CRITICAL_PROCESS_DIED).",
    ),
    (
        "lsass.exe",
        "Local Security Authority Subsystem Service (lsass.exe) enforces Windows security policies; terminating it triggers an immediate system restart.",
    ),
    (
        "services.exe",
        "Service Control Manager (services.exe) manages all system background services; terminating it crashes the operating system.",
    ),
    (
        "explorer.exe",
        "Windows Explorer (explorer.exe) provides the desktop GUI, taskbar, and file manager; terminating it crashes the graphical user session.",
    ),
    (
        "smss.exe",
        "Session Manager Subsystem (smss.exe) is the first user-mode process started during boot; terminating it causes an instant kernel panic.",
    ),
    (
        "wininit.exe",
        "Windows Initialization Process (wininit.exe) launches core subsystems; terminating it triggers an immediate system crash.",
    ),
    (
        "winlogon.exe",
        "Windows Logon Process (winlogon.exe) manages user logon and secure attention sequences; terminating it locks the user out or causes a BSOD.",
    ),
    (
        "svchost.exe",
        "Service Host (svchost.exe) hosts critical Windows system services and networking components; terminating it disrupts essential operating system functionality.",
    ),
    (
        "dwm.exe",
        "Desktop Window Manager (dwm.exe) manages display compositing and window rendering; terminating it disrupts desktop display.",
    ),
    (
        "system",
        "Windows NT Kernel Process executing core system threads; cannot be terminated.",
    ),
    (
        "system idle process",
        "CPU Idle Loop Thread measuring processor idle cycles; cannot be terminated.",
    ),
    (
        "registry",
        "Windows Registry Subsystem holding in-memory registry hives; cannot be terminated.",
    ),
    (
        "fontdrvhost.exe",
        "User-Mode Font Driver Host providing isolated font rendering.",
    ),
    (
        "conhost.exe",
        "Console Window Host providing terminal UI support.",
    ),
    (
        "taskhostw.exe",
        "Host Process for Windows Tasks executing background system tasks.",
    ),
    (
        "sihost.exe",
        "Shell Infrastructure Host coordinating Windows shell components.",
    ),
    (
        "securityhealthservice.exe",
        "Windows Defender Security Center Service monitoring system protection.",
    ),
    (
        "msmpeng.exe",
        "Microsoft Defender Antivirus Core scanning engine.",
    ),
    (
        "tbk_core.exe",
        "PC Doctor / TBK Core desktop application guard preventing self-termination.",
    ),
    (
        "pc-doctor.exe",
        "PC Doctor application executable guard preventing self-termination.",
    ),
];

#[derive(Debug, Clone)]
pub struct ProcessGuard {
    self_pid: u32,
}

impl Default for ProcessGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessGuard {
    pub fn new() -> Self {
        Self {
            self_pid: std::process::id(),
        }
    }

    /// Construct a guard with an explicit self_pid (vital for deterministic unit testing)
    pub fn with_self_pid(self_pid: u32) -> Self {
        Self { self_pid }
    }

    pub fn self_pid(&self) -> u32 {
        self.self_pid
    }

    /// Validates whether a process can be safely terminated.
    /// Returns Ok(()) if safe to terminate, or Err(ProcessTerminationError) if protected.
    pub fn validate_termination(
        &self,
        pid: u32,
        name: &str,
    ) -> Result<(), ProcessTerminationError> {
        // 1. PID <= 4 check (PID 0 Idle, PID 4 System Kernel)
        if pid <= 4 {
            let proc_name = if name.is_empty() {
                if pid == 0 {
                    "System Idle Process"
                } else {
                    "System"
                }
            } else {
                name
            };
            return Err(ProcessTerminationError::ProtectedSystemProcess {
                pid,
                name: proc_name.to_string(),
                reason: format!(
                    "Process with PID {} is a core Windows NT kernel process and cannot be terminated.",
                    pid
                ),
            });
        }

        // 2. Self PID check
        if pid == self.self_pid {
            return Err(ProcessTerminationError::ProtectedSystemProcess {
                pid,
                name: name.to_string(),
                reason: "PC Doctor cannot terminate its own application process.".to_string(),
            });
        }

        // 3. Blacklist process name check (case-insensitive, normalized)
        let normalized = normalize_proc_name(name);
        for &(protected_name, reason) in WINDOWS_PROTECTED_PROCESSES {
            if normalized == normalize_proc_name(protected_name) {
                return Err(ProcessTerminationError::ProtectedSystemProcess {
                    pid,
                    name: name.to_string(),
                    reason: reason.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Returns true if the process is protected from termination.
    pub fn is_protected(&self, pid: u32, name: &str) -> bool {
        self.validate_termination(pid, name).is_err()
    }

    /// Returns the protection reason if protected, or None if unprotected.
    pub fn protection_reason(&self, pid: u32, name: &str) -> Option<String> {
        match self.validate_termination(pid, name) {
            Err(ProcessTerminationError::ProtectedSystemProcess { reason, .. }) => Some(reason),
            _ => None,
        }
    }
}
