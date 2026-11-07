// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &[&str] = &[
    "ALTER TABLE user_settings ADD COLUMN button_display TEXT NOT NULL DEFAULT 'icon_text'",
];

const DOWN: &[&str] = &[
    "ALTER TABLE user_settings DROP COLUMN button_display",
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
