// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

//! Persistence and business services on top of SQLite (SeaORM).

pub mod entity;
pub mod error;
pub mod migration;
pub mod services;

pub use error::{DbError, DbResult};
pub use sea_orm::DatabaseConnection;

use sea_orm::{ConnectOptions, ConnectionTrait, Database};
use sea_orm_migration::MigratorTrait;

/// Connect (creating the file if needed), enable foreign keys, and run migrations.
/// `url` examples: `sqlite://kaaryasoochi.db?mode=rwc`, `sqlite::memory:`.
pub async fn connect(url: &str) -> DbResult<DatabaseConnection> {
    let mut opt = ConnectOptions::new(url);
    // An in-memory SQLite DB is per-connection, so pin the pool to one.
    if url.contains(":memory:") {
        opt.max_connections(1);
    }
    opt.sqlx_logging(false);
    let db = Database::connect(opt).await?;
    db.execute_unprepared("PRAGMA foreign_keys = ON").await?;
    migration::Migrator::up(&db, None).await?;
    Ok(db)
}

#[cfg(test)]
mod tests;
