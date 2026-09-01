// SPDX-License-Identifier: GPL-3.0-or-later
//! Abstracts shelling out to external tools (`idevice_id`, `idevicebackup2`,
//! `sc query`, ...) behind a trait, so device-detection, backup
//! orchestration, and progress-parsing logic can be unit tested against
//! canned command output instead of requiring real hardware.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub stderr: String,
}

pub trait CommandRunner: Send + Sync {
    /// Runs `program` with `args` and `envs`, calling `on_stdout_line` for
    /// each line of stdout as the process produces it (e.g. `idevicebackup2`'s
    /// progress output), and returns once the process exits.
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        on_stdout_line: &mut dyn FnMut(&str),
    ) -> std::io::Result<CommandOutput>;
}

/// Runs real child processes, streaming stdout line-by-line.
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        on_stdout_line: &mut dyn FnMut(&str),
    ) -> std::io::Result<CommandOutput> {
        let mut child = Command::new(program)
            .args(args)
            .envs(envs.iter().copied())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Drain stderr on its own thread so a chatty child can't deadlock
        // us by filling the stderr pipe while we're blocked reading stdout.
        let stderr_handle = child.stderr.take().map(|mut stderr| {
            thread::spawn(move || {
                let mut buf = String::new();
                let _ = stderr.read_to_string(&mut buf);
                buf
            })
        });

        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => on_stdout_line(&line),
                    Err(_) => break,
                }
            }
        }

        let status = child.wait()?;
        let stderr = stderr_handle
            .and_then(|h| h.join().ok())
            .unwrap_or_default();

        Ok(CommandOutput {
            success: status.success(),
            stderr,
        })
    }
}
