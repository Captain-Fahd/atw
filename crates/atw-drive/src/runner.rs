//! The [`CommandRunner`] trait: the only way atw runs external drive tools.
//!
//! [`SystemRunner`] runs real programs, but only those [`crate::safety`]
//! classes as read-only. [`FakeRunner`] replays recorded output for tests.

use std::cell::RefCell;
use std::collections::HashMap;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::safety;

/// What a command produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutput {
    /// Exit code, or `None` if the process was killed by a signal.
    pub code: Option<i32>,
    /// Standard output, decoded lossily as UTF-8.
    pub stdout: String,
    /// Standard error, decoded lossily as UTF-8.
    pub stderr: String,
}

impl CommandOutput {
    /// A successful run (exit code 0) with the given stdout.
    pub fn ok(stdout: impl Into<String>) -> Self {
        Self {
            code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    /// A failed run with the given exit code and stderr.
    pub fn failed(code: i32, stderr: impl Into<String>) -> Self {
        Self {
            code: Some(code),
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    /// True if the command exited with code 0.
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// Why a command could not be run.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The runner refused the command because it is not known to be read-only.
    #[error("refused to run `{0}`: not on the read-only allowlist")]
    Refused(String),
    /// The program could not be started (missing binary, permissions).
    #[error("could not start `{program}`: {source}")]
    Spawn {
        /// The program that failed to start.
        program: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// [`FakeRunner`] has no recorded output for this command.
    #[error("no recorded output for `{0}`")]
    NoFixture(String),
}

/// Runs an external program and returns its output.
///
/// Every drive command in atw goes through this trait so tests can swap in
/// [`FakeRunner`].
pub trait CommandRunner {
    /// Run `program` with `args` and wait for it to finish.
    ///
    /// # Errors
    /// [`RunError`] if the command is refused, can't start, or (for fakes)
    /// has no recorded output. A non-zero exit is *not* an error; check
    /// [`CommandOutput::success`].
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, RunError>;
}

/// Formats a command line for logs and fixture keys: `program arg1 arg2`.
pub fn command_line(program: &str, args: &[&str]) -> String {
    std::iter::once(program)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs real programs on this machine, read-only commands only.
///
/// # Safety
/// This runner touches real devices. It refuses anything that
/// [`safety::is_read_only`] does not recognise, so it can only issue
/// identify/query commands. Destructive commands will need a separate,
/// `live`-gated runner (Phase 2).
#[derive(Debug, Default)]
pub struct SystemRunner {
    _private: (),
}

impl SystemRunner {
    /// A runner that only allows read-only commands.
    pub fn read_only() -> Self {
        Self { _private: () }
    }
}

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, RunError> {
        if !safety::is_read_only(program, args) {
            return Err(RunError::Refused(command_line(program, args)));
        }
        let out = Command::new(program)
            .args(args)
            .env("LC_ALL", "C")
            .output()
            .map_err(|source| RunError::Spawn {
                program: program.to_string(),
                source,
            })?;
        Ok(CommandOutput {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// Replays recorded outputs, keyed by [`command_line`], and records calls.
#[derive(Debug, Default)]
pub struct FakeRunner {
    outputs: HashMap<String, CommandOutput>,
    calls: RefCell<Vec<String>>,
}

impl FakeRunner {
    /// An empty fake: every command returns [`RunError::NoFixture`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `output` for the exact command line `cmd` (e.g. `"hdparm -I /dev/sda"`).
    pub fn with(mut self, cmd: &str, output: CommandOutput) -> Self {
        self.outputs.insert(cmd.to_string(), output);
        self
    }

    /// Every command line run so far, in order.
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, RunError> {
        let cmd = command_line(program, args);
        self.calls.borrow_mut().push(cmd.clone());
        self.outputs
            .get(&cmd)
            .cloned()
            .ok_or(RunError::NoFixture(cmd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_replays_and_records() {
        let fake = FakeRunner::new().with("lsblk -J", CommandOutput::ok("{}"));
        assert_eq!(fake.run("lsblk", &["-J"]).unwrap().stdout, "{}");
        assert!(matches!(
            fake.run("hdparm", &["-I", "/dev/sda"]),
            Err(RunError::NoFixture(_))
        ));
        assert_eq!(fake.calls(), vec!["lsblk -J", "hdparm -I /dev/sda"]);
    }

    #[test]
    fn system_runner_refuses_destructive_commands() {
        let r = SystemRunner::read_only();
        for (p, a) in [
            ("hdparm", &["--security-erase", "x", "/dev/sda"][..]),
            ("nvme", &["format", "/dev/nvme0n1"][..]),
            ("blkdiscard", &["/dev/sda"][..]),
            ("dd", &["if=/dev/zero", "of=/dev/sda"][..]),
        ] {
            assert!(matches!(r.run(p, a), Err(RunError::Refused(_))), "{p}");
        }
    }
}
