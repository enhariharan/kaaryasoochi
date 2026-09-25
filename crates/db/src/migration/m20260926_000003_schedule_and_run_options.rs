// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    // Cron expression for `repeat_kind = 'cron'` (canonical 5-field text).
    "ALTER TABLE job ADD COLUMN cron_expr text NOT NULL DEFAULT ''",
    // What to run: a typed command, or a script file with arguments.
    "ALTER TABLE job ADD COLUMN run_mode varchar NOT NULL DEFAULT 'command'",
    "ALTER TABLE job ADD COLUMN script_path text NOT NULL DEFAULT ''",
    "ALTER TABLE job ADD COLUMN script_args text NOT NULL DEFAULT ''",
    "ALTER TABLE job ADD COLUMN working_dir text NOT NULL DEFAULT ''",
    "ALTER TABLE job ADD COLUMN notify_on_failure boolean NOT NULL DEFAULT 1",
];

const DOWN: &[&str] = &[
    "ALTER TABLE job DROP COLUMN notify_on_failure",
    "ALTER TABLE job DROP COLUMN working_dir",
    "ALTER TABLE job DROP COLUMN script_args",
    "ALTER TABLE job DROP COLUMN script_path",
    "ALTER TABLE job DROP COLUMN run_mode",
    "ALTER TABLE job DROP COLUMN cron_expr",
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
