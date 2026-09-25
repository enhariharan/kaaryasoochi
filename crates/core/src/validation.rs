// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::exec::RunMode;
use crate::limits::*;
use crate::recurrence::{Repeat, RepeatError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("title is required")]
    TitleRequired,
    #[error("title must be at most {TITLE_MAX} characters")]
    TitleTooLong,
    #[error("category must be at most {CATEGORY_MAX} characters")]
    CategoryTooLong,
    #[error("summary must be at most {SUMMARY_MAX} characters")]
    SummaryTooLong,
    #[error("summary must be a single line")]
    SummaryMultiline,
    #[error("description must be at most {DESCRIPTION_MAX} characters")]
    DescriptionTooLong,
    #[error("command must be at most {COMMAND_MAX} characters")]
    CommandTooLong,
    #[error(
        "script path must be an absolute path on a single line (at most {PATH_MAX} characters)"
    )]
    ScriptPath,
    #[error("script arguments must be on a single line (at most {SCRIPT_ARGS_MAX} characters)")]
    ScriptArgs,
    #[error(
        "working directory must be an absolute path (at most {PATH_MAX} characters), or empty"
    )]
    WorkingDir,
    #[error("retries must be between 1 and {RETRY_COUNT_MAX}")]
    RetryCount,
    #[error("retry delay must be between 1 and {RETRY_DELAY_MAX_SECS} seconds")]
    RetryDelay,
    #[error("timeout must be between 1 and {TIMEOUT_MAX_SECS} seconds")]
    Timeout,
    #[error("unknown time zone")]
    Timezone,
    #[error("only administrators can set a command")]
    CommandForbidden,
    #[error("first run must be in the future")]
    RunInPast,
    #[error("invalid repeat: {0}")]
    Repeat(String),
    #[error(
        "username must be {USERNAME_MIN}-{USERNAME_MAX} characters of a-z, 0-9, '_', '-', '.'"
    )]
    Username,
    #[error("password must be {PASSWORD_MIN}-{PASSWORD_MAX} characters")]
    Password,
    #[error("full name must be at most {FULL_NAME_MAX} characters")]
    FullNameTooLong,
}

impl From<RepeatError> for ValidationError {
    fn from(e: RepeatError) -> Self {
        Self::Repeat(e.to_string())
    }
}

/// User-supplied job fields, before persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobInput {
    pub title: String,
    /// Empty/blank means the Default category.
    pub category: String,
    pub summary: String,
    pub description: String,
    pub first_run: DateTime<Utc>,
    pub repeat: Option<Repeat>,
    pub run_mode: RunMode,
    /// Shell command (used when `run_mode` is `Command`). Empty means the run is only recorded.
    pub command: String,
    /// Absolute script path and shell-syntax arguments (used when `run_mode` is `Script`).
    pub script_path: String,
    pub script_args: String,
    /// Absolute directory to run in; empty means the user's home.
    pub working_dir: String,
    pub timeout_secs: u32,
    pub notify_on_failure: bool,
    /// Re-run a failed run up to `retry_count` more times, `retry_delay_secs` apart.
    pub retry_on_failure: bool,
    pub retry_count: u32,
    pub retry_delay_secs: u32,
}

fn is_abs_single_line(s: &str) -> bool {
    s.starts_with('/') && !s.contains(['\n', '\r', '\0']) && s.chars().count() <= PATH_MAX
}

impl JobInput {
    /// True when the job carries anything to execute.
    pub fn has_exec(&self) -> bool {
        match self.run_mode {
            RunMode::Command => !self.command.is_empty(),
            RunMode::Script => !self.script_path.is_empty(),
        }
    }

    /// The line to hand to `/bin/sh -c` (empty = nothing to run).
    pub fn shell_line(&self) -> String {
        crate::exec::shell_line(
            self.run_mode,
            &self.command,
            &self.script_path,
            &self.script_args,
        )
    }
}

impl JobInput {
    /// Trims text fields, enforces limits, and returns the normalised input.
    /// `now` is injected so the future-date rule is testable.
    pub fn validate(mut self, now: DateTime<Utc>) -> Result<Self, ValidationError> {
        self.title = self.title.trim().to_owned();
        self.category = self.category.trim().to_owned();
        self.summary = self.summary.trim().to_owned();
        self.description = self.description.trim().to_owned();
        self.command = self.command.trim().to_owned();
        self.script_path = self.script_path.trim().to_owned();
        self.script_args = self.script_args.trim().to_owned();
        self.working_dir = self.working_dir.trim().to_owned();

        if self.title.is_empty() {
            return Err(ValidationError::TitleRequired);
        }
        if self.title.chars().count() > TITLE_MAX {
            return Err(ValidationError::TitleTooLong);
        }
        if self.category.chars().count() > CATEGORY_MAX {
            return Err(ValidationError::CategoryTooLong);
        }
        if self.summary.contains(['\n', '\r']) {
            return Err(ValidationError::SummaryMultiline);
        }
        if self.summary.chars().count() > SUMMARY_MAX {
            return Err(ValidationError::SummaryTooLong);
        }
        if self.description.chars().count() > DESCRIPTION_MAX {
            return Err(ValidationError::DescriptionTooLong);
        }
        if self.command.chars().count() > COMMAND_MAX {
            return Err(ValidationError::CommandTooLong);
        }
        if !self.script_path.is_empty() && !is_abs_single_line(&self.script_path) {
            return Err(ValidationError::ScriptPath);
        }
        if self.run_mode == RunMode::Script
            && self.script_path.is_empty()
            && !self.script_args.is_empty()
        {
            return Err(ValidationError::ScriptPath);
        }
        if self.script_args.contains(['\n', '\r', '\0'])
            || self.script_args.chars().count() > SCRIPT_ARGS_MAX
        {
            return Err(ValidationError::ScriptArgs);
        }
        if !self.working_dir.is_empty() && !is_abs_single_line(&self.working_dir) {
            return Err(ValidationError::WorkingDir);
        }
        if !(1..=RETRY_COUNT_MAX).contains(&self.retry_count) {
            return Err(ValidationError::RetryCount);
        }
        if !(1..=RETRY_DELAY_MAX_SECS).contains(&self.retry_delay_secs) {
            return Err(ValidationError::RetryDelay);
        }
        if !(1..=TIMEOUT_MAX_SECS).contains(&self.timeout_secs) {
            return Err(ValidationError::Timeout);
        }
        let is_cron = matches!(self.repeat, Some(Repeat::Cron(_)));
        if !is_cron && self.first_run <= now {
            return Err(ValidationError::RunInPast);
        }
        if let Some(r) = &self.repeat {
            r.validate()?;
        }
        Ok(self)
    }
}

pub fn validate_username(u: &str) -> Result<String, ValidationError> {
    let u = u.trim().to_lowercase();
    let ok = (USERNAME_MIN..=USERNAME_MAX).contains(&u.chars().count())
        && u.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    ok.then_some(u).ok_or(ValidationError::Username)
}

pub fn validate_password(p: &str) -> Result<(), ValidationError> {
    (PASSWORD_MIN..=PASSWORD_MAX)
        .contains(&p.chars().count())
        .then_some(())
        .ok_or(ValidationError::Password)
}

pub fn validate_full_name(n: &str) -> Result<String, ValidationError> {
    let n = n.trim();
    (n.chars().count() <= FULL_NAME_MAX)
        .then(|| n.to_owned())
        .ok_or(ValidationError::FullNameTooLong)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn input() -> (JobInput, DateTime<Utc>) {
        let now = Utc::now();
        (
            JobInput {
                title: " Backup ".into(),
                category: "".into(),
                summary: "".into(),
                description: "".into(),
                first_run: now + Duration::hours(1),
                repeat: None,
                run_mode: RunMode::Command,
                command: String::new(),
                script_path: String::new(),
                script_args: String::new(),
                working_dir: String::new(),
                timeout_secs: TIMEOUT_DEFAULT_SECS,
                notify_on_failure: true,
                retry_on_failure: false,
                retry_count: RETRY_COUNT_DEFAULT,
                retry_delay_secs: RETRY_DELAY_DEFAULT_SECS,
            },
            now,
        )
    }
    #[test]
    fn trims_and_accepts() {
        let (i, now) = input();
        assert_eq!(i.validate(now).unwrap().title, "Backup");
    }
    #[test]
    fn rejects_bad_fields() {
        let (mut i, now) = input();
        i.title = "  ".into();
        assert_eq!(i.validate(now), Err(ValidationError::TitleRequired));
        let (mut i, now) = input();
        i.title = "x".repeat(TITLE_MAX + 1);
        assert_eq!(i.validate(now), Err(ValidationError::TitleTooLong));
        let (mut i, now) = input();
        i.summary = "a\nb".into();
        assert_eq!(i.validate(now), Err(ValidationError::SummaryMultiline));
        let (mut i, now) = input();
        i.first_run = now - Duration::seconds(1);
        assert_eq!(i.validate(now), Err(ValidationError::RunInPast));
        let (mut i, now) = input();
        i.script_path = "relative/run.sh".into();
        assert_eq!(i.validate(now), Err(ValidationError::ScriptPath));
        let (mut i, now) = input();
        i.working_dir = "~/x".into();
        assert_eq!(i.validate(now), Err(ValidationError::WorkingDir));
        let (mut i, now) = input();
        i.script_args = "a\nb".into();
        assert_eq!(i.validate(now), Err(ValidationError::ScriptArgs));
        let (mut i, now) = input();
        i.repeat = Some(Repeat::Cron("* * * * *".parse().unwrap()));
        i.first_run = now - chrono::Duration::days(1); // "start from" may already have passed
        assert!(i.validate(now).is_ok());
        let (mut i, now) = input();
        i.retry_count = 0;
        assert_eq!(i.validate(now), Err(ValidationError::RetryCount));
        let (mut i, now) = input();
        i.retry_count = RETRY_COUNT_MAX + 1;
        assert_eq!(i.validate(now), Err(ValidationError::RetryCount));
        let (mut i, now) = input();
        i.retry_delay_secs = 0;
        assert_eq!(i.validate(now), Err(ValidationError::RetryDelay));
        let (mut i, now) = input();
        i.timeout_secs = 0;
        assert_eq!(i.validate(now), Err(ValidationError::Timeout));
        let (mut i, now) = input();
        i.repeat = Some(Repeat::EveryDays(0));
        assert!(matches!(i.validate(now), Err(ValidationError::Repeat(_))));
    }
    #[test]
    fn credentials() {
        assert_eq!(validate_username(" Alice ").unwrap(), "alice");
        assert!(validate_username("a b").is_err());
        assert!(validate_password("short").is_err());
        assert!(validate_password("long-enough-pw").is_ok());
    }
}
