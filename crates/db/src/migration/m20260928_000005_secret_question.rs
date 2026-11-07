// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Existing accounts get an empty question/answer: they can still log in, and are prompted to set
// one in Settings (password recovery and password changes need it).
const UP: &[&str] = &[
    "ALTER TABLE user ADD COLUMN secret_question text NOT NULL DEFAULT ''",
    // Argon2id of the *normalised* answer; never the answer itself.
    "ALTER TABLE user ADD COLUMN secret_answer_hash text NOT NULL DEFAULT ''",
    // Brute-force protection for every flow that checks the answer.
    "ALTER TABLE user ADD COLUMN recovery_failures integer NOT NULL DEFAULT 0",
    "ALTER TABLE user ADD COLUMN recovery_locked_until timestamp_text NULL",
    // Single-use token handed out after a correct answer; only its SHA-256 is stored.
    "ALTER TABLE user ADD COLUMN reset_token_hash text NOT NULL DEFAULT ''",
    "ALTER TABLE user ADD COLUMN reset_token_expires timestamp_text NULL",
];

const DOWN: &[&str] = &[
    "ALTER TABLE user DROP COLUMN reset_token_expires",
    "ALTER TABLE user DROP COLUMN reset_token_hash",
    "ALTER TABLE user DROP COLUMN recovery_locked_until",
    "ALTER TABLE user DROP COLUMN recovery_failures",
    "ALTER TABLE user DROP COLUMN secret_answer_hash",
    "ALTER TABLE user DROP COLUMN secret_question",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        for sql in UP {
            m.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        for sql in DOWN {
            m.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
