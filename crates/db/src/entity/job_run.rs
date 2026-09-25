// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "job_run")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub job_id: i32,
    pub scheduled_for: DateTimeUtc,
    pub started_at: DateTimeUtc,
    pub finished_at: Option<DateTimeUtc>,
    /// "running" | "success" | "failed"
    pub status: String,
    pub exit_code: Option<i32>,
    /// 1 = the scheduled run, 2.. = automatic retries of it.
    pub attempt: i32,
    pub message: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
