// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Secret question and answer: password recovery, changing the secret, and the answer check that
//! every one of those flows (and "change password") shares.
//!
//! Threat model: the answer is a low-entropy secret, so it is (1) stored only as an Argon2id hash
//! of the normalised text, (2) rate limited, and (3) never accepted on its own: a correct answer
//! yields a short-lived single-use token, and only that token can set a new password.

use chrono::{DateTime, Duration, Utc};
use kaaryasoochi_core::validation::{
    normalize_answer, validate_new_password, validate_password, validate_secret_answer,
    validate_secret_question,
};
use sea_orm::{prelude::*, ActiveValue::Set};
use sha2::{Digest, Sha256};

use super::auth::{hash_password, revoke_sessions, token_hash, verify_password};
use crate::entity::user;
use crate::{DbError, DbResult};

/// Wrong answers allowed before recovery is locked.
pub const MAX_ANSWER_FAILURES: i32 = 5;
pub const LOCKOUT_MINUTES: i64 = 15;
/// How long the token from a correct answer stays valid.
pub const RESET_TOKEN_MINUTES: i64 = 10;

/// Shown for usernames that do not exist (or have no secret yet), so the recovery screen does not
/// reveal which accounts exist. The same username always gets the same decoy.
const DECOYS: [&str; 6] = [
    "What was the name of your first pet?",
    "In what city were you born?",
    "What was your first school called?",
    "What is your mother's maiden name?",
    "What was the make of your first car?",
    "What is the name of your favourite teacher?",
];

fn decoy(name: &str) -> &'static str {
    DECOYS[Sha256::digest(name.as_bytes())[0] as usize % DECOYS.len()]
}

fn norm_name(username: &str) -> String {
    username.trim().to_lowercase()
}

async fn find(db: &impl ConnectionTrait, name: &str) -> DbResult<Option<user::Model>> {
    Ok(user::Entity::find()
        .filter(user::Column::Username.eq(name))
        .one(db)
        .await?)
}

/// The question to show on the recovery screen. Unknown users and users who never set a secret
/// get a decoy, so the two cannot be told apart from the outside.
pub async fn question_for(db: &impl ConnectionTrait, username: &str) -> DbResult<String> {
    let name = norm_name(username);
    Ok(match find(db, &name).await? {
        Some(u) if !u.secret_question.is_empty() => u.secret_question,
        _ => decoy(&name).to_string(),
    })
}

/// Checks `answer` against the user's stored answer, with lockout. `Ok` resets the failure count;
/// a wrong answer counts, and the fifth in a row locks the account's answer checks for
/// [`LOCKOUT_MINUTES`] (even a correct answer is refused while locked).
pub(crate) async fn verify_answer_at(
    db: &impl ConnectionTrait,
    u: &user::Model,
    answer: &str,
    now: DateTime<Utc>,
) -> DbResult<()> {
    if let Some(until) = u.recovery_locked_until.filter(|t| *t > now) {
        return Err(DbError::Locked(((until - now).num_seconds() + 59) / 60));
    }
    let ok = !u.secret_answer_hash.is_empty()
        && verify_password(&normalize_answer(answer), &u.secret_answer_hash);
    let mut am: user::ActiveModel = u.clone().into();
    if ok {
        if u.recovery_failures != 0 || u.recovery_locked_until.is_some() {
            am.recovery_failures = Set(0);
            am.recovery_locked_until = Set(None);
            am.update(db).await?;
        }
        return Ok(());
    }
    let failures = u.recovery_failures + 1;
    if failures >= MAX_ANSWER_FAILURES {
        am.recovery_failures = Set(0);
        am.recovery_locked_until = Set(Some(now + Duration::minutes(LOCKOUT_MINUTES)));
    } else {
        am.recovery_failures = Set(failures);
        am.recovery_locked_until = Set(None);
    }
    am.update(db).await?;
    Err(DbError::WrongAnswer)
}

/// Step 2 of "forgot password": a correct answer returns a single-use reset token.
pub async fn start_recovery(
    db: &impl ConnectionTrait,
    username: &str,
    answer: &str,
) -> DbResult<String> {
    start_recovery_at(db, username, answer, Utc::now()).await
}

pub(crate) async fn start_recovery_at(
    db: &impl ConnectionTrait,
    username: &str,
    answer: &str,
    now: DateTime<Utc>,
) -> DbResult<String> {
    let name = norm_name(username);
    let u = match find(db, &name).await? {
        Some(u) if !u.secret_question.is_empty() => u,
        _ => {
            // Do comparable work for unknown users so response time does not give them away.
            let _ = verify_password(&normalize_answer(answer), &hash_password("dummy-answer")?);
            return Err(DbError::WrongAnswer);
        }
    };
    verify_answer_at(db, &u, answer, now).await?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let mut am: user::ActiveModel = u.into();
    am.reset_token_hash = Set(token_hash(&token));
    am.reset_token_expires = Set(Some(now + Duration::minutes(RESET_TOKEN_MINUTES)));
    am.update(db).await?;
    Ok(token)
}

/// Step 3: the token from [`start_recovery`] plus the new password typed twice. The token is
/// consumed, the password replaced and every session signed out.
pub async fn finish_recovery(
    db: &sea_orm::DatabaseConnection,
    token: &str,
    new: &str,
    confirm: &str,
) -> DbResult<()> {
    finish_recovery_at(db, token, new, confirm, Utc::now()).await
}

pub(crate) async fn finish_recovery_at(
    db: &sea_orm::DatabaseConnection,
    token: &str,
    new: &str,
    confirm: &str,
    now: DateTime<Utc>,
) -> DbResult<()> {
    validate_new_password(new, confirm)?;
    if token.is_empty() {
        return Err(DbError::InvalidToken);
    }
    let u = user::Entity::find()
        .filter(user::Column::ResetTokenHash.eq(token_hash(token)))
        .one(db)
        .await?
        .ok_or(DbError::InvalidToken)?;
    let user_id = u.id;
    let valid = u.reset_token_expires.is_some_and(|e| e > now);
    let mut am: user::ActiveModel = u.into();
    am.reset_token_hash = Set(String::new()); // single use, whether or not it was still valid
    am.reset_token_expires = Set(None);
    if !valid {
        am.update(db).await?;
        return Err(DbError::InvalidToken);
    }
    am.password_hash = Set(hash_password(new)?);
    am.recovery_failures = Set(0);
    am.recovery_locked_until = Set(None);
    am.update(db).await?;
    revoke_sessions(db, user_id).await
}

/// Changes the secret question and answer. Needs the current password and, if a secret is already
/// set, the current answer. Returns the new question.
pub async fn set_secret(
    db: &impl ConnectionTrait,
    user_id: i32,
    current_password: &str,
    current_answer: &str,
    question: &str,
    answer: &str,
) -> DbResult<String> {
    set_secret_at(
        db,
        user_id,
        current_password,
        current_answer,
        question,
        answer,
        Utc::now(),
    )
    .await
}

pub(crate) async fn set_secret_at(
    db: &impl ConnectionTrait,
    user_id: i32,
    current_password: &str,
    current_answer: &str,
    question: &str,
    answer: &str,
    now: DateTime<Utc>,
) -> DbResult<String> {
    let question = validate_secret_question(question)?;
    let answer = validate_secret_answer(answer)?;
    let u = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    if !verify_password(current_password, &u.password_hash) {
        return Err(DbError::InvalidCredentials);
    }
    if !u.secret_question.is_empty() {
        verify_answer_at(db, &u, current_answer, now).await?;
    }
    let mut am: user::ActiveModel = u.into();
    am.secret_question = Set(question.clone());
    am.secret_answer_hash = Set(hash_password(&answer)?);
    am.recovery_failures = Set(0);
    am.recovery_locked_until = Set(None);
    am.update(db).await?;
    Ok(question)
}

/// Operator reset (`scripts/reset-password`): no old password or secret needed, because whoever
/// can run it already has access to the database. Signs the user out everywhere.
pub async fn admin_reset_password(
    db: &sea_orm::DatabaseConnection,
    username: &str,
    new: &str,
) -> DbResult<()> {
    validate_password(new)?;
    let u = find(db, &norm_name(username))
        .await?
        .ok_or(DbError::NotFound)?;
    let user_id = u.id;
    let mut am: user::ActiveModel = u.into();
    am.password_hash = Set(hash_password(new)?);
    am.reset_token_hash = Set(String::new());
    am.reset_token_expires = Set(None);
    am.recovery_failures = Set(0);
    am.recovery_locked_until = Set(None);
    am.update(db).await?;
    revoke_sessions(db, user_id).await
}

/// `(username, is_admin, has_secret_question)` for every account, for the operator tool.
pub async fn list_users(db: &impl ConnectionTrait) -> DbResult<Vec<(String, bool, bool)>> {
    use sea_orm::QueryOrder;
    Ok(user::Entity::find()
        .order_by_asc(user::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(|u| (u.username, u.is_admin, !u.secret_question.is_empty()))
        .collect())
}
