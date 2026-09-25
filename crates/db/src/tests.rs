// SPDX-License-Identifier: BSD-3-Clause
// Copyright (c) 2026, Hariharan Narayanan

use chrono::{Duration, Utc};
use kaaryasoochi_core::dto::{RunStatus, UserSettings};
use kaaryasoochi_core::{JobInput, Layout, Repeat, DEFAULT_CATEGORY};

use crate::services::*;
use crate::{connect, DbError};

fn input(title: &str, category: &str) -> JobInput {
    JobInput {
        title: title.into(),
        category: category.into(),
        summary: String::new(),
        description: String::new(),
        first_run: Utc::now() + Duration::hours(1),
        repeat: None,
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
async fn scheduler_records_runs_and_advances() {
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
    assert_eq!(
        run::tick(&db, later, |_| Ok("done".into())).await.unwrap(),
        2
    );
    assert_eq!(
        run::tick(&db, later, |_| Ok("done".into())).await.unwrap(),
        0,
        "not due twice"
    );

    let repeating = job::get(&db, u.id, j.id).await.unwrap();
    assert_eq!(
        repeating.next_run,
        Some(j.first_run + Duration::minutes(30))
    );
    assert_eq!(job::get(&db, u.id, once.id).await.unwrap().next_run, None);

    let page = run::history(&db, u.id, j.id, 1, 10).await.unwrap();
    assert_eq!((page.total, page.items[0].status), (1, RunStatus::Success));
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
        run::tick(&db, j.first_run + Duration::seconds(n), |_| {
            Err("boom".into())
        })
        .await
        .unwrap();
    }
    let p1 = run::history(&db, u.id, j.id, 1, 5).await.unwrap();
    let p3 = run::history(&db, u.id, j.id, 3, 5).await.unwrap();
    assert_eq!(
        (p1.total, p1.items.len(), p3.items.len(), p1.total_pages()),
        (12, 5, 2, 3)
    );
    assert_eq!(p1.items[0].status, RunStatus::Failed);
}
