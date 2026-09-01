// SPDX-License-Identifier: GPL-3.0-or-later
//! First-run / pre-ingest checks: are the bundled device-backup tools
//! present, and does the platform have what it needs (Full Disk Access on
//! macOS, Apple's USB driver on Windows)?

use std::path::Path;

use crate::command_runner::CommandRunner;
use crate::device::Tools;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrerequisiteStatus {
    Ok,
    /// Not met, with user-facing guidance on how to fix it.
    Missing {
        guidance: String,
    },
}

impl PrerequisiteStatus {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, PrerequisiteStatus::Ok)
    }
}

#[derive(Debug, Clone)]
pub struct PrerequisiteReport {
    /// Bundled `idevice_id` sidecar (device enumeration).
    pub idevice_id: PrerequisiteStatus,
    /// Bundled `idevicebackup2` sidecar (backup creation/refresh).
    pub idevicebackup2: PrerequisiteStatus,
    /// Full Disk Access (macOS) or Apple's USB driver (Windows). A no-op
    /// `Ok` on the platform it doesn't apply to.
    pub platform: PrerequisiteStatus,
}

impl PrerequisiteReport {
    #[must_use]
    pub fn all_ok(&self) -> bool {
        self.idevice_id.is_ok() && self.idevicebackup2.is_ok() && self.platform.is_ok()
    }
}

/// Check everything `Source::Device` ingest needs before it's attempted:
/// the bundled tools exist, and the platform-specific USB/Full-Disk-Access
/// prerequisite is met.
pub fn check_prerequisites(runner: &dyn CommandRunner, tools: &Tools) -> PrerequisiteReport {
    PrerequisiteReport {
        idevice_id: check_tool_present("idevice_id", &tools.idevice_id),
        idevicebackup2: check_tool_present("idevicebackup2", &tools.idevicebackup2),
        platform: check_platform_driver(runner),
    }
}

fn check_tool_present(name: &str, path: &Path) -> PrerequisiteStatus {
    if path.is_file() {
        PrerequisiteStatus::Ok
    } else {
        PrerequisiteStatus::Missing {
            guidance: format!(
                "{name} is missing from Amber's bundled tools ({}). Reinstalling Amber should restore it.",
                path.display()
            ),
        }
    }
}

#[cfg(target_os = "macos")]
fn check_platform_driver(_runner: &dyn CommandRunner) -> PrerequisiteStatus {
    let Some(home) = dirs::home_dir() else {
        return PrerequisiteStatus::Missing {
            guidance: "Could not determine your home directory.".to_string(),
        };
    };
    let chat_db = home.join("Library/Messages/chat.db");

    match std::fs::metadata(&chat_db) {
        Ok(_) => PrerequisiteStatus::Ok,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            PrerequisiteStatus::Missing {
                guidance: "Amber needs Full Disk Access to read Messages. Go to System \
                    Settings > Privacy & Security > Full Disk Access, and turn Amber on."
                    .to_string(),
            }
        }
        // Not found just means "no Messages history yet" or this isn't the
        // Mac with the conversation - not a permissions problem.
        Err(_) => PrerequisiteStatus::Ok,
    }
}

#[cfg(target_os = "windows")]
fn check_platform_driver(runner: &dyn CommandRunner) -> PrerequisiteStatus {
    let missing = PrerequisiteStatus::Missing {
        guidance: "Amber needs Apple's Mobile Device USB driver to talk to an iPhone. Install \
            it via the Apple Devices app (Microsoft Store) or iTunes from apple.com, then \
            reconnect your phone."
            .to_string(),
    };

    let mut ignored = String::new();
    let Ok(output) = runner.run(
        Path::new("sc"),
        &["query", "Apple Mobile Device Service"],
        &[],
        &mut |line| ignored.push_str(line),
    ) else {
        return missing;
    };

    if output.success {
        PrerequisiteStatus::Ok
    } else {
        missing
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn check_platform_driver(_runner: &dyn CommandRunner) -> PrerequisiteStatus {
    PrerequisiteStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_runner::CommandOutput;

    struct FakeCommandRunner {
        success: bool,
    }

    impl CommandRunner for FakeCommandRunner {
        fn run(
            &self,
            _program: &Path,
            _args: &[&str],
            _envs: &[(&str, &str)],
            _on_stdout_line: &mut dyn FnMut(&str),
        ) -> std::io::Result<CommandOutput> {
            Ok(CommandOutput {
                success: self.success,
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn missing_tools_are_reported_missing_with_guidance() {
        let tools = Tools {
            idevice_id: Path::new("/definitely/does/not/exist/idevice_id").to_path_buf(),
            idevicebackup2: Path::new("/definitely/does/not/exist/idevicebackup2").to_path_buf(),
        };
        let report = check_prerequisites(&FakeCommandRunner { success: true }, &tools);

        assert!(!report.idevice_id.is_ok());
        assert!(!report.idevicebackup2.is_ok());
        assert!(!report.all_ok());
    }

    #[test]
    fn present_tools_are_reported_ok() {
        // This test binary itself is a real file - stands in for a present tool.
        let self_path = std::env::current_exe().unwrap();
        let tools = Tools {
            idevice_id: self_path.clone(),
            idevicebackup2: self_path,
        };
        let report = check_prerequisites(&FakeCommandRunner { success: true }, &tools);

        assert!(report.idevice_id.is_ok());
        assert!(report.idevicebackup2.is_ok());
    }
}
