// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Run bookkeeping. Scheduling is split into two steps so a slow job can never block the
//! scheduler or run twice: [`claim_due`] records a `running` row and advances `next_run`
//! immediately, and the caller executes the job and reports back with [`finish`].

use chrono::{DateTime, Duration, Utc};
use kaaryasoochi_core::dto::{Page, RunStatus, RunSummary, RunView};
use kaaryasoochi_core::timezone::{self, Tz};
use sea_orm::{prelude::*, ActiveValue::Set, PaginatorTrait, QueryOrder};

use crate::entity::{job, job_run};
use crate::{DbError, DbResult};

/// A run that has been recorded as `running` and is ready to execute.
pub struct Claim {
    pub run_id: i32,
    pub job: job::Model,
}

fn view(m: job_run::Model) -> RunView {
    RunView {
        id: m.id,
        scheduled_for: m.scheduled_for,
        started_at: m.started_at,
        finished_at: m.finished_at,
        status: RunStatus::parse(&m.status),
        exit_code: m.exit_code,
        attempt: m.attempt.max(1) as u32,
        message: m.message,
    }
}

/// Newest first. `page` is 1-based.
pub async fn history(
    db: &impl ConnectionTrait,
    user_id: i32,
    job_id: i32,
    page: u32,
    page_size: u32,
) -> DbResult<Page<RunView>> {
    super::job::get(db, user_id, job_id).await?; // ownership check
    let (page, page_size) = (page.max(1), page_size.max(1));
    let paginator = job_run::Entity::find()
        .filter(job_run::Column::JobId.eq(job_id))
        .order_by_desc(job_run::Column::StartedAt)
        .order_by_desc(job_run::Column::Id)
        .paginate(db, page_size as u64);
    let total = paginator.num_items().await?;
    let items = paginator
        .fetch_page(page as u64 - 1)
        .await?
        .into_iter()
        .map(view)
        .collect();
    Ok(Page {
        items,
        total,
        page,
        page_size,
    })
}

/// Records every due job as started and moves its `next_run` forward.
///
/// Missed occurrences are skipped, not replayed: the next run is the first slot after `now`.
/// A job whose previous run is still going is not started again; that slot is recorded as a
/// failed "skipped" run instead.
pub async fn claim_due(db: &impl ConnectionTrait, now: DateTime<Utc>) -> DbResult<Vec<Claim>> {
    let mut claims = Vec::new();
    let mut handled = Vec::new();
    for j in super::job::due(db, now).await? {
        let scheduled_for = j.next_run.unwrap_or(now);
        let still_running = job_run::Entity::find()
            .filter(job_run::Column::JobId.eq(j.id))
            .filter(job_run::Column::Status.eq(RunStatus::Running.as_str()))
            .count(db)
            .await?
            > 0;
        let (status, message, finished) = if still_running {
            (
                RunStatus::Failed,
                "skipped: previous run still running",
                Some(now),
            )
        } else {
            (RunStatus::Running, "", None)
        };
        let run = job_run::ActiveModel {
            job_id: Set(j.id),
            scheduled_for: Set(scheduled_for),
            started_at: Set(now),
            finished_at: Set(finished),
            status: Set(status.as_str().into()),
            attempt: Set(1),
            message: Set(message.into()),
            ..Default::default()
        }
        .insert(db)
        .await?;

        let tz = timezone::parse(&j.timezone).unwrap_or(Tz::UTC);
        let next = super::job::repeat_of(&j).and_then(|r| r.next_after(j.first_run, now, tz));
        let mut am: job::ActiveModel = j.clone().into();
        am.next_run = Set(next);
        // A new scheduled occurrence supersedes any retry still waiting for the previous one.
        am.retry_at = Set(None);
        am.retry_attempt = Set(0);
        am.update(db).await?;
        handled.push(j.id);

        if status == RunStatus::Running {
            claims.push(Claim {
                run_id: run.id,
                job: j,
            });
        }
    }

    // Automatic retries that have waited long enough (unless the job was just scheduled above).
    for j in super::job::retries_due(db, now).await? {
        if handled.contains(&j.id) {
            continue;
        }
        let running = job_run::Entity::find()
            .filter(job_run::Column::JobId.eq(j.id))
            .filter(job_run::Column::Status.eq(RunStatus::Running.as_str()))
            .count(db)
            .await?;
        if running > 0 {
            continue; // try again on a later tick
        }
        let run = job_run::ActiveModel {
            job_id: Set(j.id),
            scheduled_for: Set(j.retry_at.unwrap_or(now)),
            started_at: Set(now),
            status: Set(RunStatus::Running.as_str().into()),
            attempt: Set(j.retry_attempt + 1),
            message: Set(String::new()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        let mut am: job::ActiveModel = j.clone().into();
        am.retry_at = Set(None);
        am.update(db).await?;
        claims.push(Claim {
            run_id: run.id,
            job: j,
        });
    }
    Ok(claims)
}

/// What [`finish`] decided about the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finish {
    /// A failed run that will be retried automatically.
    pub will_retry: bool,
}

/// Records the outcome of a claimed run and, for a failure with retries left, schedules the
/// next attempt `retry_delay_secs` from now. Success (or running out of retries) clears it.
pub async fn finish(
    db: &impl ConnectionTrait,
    run_id: i32,
    status: RunStatus,
    exit_code: Option<i32>,
    message: String,
) -> DbResult<Finish> {
    let Some(run) = job_run::Entity::find_by_id(run_id).one(db).await? else {
        return Ok(Finish { will_retry: false });
    };
    let (job_id, attempt) = (run.job_id, run.attempt.max(1));
    let now = Utc::now();
    let mut am: job_run::ActiveModel = run.into();
    am.status = Set(status.as_str().into());
    am.exit_code = Set(exit_code);
    am.message = Set(message);
    am.finished_at = Set(Some(now));
    am.update(db).await?;

    let Some(job) = job::Entity::find_by_id(job_id).one(db).await? else {
        return Ok(Finish { will_retry: false });
    };
    // Attempt 1 is the scheduled run, so `retry_count` retries allow attempts up to 1 + count.
    let will_retry =
        status == RunStatus::Failed && job.retry_on_failure && attempt <= job.retry_count;
    let mut jm: job::ActiveModel = job.clone().into();
    if will_retry {
        jm.retry_at = Set(Some(
            now + Duration::seconds(job.retry_delay_secs.max(1) as i64),
        ));
        jm.retry_attempt = Set(attempt);
    } else {
        jm.retry_at = Set(None);
        jm.retry_attempt = Set(0);
    }
    jm.update(db).await?;
    Ok(Finish { will_retry })
}

/// Marks runs left `running` by a previous server process as failed. Call once at startup.
pub async fn recover_interrupted(db: &impl ConnectionTrait) -> DbResult<u64> {
    let res = job_run::Entity::update_many()
        .col_expr(
            job_run::Column::Status,
            Expr::value(RunStatus::Failed.as_str()),
        )
        .col_expr(
            job_run::Column::Message,
            Expr::value("interrupted: server stopped while the job was running"),
        )
        .col_expr(job_run::Column::FinishedAt, Expr::value(Utc::now()))
        .filter(job_run::Column::Status.eq(RunStatus::Running.as_str()))
        .exec(db)
        .await?;
    Ok(res.rows_affected)
}

/// Total runs and the latest one, for the folded history header.
pub async fn summary(db: &impl ConnectionTrait, user_id: i32, job_id: i32) -> DbResult<RunSummary> {
    super::job::get(db, user_id, job_id).await?; // ownership check
    let total = job_run::Entity::find()
        .filter(job_run::Column::JobId.eq(job_id))
        .count(db)
        .await?;
    let last = job_run::Entity::find()
        .filter(job_run::Column::JobId.eq(job_id))
        .order_by_desc(job_run::Column::StartedAt)
        .order_by_desc(job_run::Column::Id)
        .one(db)
        .await?
        .map(view);
    let retry_at = job::Entity::find_by_id(job_id)
        .one(db)
        .await?
        .and_then(|j| j.retry_at);
    Ok(RunSummary {
        total,
        last,
        retry_at,
    })
}

/// "Run now": records a run immediately without touching the schedule (`next_run` is left
/// alone). Refused while the job is already running.
pub async fn start_manual(db: &impl ConnectionTrait, user_id: i32, job_id: i32) -> DbResult<Claim> {
    super::job::get(db, user_id, job_id).await?; // ownership check
    let running = job_run::Entity::find()
        .filter(job_run::Column::JobId.eq(job_id))
        .filter(job_run::Column::Status.eq(RunStatus::Running.as_str()))
        .count(db)
        .await?;
    if running > 0 {
        return Err(DbError::AlreadyRunning);
    }
    let job = job::Entity::find_by_id(job_id)
        .one(db)
        .await?
        .ok_or(DbError::NotFound)?;
    let now = Utc::now();
    let run = job_run::ActiveModel {
        job_id: Set(job_id),
        scheduled_for: Set(now),
        started_at: Set(now),
        status: Set(RunStatus::Running.as_str().into()),
        attempt: Set(1),
        message: Set(String::new()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    // A fresh manual run supersedes a retry that was waiting.
    let mut am: job::ActiveModel = job.clone().into();
    am.retry_at = Set(None);
    am.retry_attempt = Set(0);
    am.update(db).await?;
    Ok(Claim {
        run_id: run.id,
        job,
    })
}
