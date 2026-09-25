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
    /// Next scheduled run; NULL once a non-repeating job has run.
    pub next_run: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
