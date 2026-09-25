use chrono::{DateTime, Utc};
use kaaryasoochi_core::dto::{Page, RunStatus, RunView};
use kaaryasoochi_core::Repeat;
use sea_orm::{prelude::*, ActiveValue::Set, PaginatorTrait, QueryOrder};

use crate::entity::{job, job_run};
use crate::DbResult;

fn view(m: job_run::Model) -> RunView {
    RunView {
        id: m.id,
        scheduled_for: m.scheduled_for,
        started_at: m.started_at,
        finished_at: m.finished_at,
        status: RunStatus::parse(&m.status),
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

/// Runs every due job through `exec`, records the outcome, and advances `next_run`.
/// `exec` returns `Ok(message)` or `Err(message)`. Returns how many jobs ran.
pub async fn tick<F>(db: &impl ConnectionTrait, now: DateTime<Utc>, exec: F) -> DbResult<usize>
where
    F: Fn(&job::Model) -> Result<String, String>,
{
    let due = super::job::due(db, now).await?;
    for j in &due {
        let scheduled_for = j.next_run.unwrap_or(now);
        let (status, message) = match exec(j) {
            Ok(m) => (RunStatus::Success, m),
            Err(m) => (RunStatus::Failed, m),
        };
        job_run::ActiveModel {
            job_id: Set(j.id),
            scheduled_for: Set(scheduled_for),
            started_at: Set(now),
            finished_at: Set(Some(Utc::now())),
            status: Set(status.as_str().into()),
            message: Set(message),
            ..Default::default()
        }
        .insert(db)
        .await?;

        // Missed occurrences are skipped, not replayed: jump to the first slot after `now`.
        let next = j
            .repeat_kind
            .as_deref()
            .zip(j.repeat_value)
            .and_then(|(k, v)| Repeat::decode(k, v as u32).ok())
            .and_then(|r| r.next_after(j.first_run, now));
        let mut am: job::ActiveModel = j.clone().into();
        am.next_run = Set(next);
        am.update(db).await?;
    }
    Ok(due.len())
}
