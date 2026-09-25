// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

use chrono::{DateTime, Duration, Utc};
use kaaryasoochi_core::dto::JobView;
use kaaryasoochi_core::{JobInput, Repeat};
use sea_orm::{prelude::*, ActiveValue::Set, QueryOrder};

use crate::entity::{category, job};
use crate::services::category::get_or_create;
use crate::{DbError, DbResult};

fn view(m: job::Model, category: String) -> JobView {
    let repeat = m
        .repeat_kind
        .as_deref()
        .zip(m.repeat_value)
        .and_then(|(k, v)| Repeat::decode(k, v as u32).ok());
    JobView {
        id: m.id,
        title: m.title,
        category,
        summary: m.summary,
        description: m.description,
        first_run: m.first_run,
        repeat,
        next_run: m.next_run,
    }
}

fn repeat_cols(r: Option<Repeat>) -> (Option<String>, Option<i32>) {
    match r.map(|r| r.encode()) {
        Some((k, v)) => (Some(k.into()), Some(v as i32)),
        None => (None, None),
    }
}

pub async fn create(db: &impl ConnectionTrait, user_id: i32, input: JobInput) -> DbResult<JobView> {
    let now = Utc::now();
    let input = input.validate(now)?;
    let cat = get_or_create(db, user_id, &input.category).await?;
    let (rk, rv) = repeat_cols(input.repeat);
    let m = job::ActiveModel {
        user_id: Set(user_id),
        category_id: Set(cat.id),
        title: Set(input.title),
        summary: Set(input.summary),
        description: Set(input.description),
        first_run: Set(input.first_run),
        repeat_kind: Set(rk),
        repeat_value: Set(rv),
        next_run: Set(Some(input.first_run)),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(view(m, cat.name))
}

pub async fn update(
    db: &impl ConnectionTrait,
    user_id: i32,
    id: i32,
    input: JobInput,
) -> DbResult<JobView> {
    let existing = find_owned(db, user_id, id).await?;
    // An unchanged first-run may legitimately be in the past (the job already ran),
    // so only enforce the future rule when the user actually moved it.
    let check_now = if input.first_run == existing.first_run {
        input.first_run - Duration::seconds(1)
    } else {
        Utc::now()
    };
    let input = input.validate(check_now)?;
    let cat = get_or_create(db, user_id, &input.category).await?;
    let (rk, rv) = repeat_cols(input.repeat);
    let rescheduled = input.first_run != existing.first_run
        || (rk.as_deref(), rv) != (existing.repeat_kind.as_deref(), existing.repeat_value);
    let next_run = if rescheduled {
        Some(input.first_run)
    } else {
        existing.next_run
    };

    let mut am: job::ActiveModel = existing.into();
    am.category_id = Set(cat.id);
    am.title = Set(input.title);
    am.summary = Set(input.summary);
    am.description = Set(input.description);
    am.first_run = Set(input.first_run);
    am.repeat_kind = Set(rk);
    am.repeat_value = Set(rv);
    am.next_run = Set(next_run);
    Ok(view(am.update(db).await?, cat.name))
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
    Ok(view(m, category))
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
    Ok(jobs
        .into_iter()
        .map(|j| {
            let c = cats.get(&j.category_id).cloned().unwrap_or_default();
            view(j, c)
        })
        .collect())
}

/// Jobs whose `next_run` is at or before `now`, across all users (scheduler use).
pub async fn due(db: &impl ConnectionTrait, now: DateTime<Utc>) -> DbResult<Vec<job::Model>> {
    Ok(job::Entity::find()
        .filter(job::Column::NextRun.lte(now))
        .order_by_asc(job::Column::NextRun)
        .all(db)
        .await?)
}
