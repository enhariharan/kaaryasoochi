// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use sea_orm_migration::prelude::*;

mod m20260101_000001_init;
mod m20260925_000002_commands_timezone;
mod m20260926_000003_schedule_and_run_options;
mod m20260927_000004_retries;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260101_000001_init::Migration),
            Box::new(m20260925_000002_commands_timezone::Migration),
            Box::new(m20260926_000003_schedule_and_run_options::Migration),
            Box::new(m20260927_000004_retries::Migration),
        ]
    }
}
