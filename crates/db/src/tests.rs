// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use chrono::{Duration, Utc};
use kaaryasoochi_core::dto::{RunStatus, UserSettings};
use kaaryasoochi_core::limits::TIMEOUT_DEFAULT_SECS;
use kaaryasoochi_core::{JobInput, Layout, Repeat, RunMode, WeekdaySet, DEFAULT_CATEGORY};

use crate::services::*;
use crate::{connect, DbError};
use sea_orm::EntityTrait;

fn input(title: &str, category: &str) -> JobInput {
    JobInput {
        title: title.into(),
        category: category.into(),
        summary: String::new(),
        description: String::new(),
        first_run: Utc::now() + Duration::hours(1),
        repeat: None,
        run_mode: RunMode::Command,
        command: String::new(),
        script_path: String::new(),
        script_args: String::new(),
        working_dir: String::new(),
        timeout_secs: TIMEOUT_DEFAULT_SECS,
        notify_on_failure: true,
        retry_on_failure: false,
        retry_count: 3,
        retry_delay_secs: 60,
    }
}

#[tokio::test]
async fn auth_flow() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "Alice", "correct horse", "Alice A")
        .await
        .unwrap();
    assert!(matches!(
        auth::register(&db, "alice", "correct horse", "").await,
        Err(DbError::UsernameTaken)
    ));
    assert!(matches!(
        auth::login(&db, "alice", "wrong password").await,
        Err(DbError::InvalidCredentials)
    ));
    let (tok, _) = auth::login(&db, "ALICE", "correct horse").await.unwrap();
    assert_eq!(auth::user_for_token(&db, &tok).await.unwrap().id, u.id);
    auth::change_password(&db, u.id, "correct horse", "battery staple")
        .await
        .unwrap();
    assert!(
        auth::user_for_token(&db, &tok).await.is_err(),
        "sessions revoked on password change"
    );
    let (tok, _) = auth::login(&db, "alice", "battery staple").await.unwrap();
    auth::logout(&db, &tok).await.unwrap();
    assert!(auth::user_for_token(&db, &tok).await.is_err());
}

#[tokio::test]
async fn defaults_on_register() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "bob", "correct horse", "")
        .await
        .unwrap();
    let cats = category::list(&db, u.id).await.unwrap();
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0].name, DEFAULT_CATEGORY);
    assert_eq!(
        settings::get(&db, u.id).await.unwrap(),
        UserSettings::default()
    );
    let s = UserSettings {
        job_layout: Layout::List,
        ..Default::default()
    };
    settings::update(&db, u.id, s).await.unwrap();
    assert_eq!(
        settings::get(&db, u.id).await.unwrap().job_layout,
        Layout::List
    );
}

#[tokio::test]
async fn jobs_categories_and_isolation() {
    let db = connect("sqlite::memory:").await.unwrap();
    let a = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let b = auth::register(&db, "bobby", "correct horse", "")
        .await
        .unwrap();

    let j = job::create(&db, a.id, input("Backup", "")).await.unwrap();
    assert_eq!(j.category, DEFAULT_CATEGORY);
    let j2 = job::create(&db, a.id, input("Report", "Finance"))
        .await
        .unwrap();
    job::create(&db, a.id, input("Report 2", "finance"))
        .await
        .unwrap(); // case-insensitive reuse
    assert_eq!(category::list(&db, a.id).await.unwrap().len(), 2);

    assert!(matches!(
        job::get(&db, b.id, j.id).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        job::delete(&db, b.id, j.id).await,
        Err(DbError::NotFound)
    ));

    let fin = category::list(&db, a.id)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Finance")
        .unwrap();
    category::delete(&db, a.id, fin.id).await.unwrap();
    assert_eq!(
        job::get(&db, a.id, j2.id).await.unwrap().category,
        DEFAULT_CATEGORY
    );
    let default = category::list(&db, a.id).await.unwrap()[0].id;
    assert!(matches!(
        category::delete(&db, a.id, default).await,
        Err(DbError::ProtectedCategory)
    ));

    let mut past = input("Old", "");
    past.first_run = Utc::now() - Duration::hours(1);
    assert!(matches!(
        job::create(&db, a.id, past).await,
        Err(DbError::Validation(_))
    ));
}

#[tokio::test]
async fn claim_records_running_and_advances() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let mut i = input("Tick", "");
    i.repeat = Some(Repeat::EveryMinutes(10));
    let j = job::create(&db, u.id, i).await.unwrap();
    let mut once = input("Once", "");
    once.first_run = j.first_run;
    let once = job::create(&db, u.id, once).await.unwrap();

    let later = j.first_run + Duration::minutes(25);
    let claims = run::claim_due(&db, later).await.unwrap();
    assert_eq!(claims.len(), 2);
    assert!(
        run::claim_due(&db, later).await.unwrap().is_empty(),
        "not due twice"
    );

    // next_run advances immediately, before the job finishes.
    assert_eq!(
        job::get(&db, u.id, j.id).await.unwrap().next_run,
        Some(j.first_run + Duration::minutes(30))
    );
    assert_eq!(job::get(&db, u.id, once.id).await.unwrap().next_run, None);

    let page = run::history(&db, u.id, j.id, 1, 10).await.unwrap();
    assert_eq!(page.items[0].status, RunStatus::Running);
    let claim = claims.into_iter().find(|c| c.job.id == j.id).unwrap();
    run::finish(&db, claim.run_id, RunStatus::Success, Some(0), "ok".into())
        .await
        .unwrap();
    let r = &run::history(&db, u.id, j.id, 1, 10).await.unwrap().items[0];
    assert_eq!((r.status, r.exit_code), (RunStatus::Success, Some(0)));
    assert!(r.finished_at.is_some());
}

#[tokio::test]
async fn overlapping_run_is_skipped_and_restart_recovers() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let mut i = input("Slow", "");
    i.repeat = Some(Repeat::EveryMinutes(1));
    let j = job::create(&db, u.id, i).await.unwrap();

    assert_eq!(run::claim_due(&db, j.first_run).await.unwrap().len(), 1);
    // Still "running" when the next slot arrives.
    let next = j.first_run + Duration::minutes(1);
    assert!(run::claim_due(&db, next).await.unwrap().is_empty());
    let items = run::history(&db, u.id, j.id, 1, 10).await.unwrap().items;
    assert_eq!(items[0].status, RunStatus::Failed);
    assert!(items[0].message.starts_with("skipped"));

    // A crash leaves the first run "running"; startup recovery fails it.
    assert_eq!(run::recover_interrupted(&db).await.unwrap(), 1);
    let items = run::history(&db, u.id, j.id, 1, 10).await.unwrap().items;
    assert!(items.iter().all(|r| r.status == RunStatus::Failed));
    assert!(items.iter().any(|r| r.message.starts_with("interrupted")));
}

#[tokio::test]
async fn weekday_schedule_runs_in_job_timezone() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let mut s = settings::get(&db, u.id).await.unwrap();
    s.timezone = "Asia/Kolkata".into();
    settings::update(&db, u.id, s).await.unwrap();

    // Next Monday 07:01 IST (01:31 UTC), far enough ahead to be in the future.
    let mut d = (Utc::now() + Duration::days(7)).date_naive();
    while d.format("%a").to_string() != "Mon" {
        d = d.succ_opt().unwrap();
    }
    let first = d.and_hms_opt(1, 31, 0).unwrap().and_utc();
    let mut i = input("Mon-Fri", "");
    i.first_run = first;
    i.repeat = Some(Repeat::Weekdays(WeekdaySet::MON_FRI));
    let j = job::create(&db, u.id, i).await.unwrap();
    assert_eq!(j.timezone, "Asia/Kolkata");

    // Friday's run is followed by Monday's, not Saturday's.
    let friday = first + Duration::days(4);
    let mut monday = run::claim_due(&db, first + Duration::seconds(1))
        .await
        .unwrap();
    run::finish(
        &db,
        monday.remove(0).run_id,
        RunStatus::Success,
        Some(0),
        "".into(),
    )
    .await
    .unwrap();
    assert_eq!(
        job::get(&db, u.id, j.id).await.unwrap().next_run,
        Some(first + Duration::days(1))
    );
    let mut c = run::claim_due(&db, friday).await.unwrap();
    assert_eq!(c.len(), 1);
    run::finish(
        &db,
        c.remove(0).run_id,
        RunStatus::Success,
        Some(0),
        "".into(),
    )
    .await
    .unwrap();
    assert_eq!(
        job::get(&db, u.id, j.id).await.unwrap().next_run,
        Some(first + Duration::days(7))
    );

    // Changing the setting later does not move the existing job's zone...
    let mut s = settings::get(&db, u.id).await.unwrap();
    s.timezone = "America/New_York".into();
    settings::update(&db, u.id, s).await.unwrap();
    let mut same = input("Mon-Fri", "");
    same.first_run = first;
    same.repeat = Some(Repeat::Weekdays(WeekdaySet::MON_FRI));
    same.summary = "edited".into();
    assert_eq!(
        job::update(&db, u.id, j.id, same).await.unwrap().timezone,
        "Asia/Kolkata"
    );
}

#[tokio::test]
async fn timezone_setting_resolution() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    // Default follows the device once it has reported a zone.
    settings::set_system_timezone(&db, u.id, "Asia/Tokyo")
        .await
        .unwrap();
    assert_eq!(
        settings::effective_timezone(&db, u.id)
            .await
            .unwrap()
            .name(),
        "Asia/Tokyo"
    );
    let mut s = settings::get(&db, u.id).await.unwrap();
    assert_eq!(s.timezone, "system");
    s.timezone = "Europe/Paris".into();
    settings::update(&db, u.id, s.clone()).await.unwrap();
    assert_eq!(
        settings::effective_timezone(&db, u.id)
            .await
            .unwrap()
            .name(),
        "Europe/Paris"
    );
    s.timezone = "Mars/Base".into();
    assert!(settings::update(&db, u.id, s).await.is_err());
    assert!(settings::set_system_timezone(&db, u.id, "nope")
        .await
        .is_err());
}

#[tokio::test]
async fn only_admin_can_set_commands() {
    let db = connect("sqlite::memory:").await.unwrap();
    let admin = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let other = auth::register(&db, "bobby", "correct horse", "")
        .await
        .unwrap();
    assert!((admin.is_admin, other.is_admin) == (true, false));

    let mut i = input("Run", "");
    i.command = "echo hi".into();
    assert!(job::create(&db, admin.id, i.clone()).await.is_ok());
    assert!(matches!(
        job::create(&db, other.id, i).await,
        Err(DbError::Validation(_))
    ));
    // Non-admins can still schedule command-less jobs.
    assert!(job::create(&db, other.id, input("Plain", "")).await.is_ok());
}

#[tokio::test]
async fn history_paginates() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let mut i = input("Often", "");
    i.repeat = Some(Repeat::EverySeconds(1));
    let j = job::create(&db, u.id, i).await.unwrap();
    for n in 0..12 {
        for c in run::claim_due(&db, j.first_run + Duration::seconds(n))
            .await
            .unwrap()
        {
            run::finish(&db, c.run_id, RunStatus::Failed, Some(1), "boom".into())
                .await
                .unwrap();
        }
    }
    let p1 = run::history(&db, u.id, j.id, 1, 5).await.unwrap();
    let p3 = run::history(&db, u.id, j.id, 3, 5).await.unwrap();
    assert_eq!(
        (p1.total, p1.items.len(), p3.items.len(), p1.total_pages()),
        (12, 5, 2, 3)
    );
    assert_eq!(p1.items[0].status, RunStatus::Failed);
}

#[tokio::test]
async fn cron_schedule_uses_job_timezone_and_start_from() {
    use chrono::Timelike;
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let mut s = settings::get(&db, u.id).await.unwrap();
    s.timezone = "Asia/Kolkata".into();
    settings::update(&db, u.id, s).await.unwrap();

    // "Start from" may be right now; the first run is the next matching minute after it.
    let mut i = input("Weekday sync", "");
    i.first_run = Utc::now();
    i.repeat = Some(Repeat::Cron("1 7 * * 1-5".parse().unwrap()));
    let j = job::create(&db, u.id, i.clone()).await.unwrap();
    let first = j.next_run.unwrap();
    let local = first.with_timezone(&chrono_tz::Asia::Kolkata);
    assert!(first > Utc::now());
    assert_eq!((local.hour(), local.minute()), (7, 1));
    assert!(!matches!(
        local.format("%a").to_string().as_str(),
        "Sat" | "Sun"
    ));
    assert_eq!(j.repeat, i.repeat);

    // Claiming it schedules the next weekday, again at 07:01 IST.
    let claims = run::claim_due(&db, first + Duration::seconds(1))
        .await
        .unwrap();
    assert_eq!(claims.len(), 1);
    let next = job::get(&db, u.id, j.id).await.unwrap().next_run.unwrap();
    let next_local = next.with_timezone(&chrono_tz::Asia::Kolkata);
    assert!(next > first && (next_local.hour(), next_local.minute()) == (7, 1));

    // Editing only the title keeps the schedule (and its zone) untouched.
    let mut edit = i;
    edit.title = "Renamed".into();
    edit.first_run = j.first_run;
    let updated = job::update(&db, u.id, j.id, edit).await.unwrap();
    assert_eq!(updated.next_run, Some(next));
}

#[tokio::test]
async fn script_mode_and_working_dir_are_admin_only() {
    let db = connect("sqlite::memory:").await.unwrap();
    let admin = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let other = auth::register(&db, "bobby", "correct horse", "")
        .await
        .unwrap();
    let mut i = input("Script", "");
    i.run_mode = RunMode::Script;
    i.script_path = "/opt/run.sh".into();
    i.script_args = "--fast".into();
    i.working_dir = "/tmp".into();
    let j = job::create(&db, admin.id, i.clone()).await.unwrap();
    assert_eq!(
        (
            j.run_mode,
            j.script_path.as_str(),
            j.script_args.as_str(),
            j.working_dir.as_str()
        ),
        (RunMode::Script, "/opt/run.sh", "--fast", "/tmp")
    );
    assert!(matches!(
        job::create(&db, other.id, i.clone()).await,
        Err(DbError::Validation(_))
    ));
    let mut wd_only = input("Only cwd", "");
    wd_only.working_dir = "/tmp".into();
    assert!(job::create(&db, other.id, wd_only).await.is_err());
    // A non-admin can still toggle notifications on a command-less job.
    let mut plain = input("Plain", "");
    let pj = job::create(&db, other.id, plain.clone()).await.unwrap();
    plain.first_run = pj.first_run;
    plain.notify_on_failure = false;
    assert!(
        !job::update(&db, other.id, pj.id, plain)
            .await
            .unwrap()
            .notify_on_failure
    );
}

#[tokio::test]
async fn run_now_leaves_the_schedule_alone_and_refuses_overlap() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let mut i = input("Manual", "");
    i.repeat = Some(Repeat::EveryDays(1));
    let j = job::create(&db, u.id, i).await.unwrap();

    assert!(run::summary(&db, u.id, j.id).await.unwrap().last.is_none());
    let claim = run::start_manual(&db, u.id, j.id).await.unwrap();
    assert_eq!(
        job::get(&db, u.id, j.id).await.unwrap().next_run,
        j.next_run,
        "schedule untouched"
    );
    assert!(matches!(
        run::start_manual(&db, u.id, j.id).await,
        Err(DbError::AlreadyRunning)
    ));
    assert_eq!(
        job::get(&db, u.id, j.id)
            .await
            .unwrap()
            .last_run
            .unwrap()
            .status,
        RunStatus::Running
    );

    run::finish(&db, claim.run_id, RunStatus::Failed, Some(2), "nope".into())
        .await
        .unwrap();
    let sum = run::summary(&db, u.id, j.id).await.unwrap();
    assert_eq!(sum.total, 1);
    let last = sum.last.unwrap();
    assert_eq!((last.status, last.exit_code), (RunStatus::Failed, Some(2)));
    // Someone else's job is not reachable.
    let other = auth::register(&db, "bobby", "correct horse", "")
        .await
        .unwrap();
    assert!(matches!(
        run::start_manual(&db, other.id, j.id).await,
        Err(DbError::NotFound)
    ));
    assert!(run::start_manual(&db, u.id, j.id).await.is_ok());
}

/// Claims everything due at `at`, fails each claimed run, and returns whether a retry was queued.
async fn fail_all(db: &crate::DatabaseConnection, at: chrono::DateTime<Utc>) -> Vec<(u32, bool)> {
    let mut out = Vec::new();
    for c in run::claim_due(db, at).await.unwrap() {
        let f = run::finish(db, c.run_id, RunStatus::Failed, Some(1), "boom".into())
            .await
            .unwrap();
        let attempt = crate::entity::job_run::Entity::find_by_id(c.run_id)
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .attempt as u32;
        out.push((attempt, f.will_retry));
    }
    out
}

fn retrying(count: u32, delay: u32) -> JobInput {
    let mut i = input("Flaky", "");
    i.repeat = Some(Repeat::EveryDays(1));
    i.retry_on_failure = true;
    i.retry_count = count;
    i.retry_delay_secs = delay;
    i
}

#[tokio::test]
async fn failed_run_is_retried_the_configured_number_of_times() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let j = job::create(&db, u.id, retrying(2, 60)).await.unwrap();
    assert!(j.retry_on_failure && (j.retry_count, j.retry_delay_secs) == (2, 60));

    // Attempt 1 (the scheduled run) fails -> a retry is queued 60s out, and not before.
    assert_eq!(fail_all(&db, j.first_run).await, [(1, true)]);
    let pending = job::get(&db, u.id, j.id).await.unwrap().retry_at.unwrap();
    assert!(pending > Utc::now() + Duration::seconds(55));
    assert!(run::claim_due(&db, Utc::now() + Duration::seconds(30))
        .await
        .unwrap()
        .is_empty());

    // Two retries follow, then it gives up (2 retries = attempts 2 and 3).
    let later = Utc::now() + Duration::minutes(2);
    assert_eq!(fail_all(&db, later).await, [(2, true)]);
    assert_eq!(
        fail_all(&db, later + Duration::minutes(2)).await,
        [(3, false)]
    );
    let after = job::get(&db, u.id, j.id).await.unwrap();
    assert_eq!(after.retry_at, None, "no retry left");
    assert!(run::claim_due(&db, later + Duration::minutes(10))
        .await
        .unwrap()
        .is_empty());

    let mut attempts: Vec<_> = run::history(&db, u.id, j.id, 1, 10)
        .await
        .unwrap()
        .items
        .iter()
        .map(|r| (r.attempt, r.status))
        .collect();
    attempts.sort_by_key(|(n, _)| *n);
    assert_eq!(
        attempts,
        [
            (1, RunStatus::Failed),
            (2, RunStatus::Failed),
            (3, RunStatus::Failed)
        ]
    );
}

#[tokio::test]
async fn retry_stops_on_success_and_is_opt_in() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let j = job::create(&db, u.id, retrying(3, 60)).await.unwrap();
    assert_eq!(fail_all(&db, j.first_run).await, [(1, true)]);
    let mut c = run::claim_due(&db, Utc::now() + Duration::minutes(2))
        .await
        .unwrap();
    let f = run::finish(
        &db,
        c.remove(0).run_id,
        RunStatus::Success,
        Some(0),
        "ok".into(),
    )
    .await
    .unwrap();
    assert!(!f.will_retry);
    assert_eq!(job::get(&db, u.id, j.id).await.unwrap().retry_at, None);

    // Without the option a failure is final.
    let mut plain = input("Plain", "");
    plain.repeat = Some(Repeat::EveryDays(1));
    let p = job::create(&db, u.id, plain).await.unwrap();
    assert!(!p.retry_on_failure);
    assert_eq!(fail_all(&db, p.first_run).await, [(1, false)]);
}

#[tokio::test]
async fn new_occurrence_manual_run_and_disabling_drop_a_pending_retry() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();

    // The next scheduled occurrence wins over a retry that is still waiting.
    let j = job::create(&db, u.id, retrying(3, 3600)).await.unwrap();
    assert_eq!(fail_all(&db, j.first_run).await, [(1, true)]);
    let next = job::get(&db, u.id, j.id).await.unwrap().next_run.unwrap();
    assert_eq!(
        fail_all(&db, next).await,
        [(1, true)],
        "fresh attempt 1, not attempt 2"
    );
    assert_eq!(run::history(&db, u.id, j.id, 1, 10).await.unwrap().total, 2);

    // A manual run replaces a pending retry. (Own database: the fake clock above would
    // otherwise make the first job's pending retry fall due here as well.)
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let m = job::create(&db, u.id, retrying(3, 3600)).await.unwrap();
    assert_eq!(fail_all(&db, m.first_run).await, [(1, true)]);
    assert!(job::get(&db, u.id, m.id).await.unwrap().retry_at.is_some());
    run::start_manual(&db, u.id, m.id).await.unwrap();
    assert_eq!(job::get(&db, u.id, m.id).await.unwrap().retry_at, None);

    // Unticking "retry on failure" cancels a pending retry.
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let d = job::create(&db, u.id, retrying(3, 3600)).await.unwrap();
    assert_eq!(fail_all(&db, d.first_run).await, [(1, true)]);
    let mut off = retrying(3, 3600);
    off.first_run = d.first_run;
    off.retry_on_failure = false;
    let after = job::update(&db, u.id, d.id, off).await.unwrap();
    assert_eq!((after.retry_on_failure, after.retry_at), (false, None));
}

#[tokio::test]
async fn summary_reports_the_pending_retry() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    let j = job::create(&db, u.id, retrying(1, 90)).await.unwrap();
    assert_eq!(run::summary(&db, u.id, j.id).await.unwrap().retry_at, None);
    fail_all(&db, j.first_run).await;
    let s = run::summary(&db, u.id, j.id).await.unwrap();
    assert!(s.retry_at.is_some() && s.last.unwrap().attempt == 1);
}

#[tokio::test]
async fn default_category_is_always_last_and_the_rest_alphabetical() {
    let db = connect("sqlite::memory:").await.unwrap();
    let u = auth::register(&db, "alice", "correct horse", "")
        .await
        .unwrap();
    // Created out of order; "Alpha"/"Beta" also sort before "Default" alphabetically, while
    // "Ops"/"Zeta" sort after it, so this only passes if Default is pinned last.
    for name in ["Zeta", "Alpha", "Ops", "Beta"] {
        job::create(&db, u.id, input("j", name)).await.unwrap();
    }
    let names: Vec<_> = category::list(&db, u.id)
        .await
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["Alpha", "Beta", "Ops", "Zeta", DEFAULT_CATEGORY]);
}
