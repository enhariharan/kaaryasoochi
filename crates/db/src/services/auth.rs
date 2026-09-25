// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use chrono::{Duration, Utc};
use kaaryasoochi_core::dto::UserView;
use kaaryasoochi_core::validation::{validate_full_name, validate_password, validate_username};
use sea_orm::{prelude::*, ActiveValue::Set, PaginatorTrait, TransactionTrait};
use sha2::{Digest, Sha256};

use crate::entity::{session, user};
use crate::{DbError, DbResult};

pub const SESSION_DAYS: i64 = 30;

fn hash_password(pw: &str) -> DbResult<String> {
    Argon2::default()
        .hash_password(pw.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(|e| DbError::Internal(e.to_string()))
}

fn verify_password(pw: &str, phc: &str) -> bool {
    PasswordHash::new(phc)
        .is_ok_and(|h| Argon2::default().verify_password(pw.as_bytes(), &h).is_ok())
}

fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn view(u: &user::Model) -> UserView {
    UserView {
        id: u.id,
        username: u.username.clone(),
        full_name: u.full_name.clone(),
        is_admin: u.is_admin,
    }
}

/// Creates the user, their default settings row and their `Default` category atomically.
pub async fn register(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
    full_name: &str,
) -> DbResult<UserView> {
    let username = validate_username(username)?;
    validate_password(password)?;
    let full_name = validate_full_name(full_name)?;
    let password_hash = hash_password(password)?;

    let tx = db.begin().await?;
    if user::Entity::find()
        .filter(user::Column::Username.eq(&username))
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(DbError::UsernameTaken);
    }
    // The very first account becomes the administrator.
    let is_admin = user::Entity::find().count(&tx).await? == 0;
    let u = user::ActiveModel {
        is_admin: Set(is_admin),
        username: Set(username),
        full_name: Set(full_name),
        password_hash: Set(password_hash),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    super::settings::create_defaults(&tx, u.id).await?;
    super::category::ensure_default(&tx, u.id).await?;
    tx.commit().await?;
    Ok(view(&u))
}

/// Verifies credentials and returns a fresh opaque session token.
pub async fn login(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
) -> DbResult<(String, UserView)> {
    let found = user::Entity::find()
        .filter(user::Column::Username.eq(username.trim().to_lowercase()))
        .one(db)
        .await?;
    // Verify against a throwaway hash on a miss so response time doesn't reveal valid usernames.
    let ok = match &found {
        Some(u) => verify_password(password, &u.password_hash),
        None => {
            let _ = verify_password(password, &hash_password("dummy-password")?);
            false
        }
    };
    let u = found.filter(|_| ok).ok_or(DbError::InvalidCredentials)?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    session::ActiveModel {
        token_hash: Set(token_hash(&token)),
        user_id: Set(u.id),
        expires_at: Set(Utc::now() + Duration::days(SESSION_DAYS)),
    }
    .insert(db)
    .await?;
    Ok((token, view(&u)))
}

pub async fn user_for_token(db: &DatabaseConnection, token: &str) -> DbResult<UserView> {
    let s = session::Entity::find_by_id(token_hash(token))
        .one(db)
        .await?
        .ok_or(DbError::Unauthenticated)?;
    if s.expires_at <= Utc::now() {
        session::Entity::delete_by_id(s.token_hash).exec(db).await?;
        return Err(DbError::Unauthenticated);
    }
    let u = user::Entity::find_by_id(s.user_id)
        .one(db)
        .await?
        .ok_or(DbError::Unauthenticated)?;
    Ok(view(&u))
}

pub async fn logout(db: &DatabaseConnection, token: &str) -> DbResult<()> {
    session::Entity::delete_by_id(token_hash(token))
        .exec(db)
        .await?;
    Ok(())
}

/// Requires the current password; revokes every session for the user.
pub async fn change_password(
    db: &DatabaseConnection,
    user_id: i32,
    current: &str,
    new: &str,
) -> DbResult<()> {
    validate_password(new)?;
    let u = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    if !verify_password(current, &u.password_hash) {
        return Err(DbError::InvalidCredentials);
    }
    let mut am: user::ActiveModel = u.into();
    am.password_hash = Set(hash_password(new)?);
    am.update(db).await?;
    session::Entity::delete_many()
        .filter(session::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn update_full_name(
    db: &DatabaseConnection,
    user_id: i32,
    name: &str,
) -> DbResult<UserView> {
    let name = validate_full_name(name)?;
    let u = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    let mut am: user::ActiveModel = u.into();
    am.full_name = Set(name);
    Ok(view(&am.update(db).await?))
}
