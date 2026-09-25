// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}

impl JobInput {
    /// Trims text fields, enforces limits, and returns the normalised input.
    /// `now` is injected so the future-date rule is testable.
    pub fn validate(mut self, now: DateTime<Utc>) -> Result<Self, ValidationError> {
        self.title = self.title.trim().to_owned();
        self.category = self.category.trim().to_owned();
        self.summary = self.summary.trim().to_owned();
        self.description = self.description.trim().to_owned();

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
        if self.first_run <= now {
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
