// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Retries are opt-in: existing jobs get `retry_on_failure = 0`. The count/delay columns hold
// the values shown when the user ticks the box (3 retries, 60 seconds apart).
const UP: &[&str] = &[
    "ALTER TABLE job ADD COLUMN retry_on_failure boolean NOT NULL DEFAULT 0",
    "ALTER TABLE job ADD COLUMN retry_count integer NOT NULL DEFAULT 3",
    "ALTER TABLE job ADD COLUMN retry_delay_secs integer NOT NULL DEFAULT 60",
    // A pending retry survives a server restart: due time, and how many attempts failed so far.
    "ALTER TABLE job ADD COLUMN retry_at timestamp_text NULL",
    "ALTER TABLE job ADD COLUMN retry_attempt integer NOT NULL DEFAULT 0",
    "ALTER TABLE job_run ADD COLUMN attempt integer NOT NULL DEFAULT 1",
];

const DOWN: &[&str] = &[
    "ALTER TABLE job_run DROP COLUMN attempt",
    "ALTER TABLE job DROP COLUMN retry_attempt",
    "ALTER TABLE job DROP COLUMN retry_at",
    "ALTER TABLE job DROP COLUMN retry_delay_secs",
    "ALTER TABLE job DROP COLUMN retry_count",
    "ALTER TABLE job DROP COLUMN retry_on_failure",
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
