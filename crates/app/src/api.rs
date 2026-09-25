// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

//! Server functions: the only surface the UI uses to reach the backend.
//! Every function except `register`/`login` takes the session token.

use dioxus::prelude::*;
use kaaryasoochi_core::dto::*;
use kaaryasoochi_core::JobInput;

#[cfg(feature = "server")]
use crate::backend::{auth, db, err};
#[cfg(feature = "server")]
use kaaryasoochi_db::services;

#[server]
pub async fn register(
    username: String,
    password: String,
    full_name: String,
) -> Result<(), ServerFnError> {
    services::auth::register(db().await?, &username, &password, &full_name)
        .await
        .map(|_| ())
        .map_err(err)
}

#[server]
pub async fn login(username: String, password: String) -> Result<String, ServerFnError> {
    services::auth::login(db().await?, &username, &password)
        .await
        .map(|(t, _)| t)
        .map_err(err)
}

#[server]
pub async fn logout(token: String) -> Result<(), ServerFnError> {
    services::auth::logout(db().await?, &token)
        .await
        .map_err(err)
}

#[server]
pub async fn me(token: String) -> Result<UserView, ServerFnError> {
    auth(&token).await
}

#[server]
pub async fn get_settings(token: String) -> Result<UserSettings, ServerFnError> {
    let u = auth(&token).await?;
    services::settings::get(db().await?, u.id)
        .await
        .map_err(err)
}

#[server]
pub async fn update_settings(
    token: String,
    settings: UserSettings,
) -> Result<UserSettings, ServerFnError> {
    let u = auth(&token).await?;
    services::settings::update(db().await?, u.id, settings)
        .await
        .map_err(err)
}

#[server]
pub async fn update_full_name(token: String, full_name: String) -> Result<UserView, ServerFnError> {
    let u = auth(&token).await?;
    services::auth::update_full_name(db().await?, u.id, &full_name)
        .await
        .map_err(err)
}

#[server]
pub async fn change_password(
    token: String,
    current: String,
    new: String,
) -> Result<(), ServerFnError> {
    let u = auth(&token).await?;
    services::auth::change_password(db().await?, u.id, &current, &new)
        .await
        .map_err(err)
}

#[server]
pub async fn list_categories(token: String) -> Result<Vec<CategoryView>, ServerFnError> {
    let u = auth(&token).await?;
    services::category::list(db().await?, u.id)
        .await
        .map_err(err)
}

#[server]
pub async fn delete_category(token: String, id: i32) -> Result<(), ServerFnError> {
    let u = auth(&token).await?;
    services::category::delete(db().await?, u.id, id)
        .await
        .map_err(err)
}

#[server]
pub async fn list_jobs(token: String) -> Result<Vec<JobView>, ServerFnError> {
    let u = auth(&token).await?;
    services::job::list(db().await?, u.id).await.map_err(err)
}

#[server]
pub async fn get_job(token: String, id: i32) -> Result<JobView, ServerFnError> {
    let u = auth(&token).await?;
    services::job::get(db().await?, u.id, id).await.map_err(err)
}

/// Creates the job when `id` is `None`, otherwise updates it.
#[server]
pub async fn save_job(
    token: String,
    id: Option<i32>,
    input: JobInput,
) -> Result<JobView, ServerFnError> {
    let u = auth(&token).await?;
    let db = db().await?;
    match id {
        Some(id) => services::job::update(db, u.id, id, input).await,
        None => services::job::create(db, u.id, input).await,
    }
    .map_err(err)
}

#[server]
pub async fn delete_job(token: String, id: i32) -> Result<(), ServerFnError> {
    let u = auth(&token).await?;
    services::job::delete(db().await?, u.id, id)
        .await
        .map_err(err)
}

#[server]
pub async fn job_history(
    token: String,
    job_id: i32,
    page: u32,
    page_size: u32,
) -> Result<Page<RunView>, ServerFnError> {
    let u = auth(&token).await?;
    services::run::history(db().await?, u.id, job_id, page, page_size.min(100))
        .await
        .map_err(err)
}
