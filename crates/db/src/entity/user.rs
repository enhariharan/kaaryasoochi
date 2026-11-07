// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub username: String,
    pub full_name: String,
    /// Argon2id PHC string.
    pub password_hash: String,
    pub is_admin: bool,
    /// Shown on the recovery screen; empty until set (accounts that predate the feature).
    pub secret_question: String,
    /// Argon2id PHC string of the normalised answer.
    pub secret_answer_hash: String,
    /// Wrong answers since the last success or lock.
    pub recovery_failures: i32,
    pub recovery_locked_until: Option<DateTimeUtc>,
    /// SHA-256 (hex) of the single-use reset token, or empty.
    pub reset_token_hash: String,
    pub reset_token_expires: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
