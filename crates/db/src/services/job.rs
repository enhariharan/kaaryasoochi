// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use chrono::{DateTime, Duration, Utc};
use kaaryasoochi_core::dto::{JobView, LastRun, RunStatus};
use kaaryasoochi_core::timezone::Tz;
use kaaryasoochi_core::{JobInput, Repeat, RunMode, ValidationError};
use sea_orm::{prelude::*, ActiveValue::Set, QueryOrder};

use crate::entity::{category, job, job_run, user};
use crate::services::category::get_or_create;
use crate::services::settings::effective_timezone;
use crate::{DbError, DbResult};

/// The stored repeat rule, if it decodes.
pub(crate) fn repeat_of(m: &job::Model) -> Option<Repeat> {
    m.repeat_kind
        .as_deref()
        .zip(m.repeat_value)
        .and_then(|(k, v)| Repeat::decode_full(k, v as u32, &m.cron_expr).ok())
}

fn view(m: job::Model, category: String, last_run: Option<LastRun>) -> JobView {
    JobView {
        repeat: repeat_of(&m),
        id: m.id,
        title: m.title,
        category,
        summary: m.summary,
        description: m.description,
        first_run: m.first_run,
        next_run: m.next_run,
        run_mode: RunMode::parse(&m.run_mode),
        command: m.command,
        script_path: m.script_path,
        script_args: m.script_args,
        working_dir: m.working_dir,
        timeout_secs: m.timeout_secs as u32,
        notify_on_failure: m.notify_on_failure,
        retry_on_failure: m.retry_on_failure,
        retry_count: m.retry_count as u32,
        retry_delay_secs: m.retry_delay_secs as u32,
        retry_at: m.retry_at,
        last_run,
        timezone: m.timezone,
    }
}

async fn last_run(db: &impl ConnectionTrait, job_id: i32) -> DbResult<Option<LastRun>> {
    Ok(job_run::Entity::find()
        .filter(job_run::Column::JobId.eq(job_id))
        .order_by_desc(job_run::Column::StartedAt)
        .order_by_desc(job_run::Column::Id)
        .one(db)
        .await?
        .map(|r| LastRun {
            status: RunStatus::parse(&r.status),
            started_at: r.started_at,
            exit_code: r.exit_code,
        }))
}

/// `(repeat_kind, repeat_value, cron_expr)` columns for a repeat rule.
fn repeat_cols(r: Option<Repeat>) -> (Option<String>, Option<i32>, String) {
    match r {
        Some(r) => {
            let (k, v) = r.encode();
            (
                Some(k.into()),
                Some(v as i32),
                r.cron_text().unwrap_or_default(),
            )
        }
        None => (None, None, String::new()),
    }
}

/// When a job first becomes due. Cron schedules use the first match on or after the start
/// time (and after now); everything else runs at the chosen first-run time.
fn initial_next_run(
    repeat: Option<Repeat>,
    first_run: DateTime<Utc>,
    now: DateTime<Utc>,
    tz: Tz,
) -> Option<DateTime<Utc>> {
    match repeat {
        Some(r @ Repeat::Cron(_)) => r.next_after(first_run, now, tz),
        _ => Some(first_run),
    }
}

/// Commands run as the server's OS user, so only administrators may set or change what a job
/// executes (typed command, script path, arguments or working directory).
async fn require_admin_for_exec(
    db: &impl ConnectionTrait,
    user_id: i32,
    changed: bool,
) -> DbResult<()> {
    if !changed {
        return Ok(());
    }
    let u = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    if u.is_admin {
        Ok(())
    } else {
        Err(ValidationError::CommandForbidden.into())
    }
}

pub async fn create(db: &impl ConnectionTrait, user_id: i32, input: JobInput) -> DbResult<JobView> {
    let now = Utc::now();
    let input = input.validate(now)?;
    require_admin_for_exec(
        db,
        user_id,
        input.has_exec() || !input.working_dir.is_empty(),
    )
    .await?;
    let cat = get_or_create(db, user_id, &input.category).await?;
    let tz = effective_timezone(db, user_id).await?;
    let (rk, rv, cron) = repeat_cols(input.repeat);
    let next_run = initial_next_run(input.repeat, input.first_run, now, tz);
    let m = job::ActiveModel {
        user_id: Set(user_id),
        category_id: Set(cat.id),
        title: Set(input.title),
        summary: Set(input.summary),
        description: Set(input.description),
        first_run: Set(input.first_run),
        repeat_kind: Set(rk),
        repeat_value: Set(rv),
        cron_expr: Set(cron),
        next_run: Set(next_run),
        run_mode: Set(input.run_mode.as_str().into()),
        command: Set(input.command),
        script_path: Set(input.script_path),
        script_args: Set(input.script_args),
        working_dir: Set(input.working_dir),
        timeout_secs: Set(input.timeout_secs as i32),
        notify_on_failure: Set(input.notify_on_failure),
        retry_on_failure: Set(input.retry_on_failure),
        retry_count: Set(input.retry_count as i32),
        retry_delay_secs: Set(input.retry_delay_secs as i32),
        timezone: Set(tz.name().to_owned()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(view(m, cat.name, None))
}

pub async fn update(
    db: &impl ConnectionTrait,
    user_id: i32,
    id: i32,
    input: JobInput,
) -> DbResult<JobView> {
    let existing = find_owned(db, user_id, id).await?;
    let now = Utc::now();
    // An unchanged first-run may legitimately be in the past (the job already ran),
    // so only enforce the future rule when the user actually moved it.
    let check_now = if input.first_run == existing.first_run {
        input.first_run - Duration::seconds(1)
    } else {
        now
    };
    let input = input.validate(check_now)?;
    let exec_changed = RunMode::parse(&existing.run_mode) != input.run_mode
        || existing.command != input.command
        || existing.script_path != input.script_path
        || existing.script_args != input.script_args
        || existing.working_dir != input.working_dir;
    require_admin_for_exec(db, user_id, exec_changed).await?;
    let cat = get_or_create(db, user_id, &input.category).await?;
    let (rk, rv, cron) = repeat_cols(input.repeat);
    let rescheduled = input.first_run != existing.first_run
        || (rk.as_deref(), rv, cron.as_str())
            != (
                existing.repeat_kind.as_deref(),
                existing.repeat_value,
                existing.cron_expr.as_str(),
            );
    // The schedule's zone is fixed when it is (re)scheduled, so changing the time-zone setting
    // later does not silently move existing jobs.
    let (timezone, next_run) = if rescheduled {
        let tz = effective_timezone(db, user_id).await?;
        (
            tz.name().to_owned(),
            initial_next_run(input.repeat, input.first_run, now, tz),
        )
    } else {
        (existing.timezone.clone(), existing.next_run)
    };

    let mut am: job::ActiveModel = existing.into();
    am.category_id = Set(cat.id);
    am.title = Set(input.title);
    am.summary = Set(input.summary);
    am.description = Set(input.description);
    am.first_run = Set(input.first_run);
    am.repeat_kind = Set(rk);
    am.repeat_value = Set(rv);
    am.cron_expr = Set(cron);
    am.next_run = Set(next_run);
    am.run_mode = Set(input.run_mode.as_str().into());
    am.command = Set(input.command);
    am.script_path = Set(input.script_path);
    am.script_args = Set(input.script_args);
    am.working_dir = Set(input.working_dir);
    am.timeout_secs = Set(input.timeout_secs as i32);
    am.notify_on_failure = Set(input.notify_on_failure);
    am.retry_on_failure = Set(input.retry_on_failure);
    am.retry_count = Set(input.retry_count as i32);
    am.retry_delay_secs = Set(input.retry_delay_secs as i32);
    if !input.retry_on_failure || rescheduled {
        // Turning retries off (or moving the schedule) drops any pending retry.
        am.retry_at = Set(None);
        am.retry_attempt = Set(0);
    }
    am.timezone = Set(timezone);
    let m = am.update(db).await?;
    let lr = last_run(db, m.id).await?;
    Ok(view(m, cat.name, lr))
}

async fn find_owned(db: &impl ConnectionTrait, user_id: i32, id: i32) -> DbResult<job::Model> {
    job::Entity::find_by_id(id)
        .filter(job::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or(DbError::NotFound)
}

pub async fn get(db: &impl ConnectionTrait, user_id: i32, id: i32) -> DbResult<JobView> {
    let m = find_owned(db, user_id, id).await?;
    let category = category::Entity::find_by_id(m.category_id)
        .one(db)
        .await?
        .map(|c| c.name)
        .unwrap_or_default();
    let lr = last_run(db, m.id).await?;
    Ok(view(m, category, lr))
}

pub async fn delete(db: &impl ConnectionTrait, user_id: i32, id: i32) -> DbResult<()> {
    find_owned(db, user_id, id).await?;
    job::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

/// All of a user's jobs, soonest next run first (jobs with no next run last).
pub async fn list(db: &impl ConnectionTrait, user_id: i32) -> DbResult<Vec<JobView>> {
    let cats: std::collections::HashMap<i32, String> = category::Entity::find()
        .filter(category::Column::UserId.eq(user_id))
        .all(db)
        .await?
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();
    let mut jobs = job::Entity::find()
        .filter(job::Column::UserId.eq(user_id))
        .order_by_asc(job::Column::Id)
        .all(db)
        .await?;
    jobs.sort_by_key(|j| (j.next_run.is_none(), j.next_run));
    let mut out = Vec::with_capacity(jobs.len());
    for j in jobs {
        let c = cats.get(&j.category_id).cloned().unwrap_or_default();
        let lr = last_run(db, j.id).await?;
        out.push(view(j, c, lr));
    }
    Ok(out)
}

/// Jobs with an automatic retry due at or before `now`, across all users (scheduler use).
pub async fn retries_due(
    db: &impl ConnectionTrait,
    now: DateTime<Utc>,
) -> DbResult<Vec<job::Model>> {
    Ok(job::Entity::find()
        .filter(job::Column::RetryAt.lte(now))
        .order_by_asc(job::Column::RetryAt)
        .all(db)
        .await?)
}

/// Jobs whose `next_run` is at or before `now`, across all users (scheduler use).
pub async fn due(db: &impl ConnectionTrait, now: DateTime<Utc>) -> DbResult<Vec<job::Model>> {
    Ok(job::Entity::find()
        .filter(job::Column::NextRun.lte(now))
        .order_by_asc(job::Column::NextRun)
        .all(db)
        .await?)
}
