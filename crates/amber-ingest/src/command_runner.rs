// SPDX-License-Identifier: GPL-3.0-or-later
//! Abstracts shelling out to external tools (`idevice_id`, `idevicebackup2`,
//! `sc query`, ...) behind a trait, so device-detection, backup
//! orchestration, and progress-parsing logic can be unit tested against
//! canned command output instead of requiring real hardware.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
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

    /// Like [`run`](Self::run), but also calls `on_stderr_line` for each line
    /// of stderr (`idevicebackup2` prints its status messages there), for
    /// showing live logs. The default replays stderr after the process
    /// exits; runners that can stream it should override this.
    fn run_streaming(
        &self,
        program: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        on_stdout_line: &mut dyn FnMut(&str),
        on_stderr_line: &mut dyn FnMut(&str),
    ) -> std::io::Result<CommandOutput> {
        let output = self.run(program, args, envs, on_stdout_line)?;
        for line in output.stderr.lines() {
            on_stderr_line(line);
        }
        Ok(output)
    }
}

/// Runs real child processes, streaming stdout and stderr line-by-line.
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        on_stdout_line: &mut dyn FnMut(&str),
    ) -> std::io::Result<CommandOutput> {
        self.run_streaming(program, args, envs, on_stdout_line, &mut |_| {})
    }

    fn run_streaming(
        &self,
        program: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        on_stdout_line: &mut dyn FnMut(&str),
        on_stderr_line: &mut dyn FnMut(&str),
    ) -> std::io::Result<CommandOutput> {
        let mut child = Command::new(program)
            .args(args)
            .envs(envs.iter().copied())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Both pipes are drained on their own threads and funneled through
        // one channel, so a chatty child can't deadlock us by filling one
        // pipe while we're blocked reading the other, and the (non-`Send`)
        // callbacks all run on this thread.
        let (tx, rx) = mpsc::channel::<(bool, String)>();
        let mut readers = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            readers.push(spawn_line_reader(stdout, false, tx.clone()));
        }
        if let Some(stderr) = child.stderr.take() {
            readers.push(spawn_line_reader(stderr, true, tx.clone()));
        }
        drop(tx);

        let mut stderr_all = String::new();
        for (is_stderr, line) in rx {
            if is_stderr {
                stderr_all.push_str(&line);
                stderr_all.push('\n');
                on_stderr_line(&line);
            } else {
                on_stdout_line(&line);
            }
        }
        for reader in readers {
            let _ = reader.join();
        }

        let status = child.wait()?;
        Ok(CommandOutput {
            success: status.success(),
            stderr: stderr_all,
        })
    }
}

fn spawn_line_reader(
    pipe: impl std::io::Read + Send + 'static,
    is_stderr: bool,
    tx: mpsc::Sender<(bool, String)>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(pipe).split(b'\n').map_while(Result::ok) {
            let line = String::from_utf8_lossy(&line)
                .trim_end_matches('\r')
                .to_string();
            if tx.send((is_stderr, line)).is_err() {
                break;
            }
        }
    })
}
