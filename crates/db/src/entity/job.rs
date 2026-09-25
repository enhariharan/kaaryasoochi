// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "job")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub category_id: i32,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub first_run: DateTimeUtc,
    /// Encoded via `Repeat::encode`; both NULL when the job does not repeat.
    pub repeat_kind: Option<String>,
    pub repeat_value: Option<i32>,
    /// Canonical cron text when `repeat_kind` is `cron`, else empty.
    pub cron_expr: String,
    /// Next scheduled run; NULL once a non-repeating job has run.
    pub next_run: Option<DateTimeUtc>,
    /// Shell command (run via `/bin/sh -c`); empty means only record the run.
    pub command: String,
    /// `command` or `script`.
    pub run_mode: String,
    pub script_path: String,
    pub script_args: String,
    /// Absolute directory to run in; empty means the user's home.
    pub working_dir: String,
    pub timeout_secs: i32,
    pub notify_on_failure: bool,
    pub retry_on_failure: bool,
    pub retry_count: i32,
    pub retry_delay_secs: i32,
    /// Next automatic retry, if one is pending.
    pub retry_at: Option<DateTimeUtc>,
    /// Number of the last failed attempt, while a retry is pending.
    pub retry_attempt: i32,
    /// IANA zone the schedule's calendar rules use, snapshotted when scheduled.
    pub timezone: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
