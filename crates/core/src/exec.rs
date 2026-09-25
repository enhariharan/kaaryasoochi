// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! What a job runs: either a typed shell command or a script file (plus arguments).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RunMode {
    /// A shell command line, run with `/bin/sh -c`.
    #[default]
    Command,
    /// An absolute path to a script/program, with optional shell-syntax arguments.
    Script,
}

impl RunMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Script => "script",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "script" {
            Self::Script
        } else {
            Self::Command
        }
    }
}

/// POSIX single-quote `s` so it is one literal word for `/bin/sh`.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// The line handed to `/bin/sh -c`. For scripts the path is quoted (so spaces are safe) and
/// `args` is appended as typed, so users can write quoting, globs and redirects there.
/// Empty when there is nothing to run.
pub fn shell_line(mode: RunMode, command: &str, script_path: &str, script_args: &str) -> String {
    match mode {
        RunMode::Command => command.trim().to_owned(),
        RunMode::Script if script_path.trim().is_empty() => String::new(),
        RunMode::Script => {
            let args = script_args.trim();
            let path = shell_quote(script_path.trim());
            if args.is_empty() {
                path
            } else {
                format!("{path} {args}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_is_safe() {
        assert_eq!(shell_quote("/a b/c.sh"), "'/a b/c.sh'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_quote("$(rm -rf ~)"), "'$(rm -rf ~)'");
    }

    #[test]
    fn builds_the_shell_line() {
        assert_eq!(
            shell_line(RunMode::Command, "  echo hi  ", "/ignored", "ignored"),
            "echo hi"
        );
        assert_eq!(
            shell_line(RunMode::Script, "ignored", "/opt/my scripts/run.sh", ""),
            "'/opt/my scripts/run.sh'"
        );
        assert_eq!(
            shell_line(
                RunMode::Script,
                "",
                "/opt/run.sh",
                "--day \"$(date +%F)\" >> /tmp/x.log"
            ),
            "'/opt/run.sh' --day \"$(date +%F)\" >> /tmp/x.log"
        );
        assert_eq!(shell_line(RunMode::Script, "echo hi", "  ", ""), "");
    }
}
