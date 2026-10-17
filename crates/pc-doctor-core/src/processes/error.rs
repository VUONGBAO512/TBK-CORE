use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error_type", content = "details")]
pub enum ProcessTerminationError {
    ProtectedSystemProcess {
        pid: u32,
        name: String,
        reason: String,
    },
    ProcessNotFound {
        pid: u32,
    },
    AccessDenied {
        pid: u32,
        name: String,
        os_error_code: u32,
        message: String,
    },
    InternalError {
        message: String,
    },
}

impl fmt::Display for ProcessTerminationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectedSystemProcess { pid, name, reason } => {
                write!(
                    f,
                    "Process '{}' (PID {}) is protected and cannot be terminated: {}",
                    name, pid, reason
                )
            }
            Self::ProcessNotFound { pid } => {
                write!(f, "Process with PID {} was not found", pid)
            }
            Self::AccessDenied {
                pid,
                name,
                os_error_code,
                message,
            } => {
                write!(
                    f,
                    "Access denied terminating process '{}' (PID {}, OS code {}): {}",
                    name, pid, os_error_code, message
                )
            }
            Self::InternalError { message } => {
                write!(f, "Internal error during process termination: {}", message)
            }
        }
    }
}

impl std::error::Error for ProcessTerminationError {}
