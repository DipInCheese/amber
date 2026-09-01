// SPDX-License-Identifier: GPL-3.0-or-later
//! Runs `idevice_id`/`idevicebackup2` as bundled Tauri sidecars
//! (`tauri_plugin_shell`), falling back to plain `PATH` resolution when no
//! bundled sidecar is present - which is always true today, since the
//! sidecars aren't wired up yet (see `resources/bin/README.md`), and is
//! also true for a developer running `tauri dev` without having gone
//! through a bundled build.

use std::path::Path;

use amber_ingest::{CommandOutput, CommandRunner, SystemCommandRunner};
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

pub struct SidecarCommandRunner {
    app: AppHandle,
}

impl SidecarCommandRunner {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl CommandRunner for SidecarCommandRunner {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        on_stdout_line: &mut dyn FnMut(&str),
    ) -> std::io::Result<CommandOutput> {
        // `sidecar()` wants just the bundled binary's base name (see
        // externalBin in tauri.conf.json), not a full path.
        let name = program
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        match self.run_sidecar(name, args, envs, on_stdout_line) {
            Some(result) => result,
            // No bundled sidecar (not wired up yet, or a `tauri dev` run) -
            // fall back to PATH, same as local development without Tauri.
            None => SystemCommandRunner.run(Path::new(name), args, envs, on_stdout_line),
        }
    }
}

impl SidecarCommandRunner {
    /// Returns `None` when the sidecar can't be spawned at all (not
    /// bundled), so the caller can fall back to PATH. Returns `Some` once
    /// the sidecar did start - a non-zero exit or stderr output from a
    /// bundled tool is a real result, not a reason to silently fall back.
    fn run_sidecar(
        &self,
        name: &str,
        args: &[&str],
        envs: &[(&str, &str)],
        on_stdout_line: &mut dyn FnMut(&str),
    ) -> Option<std::io::Result<CommandOutput>> {
        let mut command = self.app.shell().sidecar(name).ok()?;
        command = command.args(args);
        for (key, value) in envs {
            command = command.env(key, value);
        }

        let (mut rx, _child) = command.spawn().ok()?;

        let mut stderr = Vec::new();
        let mut success = false;

        tauri::async_runtime::block_on(async {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(bytes) => {
                        on_stdout_line(&String::from_utf8_lossy(&bytes));
                    }
                    CommandEvent::Stderr(bytes) => {
                        stderr.extend_from_slice(&bytes);
                    }
                    CommandEvent::Terminated(payload) => {
                        success = payload.code == Some(0);
                    }
                    CommandEvent::Error(message) => {
                        stderr.extend_from_slice(message.as_bytes());
                    }
                    _ => {}
                }
            }
        });

        Some(Ok(CommandOutput {
            success,
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        }))
    }
}
