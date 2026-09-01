// SPDX-License-Identifier: GPL-3.0-or-later
//! First-run / pre-ingest checks: are the device-backup tools invocable
//! (as a bundled sidecar or via `PATH`), and does the platform have what it
//! needs (Full Disk Access on macOS, Apple's USB driver on Windows)?

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
    /// `idevice_id` (device enumeration) - bundled sidecar or on `PATH`.
    pub idevice_id: PrerequisiteStatus,
    /// `idevicebackup2` (backup creation/refresh) - bundled sidecar or on `PATH`.
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
/// the tools actually run (via `runner` - a bundled sidecar or `PATH`, same
/// as the real ingest calls use), and the platform-specific USB/Full-Disk-Access
/// prerequisite is met.
pub fn check_prerequisites(runner: &dyn CommandRunner, tools: &Tools) -> PrerequisiteReport {
    PrerequisiteReport {
        idevice_id: check_tool_present(runner, "idevice_id", &tools.idevice_id),
        idevicebackup2: check_tool_present(runner, "idevicebackup2", &tools.idevicebackup2),
        platform: check_platform_driver(runner),
    }
}

/// A real invocation (`--version`, which both tools document) rather than a
/// filesystem check: it works uniformly whether the tool is a bundled Tauri
/// sidecar (resolved at a Tauri-managed runtime path, not a fixed
/// filesystem location this crate knows about) or found on `PATH` for local
/// development. The exit code doesn't matter, only whether the process
/// could be spawned at all.
fn check_tool_present(runner: &dyn CommandRunner, name: &str, path: &Path) -> PrerequisiteStatus {
    match runner.run(path, &["--version"], &[], &mut |_| {}) {
        Ok(_) => PrerequisiteStatus::Ok,
        Err(_) => PrerequisiteStatus::Missing {
            guidance: format!(
                "{name} isn't available. Reinstalling Amber should restore it if this is a bundled build; for local development, install libimobiledevice and make sure {name} is on PATH."
            ),
        },
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

    /// Simulates "binary not found" (an `Err`, like a real failed spawn)
    /// for any program name in `unavailable`; everything else "succeeds".
    struct FakeCommandRunner {
        unavailable: Vec<&'static str>,
    }

    impl CommandRunner for FakeCommandRunner {
        fn run(
            &self,
            program: &Path,
            _args: &[&str],
            _envs: &[(&str, &str)],
            _on_stdout_line: &mut dyn FnMut(&str),
        ) -> std::io::Result<CommandOutput> {
            let name = program
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if self.unavailable.contains(&name) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                ));
            }
            Ok(CommandOutput {
                success: true,
                stderr: String::new(),
            })
        }
    }

    fn tools() -> Tools {
        Tools {
            idevice_id: Path::new("idevice_id").to_path_buf(),
            idevicebackup2: Path::new("idevicebackup2").to_path_buf(),
        }
    }

    #[test]
    fn missing_tools_are_reported_missing_with_guidance() {
        let runner = FakeCommandRunner {
            unavailable: vec!["idevice_id", "idevicebackup2"],
        };
        let report = check_prerequisites(&runner, &tools());

        assert!(!report.idevice_id.is_ok());
        assert!(!report.idevicebackup2.is_ok());
        assert!(!report.all_ok());
    }

    #[test]
    fn present_tools_are_reported_ok() {
        let runner = FakeCommandRunner {
            unavailable: vec![],
        };
        let report = check_prerequisites(&runner, &tools());

        assert!(report.idevice_id.is_ok());
        assert!(report.idevicebackup2.is_ok());
    }
}
