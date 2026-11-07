// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use kaaryasoochi_core::ValidationError;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("username already taken")]
    UsernameTaken,
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("not authenticated")]
    Unauthenticated,
    #[error("incorrect answer")]
    WrongAnswer,
    #[error("too many wrong answers; try again in {0} minute(s)")]
    Locked(i64),
    #[error("this recovery session has expired; please start again")]
    InvalidToken,
    #[error("set a secret question in Settings first")]
    SecretNotSet,
    #[error("this job is already running")]
    AlreadyRunning,
    #[error("not found")]
    NotFound,
    #[error("the Default category cannot be renamed or deleted")]
    ProtectedCategory,
    #[error("internal error: {0}")]
    Internal(String),
}

pub type DbResult<T> = Result<T, DbError>;
