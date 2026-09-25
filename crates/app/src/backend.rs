//! Server-only plumbing: the shared DB handle, the background scheduler and error mapping.

use std::time::Duration;

use chrono::Utc;
use dioxus::prelude::ServerFnError;
use kaaryasoochi_core::dto::UserView;
use kaaryasoochi_db::{connect, services, DatabaseConnection, DbError};
use tokio::sync::OnceCell;

static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

const TICK: Duration = Duration::from_secs(15);

/// Lazily connects, migrates and starts the scheduler on first use.
/// Location comes from `KAARYASOOCHI_DATABASE_URL` (default: `./kaaryasoochi.db`).
pub async fn db() -> Result<&'static DatabaseConnection, ServerFnError> {
    DB.get_or_try_init(|| async {
        let url = std::env::var("KAARYASOOCHI_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://kaaryasoochi.db?mode=rwc".into());
        let db = connect(&url)
            .await
            .map_err(|e| ServerFnError::new(format!("database unavailable: {e}")))?;
        tokio::spawn(scheduler(db.clone()));
        Ok(db)
    })
    .await
}

/// No command/executor is defined for jobs yet, so a due job is recorded as a
/// successful no-op run. Swap the closure for real execution when that lands.
async fn scheduler(db: DatabaseConnection) {
    loop {
        if let Err(e) =
            services::run::tick(&db, Utc::now(), |_| Ok("scheduled run recorded".into())).await
        {
            eprintln!("scheduler tick failed: {e}");
        }
        tokio::time::sleep(TICK).await;
    }
}

pub async fn auth(token: &str) -> Result<UserView, ServerFnError> {
    services::auth::user_for_token(db().await?, token)
        .await
        .map_err(err)
}

/// User-facing errors pass through; internal ones are logged and masked.
pub fn err(e: DbError) -> ServerFnError {
    match e {
        DbError::Db(_) | DbError::Internal(_) => {
            eprintln!("internal error: {e}");
            ServerFnError::new("internal server error")
        }
        e => ServerFnError::new(e.to_string()),
    }
}
