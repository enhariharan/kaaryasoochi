// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Server functions: the only surface the UI uses to reach the backend.
//! Every function except `register`/`login` takes the session token.

use dioxus::prelude::*;
use kaaryasoochi_core::dto::*;
use kaaryasoochi_core::JobInput;

#[cfg(feature = "server")]
use crate::backend::{auth, db, err, require_admin, spawn_run};
#[cfg(feature = "server")]
use crate::fsbrowse;
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

/// Tells the server which zone the user's device is in (used while the time-zone setting is
/// "system"). Errors are ignored by the UI: an unknown zone just falls back to the server's.
#[server]
pub async fn report_system_timezone(token: String, timezone: String) -> Result<(), ServerFnError> {
    let u = auth(&token).await?;
    services::settings::set_system_timezone(db().await?, u.id, &timezone)
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

/// Header data for the folded run-history section: total runs and the latest one.
#[server]
pub async fn job_run_summary(token: String, job_id: i32) -> Result<RunSummary, ServerFnError> {
    let u = auth(&token).await?;
    services::run::summary(db().await?, u.id, job_id)
        .await
        .map_err(err)
}

/// "Run now": starts the saved job immediately, without changing its schedule.
#[server]
pub async fn run_job_now(token: String, id: i32) -> Result<(), ServerFnError> {
    let u = auth(&token).await?;
    let db = db().await?;
    let claim = services::run::start_manual(db, u.id, id)
        .await
        .map_err(err)?;
    spawn_run(db.clone(), claim);
    Ok(())
}

/// Lists a server directory for the script picker (administrators only).
#[server]
pub async fn browse_dir(
    token: String,
    path: String,
    show_hidden: bool,
) -> Result<DirListing, ServerFnError> {
    require_admin(&token).await?;
    fsbrowse::list_dir(&path, show_hidden).map_err(ServerFnError::new)
}

/// Whether a path exists / is a file / is executable (administrators only).
#[server]
pub async fn inspect_path(token: String, path: String) -> Result<FileInfo, ServerFnError> {
    require_admin(&token).await?;
    Ok(fsbrowse::file_info(&path))
}

/// Adds the owner execute bit to a script (administrators only).
#[server]
pub async fn make_executable(token: String, path: String) -> Result<FileInfo, ServerFnError> {
    require_admin(&token).await?;
    fsbrowse::make_executable(&path).map_err(ServerFnError::new)
}
