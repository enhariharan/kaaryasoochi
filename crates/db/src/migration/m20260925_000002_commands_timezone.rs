// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "ALTER TABLE user ADD COLUMN is_admin boolean NOT NULL DEFAULT 0",
    // The first account to exist is the administrator (matches what `register` does on a new DB).
    "UPDATE user SET is_admin = 1 WHERE id = (SELECT MIN(id) FROM user)",
    "ALTER TABLE user_settings ADD COLUMN timezone varchar NOT NULL DEFAULT 'system'",
    "ALTER TABLE user_settings ADD COLUMN system_timezone varchar NOT NULL DEFAULT ''",
    "ALTER TABLE job ADD COLUMN command text NOT NULL DEFAULT ''",
    "ALTER TABLE job ADD COLUMN timeout_secs integer NOT NULL DEFAULT 3600",
    "ALTER TABLE job ADD COLUMN timezone varchar NOT NULL DEFAULT 'UTC'",
    "ALTER TABLE job_run ADD COLUMN exit_code integer NULL",
];

const DOWN: &[&str] = &[
    "ALTER TABLE job_run DROP COLUMN exit_code",
    "ALTER TABLE job DROP COLUMN timezone",
    "ALTER TABLE job DROP COLUMN timeout_secs",
    "ALTER TABLE job DROP COLUMN command",
    "ALTER TABLE user_settings DROP COLUMN system_timezone",
    "ALTER TABLE user_settings DROP COLUMN timezone",
    "ALTER TABLE user DROP COLUMN is_admin",
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
