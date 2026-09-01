// SPDX-License-Identifier: GPL-3.0-or-later
//! Detects an iPhone over USB and creates/refreshes a local backup of it,
//! by shelling out to bundled `idevice_id` / `idevicebackup2` sidecar
//! binaries (the libimobiledevice project) - the simplest way to get device
//! backup support without vendoring the whole C stack or depending on the
//! pre-1.0 pure-Rust `idevice` crate for the core backup path.
//!
//! **Not exercised against real hardware in this environment** (no iPhone
//! or Mac available). The command syntax and progress-line format below are
//! verified against `idevicebackup2`'s own `--help` text and source
//! (`draw_progress_bar`/`progress_render` in `tools/idevicebackup2.c`,
//! https://github.com/libimobiledevice/libimobiledevice), but the
//! end-to-end flow has only been exercised via the injected `CommandRunner`
//! in tests below, not a live device.

use std::path::{Path, PathBuf};

use crate::command_runner::CommandRunner;
use crate::error::IngestError;

/// Paths to the bundled `idevice_id` and `idevicebackup2` sidecar binaries.
/// Resolving *where* those live (a Tauri `externalBin` resource dir vs.
/// PATH for local development) is the caller's job - this crate only knows
/// how to invoke them once it has paths.
#[derive(Debug, Clone)]
pub struct Tools {
    pub idevice_id: PathBuf,
    pub idevicebackup2: PathBuf,
}

/// List the UDIDs of iPhones currently connected over USB and trusted.
///
/// An empty result means either no device is connected, or it hasn't been
/// trusted yet ("Trust This Computer?" not yet accepted on the phone) -
/// `idevice_id` can't distinguish those cases from its output alone, so
/// callers should show generic "plug in and trust your phone" guidance.
pub fn list_devices(runner: &dyn CommandRunner, tools: &Tools) -> Result<Vec<String>, IngestError> {
    let mut stdout_lines = Vec::new();
    let output = runner.run(&tools.idevice_id, &["-l"], &[], &mut |line| {
        stdout_lines.push(line.to_string());
    })?;

    if !output.success {
        return Err(IngestError::ToolFailed {
            tool: "idevice_id",
            status: "non-zero exit".to_string(),
            stderr: fallback_detail(output.stderr, &stdout_lines),
        });
    }

    Ok(stdout_lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect())
}

/// Resolve which device to back up: the explicitly-requested UDID, or the
/// sole connected device if there's exactly one.
pub fn select_device(
    runner: &dyn CommandRunner,
    tools: &Tools,
    requested_udid: Option<&str>,
) -> Result<String, IngestError> {
    if let Some(udid) = requested_udid {
        return Ok(udid.to_string());
    }

    let devices = list_devices(runner, tools)?;
    match devices.as_slice() {
        [] => Err(IngestError::NoDeviceFound),
        [only] => Ok(only.clone()),
        multiple => Err(IngestError::AmbiguousDevice(multiple.to_vec())),
    }
}

/// Create or refresh (incremental - `idevicebackup2` only transfers changed
/// data on subsequent runs) a full backup of `udid` into `backup_root`,
/// reporting 0-100 progress via `on_progress` as it goes.
///
/// If the device already has encrypted backups turned on, `password` must
/// be `Some` - passed via the `BACKUP_PASSWORD` environment variable
/// (`idevicebackup2`'s own documented mechanism for non-interactive runs;
/// it has no `--password` flag for the `backup` subcommand). Amber never
/// sets encryption on/off itself.
///
/// Returns the specific device's backup directory
/// (`backup_root/<udid>`, `idevicebackup2`'s own layout).
pub fn create_or_refresh_backup(
    runner: &dyn CommandRunner,
    tools: &Tools,
    udid: &str,
    backup_root: &Path,
    password: Option<&str>,
    mut on_progress: impl FnMut(f32),
) -> Result<PathBuf, IngestError> {
    std::fs::create_dir_all(backup_root)?;

    let backup_root_str = backup_root.to_string_lossy().into_owned();
    let args = ["-u", udid, "backup", "--full", &backup_root_str];
    let envs: Vec<(&str, &str)> = password
        .map(|password| vec![("BACKUP_PASSWORD", password)])
        .unwrap_or_default();

    let mut stdout_lines = Vec::new();
    let output = runner.run(&tools.idevicebackup2, &args, &envs, &mut |line| {
        stdout_lines.push(line.to_string());
        if let Some(percent) = parse_progress_line(line) {
            on_progress(percent);
        }
    })?;

    if !output.success {
        return Err(IngestError::ToolFailed {
            tool: "idevicebackup2",
            status: "non-zero exit".to_string(),
            stderr: fallback_detail(output.stderr, &stdout_lines),
        });
    }

    Ok(backup_root.join(udid))
}

/// `idevice_id`/`idevicebackup2` often write their actual error text to
/// stdout rather than stderr (they're primarily progress-bar tools, not
/// well-behaved Unix citizens) - falling back to stdout when stderr is
/// empty avoids surfacing a blank, useless error message on a real
/// failure.
fn fallback_detail(stderr: String, stdout_lines: &[String]) -> String {
    if !stderr.trim().is_empty() {
        stderr
    } else {
        stdout_lines.join("\n")
    }
}

/// Extracts a `NN%` (or `NN.N%`) progress percentage from one line of
/// `idevicebackup2`'s stdout, per its `" %3.0f%%"` progress format
/// (`tools/idevicebackup2.c`). Returns the *last* match on the line, since
/// the progress bar itself doesn't contain a `%` character but summary
/// lines could plausibly mention one earlier.
fn parse_progress_line(line: &str) -> Option<f32> {
    let bytes = line.as_bytes();
    let mut best: Option<f32> = None;

    for (i, &b) in bytes.iter().enumerate() {
        if b != b'%' {
            continue;
        }
        let mut start = i;
        while start > 0 {
            let c = bytes[start - 1];
            if c.is_ascii_digit() || c == b'.' {
                start -= 1;
            } else {
                break;
            }
        }
        if start == i {
            continue; // no digits immediately before '%'
        }
        if let Ok(value) = line[start..i].parse::<f32>() {
            if (0.0..=100.0).contains(&value) {
                best = Some(value);
            }
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_runner::CommandOutput;
    use std::sync::Mutex;

    type CapturedCall = (PathBuf, Vec<String>, Vec<(String, String)>);

    struct FakeCommandRunner {
        stdout_lines: Vec<String>,
        success: bool,
        stderr: String,
        /// Captures the (program, args, envs) of the last call, for
        /// assertions on exactly what would have been shelled out to.
        last_call: Mutex<Option<CapturedCall>>,
    }

    impl CommandRunner for FakeCommandRunner {
        fn run(
            &self,
            program: &Path,
            args: &[&str],
            envs: &[(&str, &str)],
            on_stdout_line: &mut dyn FnMut(&str),
        ) -> std::io::Result<CommandOutput> {
            *self.last_call.lock().unwrap() = Some((
                program.to_path_buf(),
                args.iter().map(|s| s.to_string()).collect(),
                envs.iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ));
            for line in &self.stdout_lines {
                on_stdout_line(line);
            }
            Ok(CommandOutput {
                success: self.success,
                stderr: self.stderr.clone(),
            })
        }
    }

    fn tools() -> Tools {
        Tools {
            idevice_id: PathBuf::from("idevice_id"),
            idevicebackup2: PathBuf::from("idevicebackup2"),
        }
    }

    #[test]
    fn parses_udids_one_per_line_ignoring_blank_lines() {
        let runner = FakeCommandRunner {
            stdout_lines: vec![
                "00008030-000000000000000".to_string(),
                String::new(),
                "00008110-111111111111111".to_string(),
            ],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };

        let devices = list_devices(&runner, &tools()).unwrap();
        assert_eq!(
            devices,
            vec!["00008030-000000000000000", "00008110-111111111111111"]
        );
    }

    #[test]
    fn no_devices_is_not_an_error_from_list_devices() {
        let runner = FakeCommandRunner {
            stdout_lines: vec![],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        assert_eq!(
            list_devices(&runner, &tools()).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn select_device_prefers_explicit_udid_without_calling_list_devices() {
        let runner = FakeCommandRunner {
            stdout_lines: vec!["should-not-be-used".to_string()],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        let udid = select_device(&runner, &tools(), Some("explicit-udid")).unwrap();
        assert_eq!(udid, "explicit-udid");
        assert!(runner.last_call.lock().unwrap().is_none());
    }

    #[test]
    fn select_device_picks_the_sole_connected_device() {
        let runner = FakeCommandRunner {
            stdout_lines: vec!["only-one".to_string()],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        assert_eq!(select_device(&runner, &tools(), None).unwrap(), "only-one");
    }

    #[test]
    fn select_device_errors_on_zero_devices() {
        let runner = FakeCommandRunner {
            stdout_lines: vec![],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        assert!(matches!(
            select_device(&runner, &tools(), None),
            Err(IngestError::NoDeviceFound)
        ));
    }

    #[test]
    fn select_device_errors_on_multiple_devices_without_a_udid() {
        let runner = FakeCommandRunner {
            stdout_lines: vec!["a".to_string(), "b".to_string()],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        assert!(matches!(
            select_device(&runner, &tools(), None),
            Err(IngestError::AmbiguousDevice(_))
        ));
    }

    #[test]
    fn create_or_refresh_backup_builds_the_documented_command_line() {
        let runner = FakeCommandRunner {
            stdout_lines: vec![],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        let backup_root = tempfile::tempdir().unwrap();

        create_or_refresh_backup(
            &runner,
            &tools(),
            "UDID123",
            backup_root.path(),
            None,
            |_| {},
        )
        .unwrap();

        let (program, args, envs) = runner.last_call.lock().unwrap().clone().unwrap();
        assert_eq!(program, PathBuf::from("idevicebackup2"));
        assert_eq!(
            args,
            vec![
                "-u",
                "UDID123",
                "backup",
                "--full",
                backup_root.path().to_string_lossy().as_ref()
            ]
        );
        assert!(envs.is_empty());
    }

    #[test]
    fn create_or_refresh_backup_passes_password_via_env_var_not_a_flag() {
        let runner = FakeCommandRunner {
            stdout_lines: vec![],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        let backup_root = tempfile::tempdir().unwrap();

        create_or_refresh_backup(
            &runner,
            &tools(),
            "UDID123",
            backup_root.path(),
            Some("hunter2"),
            |_| {},
        )
        .unwrap();

        let (_, args, envs) = runner.last_call.lock().unwrap().clone().unwrap();
        assert!(!args.iter().any(|a| a.contains("hunter2")));
        assert_eq!(
            envs,
            vec![("BACKUP_PASSWORD".to_string(), "hunter2".to_string())]
        );
    }

    #[test]
    fn create_or_refresh_backup_reports_progress_from_stdout() {
        let runner = FakeCommandRunner {
            stdout_lines: vec![
                "Backup [####......] 40%".to_string(),
                "Backup [##########] 100%".to_string(),
                "Backup Successful.".to_string(),
            ],
            success: true,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        let backup_root = tempfile::tempdir().unwrap();

        let mut seen = Vec::new();
        create_or_refresh_backup(
            &runner,
            &tools(),
            "UDID123",
            backup_root.path(),
            None,
            |p| {
                seen.push(p);
            },
        )
        .unwrap();

        assert_eq!(seen, vec![40.0, 100.0]);
    }

    #[test]
    fn create_or_refresh_backup_surfaces_tool_failure() {
        let runner = FakeCommandRunner {
            stdout_lines: vec![],
            success: false,
            stderr: "Could not connect to device".to_string(),
            last_call: Mutex::new(None),
        };
        let backup_root = tempfile::tempdir().unwrap();

        let result = create_or_refresh_backup(
            &runner,
            &tools(),
            "UDID123",
            backup_root.path(),
            None,
            |_| {},
        );
        assert!(matches!(result, Err(IngestError::ToolFailed { .. })));
    }

    #[test]
    fn create_or_refresh_backup_falls_back_to_stdout_when_stderr_is_empty() {
        // idevicebackup2 often writes its actual error to stdout, not
        // stderr - a blank stderr on failure shouldn't produce a blank
        // error message.
        let runner = FakeCommandRunner {
            stdout_lines: vec![
                "ERROR: Could not start service com.apple.mobilebackup2.".to_string()
            ],
            success: false,
            stderr: String::new(),
            last_call: Mutex::new(None),
        };
        let backup_root = tempfile::tempdir().unwrap();

        let result = create_or_refresh_backup(
            &runner,
            &tools(),
            "UDID123",
            backup_root.path(),
            None,
            |_| {},
        );
        match result {
            Err(IngestError::ToolFailed { stderr, .. }) => {
                assert!(stderr.contains("Could not start service"));
            }
            other => panic!("expected ToolFailed with stdout fallback, got {other:?}"),
        }
    }

    #[test]
    fn parse_progress_line_extracts_whole_percentages() {
        assert_eq!(parse_progress_line("Backup [####......]  40%"), Some(40.0));
        assert_eq!(parse_progress_line("[##########] 100%"), Some(100.0));
        assert_eq!(parse_progress_line("Backup Successful."), None);
        assert_eq!(parse_progress_line("Received 12 files from device."), None);
    }
}
