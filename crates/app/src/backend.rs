// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Server-only plumbing: the shared DB handle, the scheduler/executor and error mapping.

use std::process::Stdio;
use std::time::Duration;

use chrono::Utc;
use dioxus::prelude::ServerFnError;
use kaaryasoochi_core::dto::{RunStatus, UserView};
use kaaryasoochi_core::exec::shell_line;
use kaaryasoochi_core::RunMode;
use kaaryasoochi_db::entity::job;
use kaaryasoochi_db::services::run::Claim;
use kaaryasoochi_db::{connect, services, DatabaseConnection, DbError};
use tokio::process::Command;
use tokio::sync::OnceCell;

static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

const TICK: Duration = Duration::from_secs(5);
/// Only the tail of a job's output is stored, so a chatty job cannot bloat the database.
const OUTPUT_TAIL_CHARS: usize = 8000;

/// Lazily connects, migrates and starts the scheduler on first use.
/// Location comes from `KAARYASOOCHI_DATABASE_URL` (default: `./kaaryasoochi.db`).
pub async fn db() -> Result<&'static DatabaseConnection, ServerFnError> {
    DB.get_or_try_init(|| async {
        let url = std::env::var("KAARYASOOCHI_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://kaaryasoochi.db?mode=rwc".into());
        let db = connect(&url)
            .await
            .map_err(|e| ServerFnError::new(format!("database unavailable: {e}")))?;
        match services::run::recover_interrupted(&db).await {
            Ok(0) => {}
            Ok(n) => eprintln!("marked {n} interrupted run(s) as failed"),
            Err(e) => eprintln!("recovering interrupted runs failed: {e}"),
        }
        tokio::spawn(scheduler(db.clone()));
        Ok(db)
    })
    .await
}

/// Claims due jobs and runs each in its own task, so one slow job never delays the others.
async fn scheduler(db: DatabaseConnection) {
    loop {
        match services::run::claim_due(&db, Utc::now()).await {
            Ok(claims) => {
                for claim in claims {
                    spawn_run(db.clone(), claim);
                }
            }
            Err(e) => eprintln!("scheduler tick failed: {e}"),
        }
        tokio::time::sleep(TICK).await;
    }
}

/// Executes a claimed run in its own task, records the outcome and notifies on failure.
pub fn spawn_run(db: DatabaseConnection, claim: Claim) {
    tokio::spawn(async move {
        eprintln!("running job {} ({})", claim.job.id, claim.job.title);
        let (status, code, message) = execute(&claim.job).await;
        match services::run::finish(&db, claim.run_id, status, code, message).await {
            // Only the final failure is worth a notification, not each attempt that will be retried.
            Ok(f)
                if status == RunStatus::Failed && claim.job.notify_on_failure && !f.will_retry =>
            {
                notify_failure(&claim.job, code).await;
            }
            Ok(_) => {}
            Err(e) => eprintln!("recording result of run {} failed: {e}", claim.run_id),
        }
    });
}

/// Best-effort desktop notification (needs `notify-send` and the user's session bus).
async fn notify_failure(job: &job::Model, code: Option<i32>) {
    let detail = match code {
        Some(c) => format!("exit code {c}"),
        None => "did not complete (see run history)".to_string(),
    };
    let _ = Command::new("notify-send")
        .args(["-u", "critical", "-a", "Kaaryasoochi", "Job failed"])
        .arg(format!("{} - {detail}", job.title))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

/// Runs the job like cron would: `/bin/sh -c`, a minimal environment, the user's
/// home as working directory. Success means exit status 0.
async fn execute(job: &job::Model) -> (RunStatus, Option<i32>, String) {
    let command = shell_line(
        RunMode::parse(&job.run_mode),
        &job.command,
        &job.script_path,
        &job.script_args,
    );
    if command.is_empty() {
        return (RunStatus::Success, None, "no command configured".into());
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let cwd = if job.working_dir.trim().is_empty() {
        home.clone()
    } else {
        job.working_dir.trim().to_owned()
    };
    if !std::path::Path::new(&cwd).is_dir() {
        return (
            RunStatus::Failed,
            None,
            format!("working directory does not exist: {cwd}"),
        );
    }
    let user = std::env::var("LOGNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    let timeout = Duration::from_secs(job.timeout_secs.max(1) as u64);

    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c")
        .arg(&command)
        .env_clear()
        .env("HOME", &home)
        .env("LOGNAME", &user)
        .env("USER", &user)
        .env("SHELL", "/bin/sh")
        .env("PATH", "/usr/bin:/bin")
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Own process group, so a timeout can stop the whole script tree, not just `sh`.
        .process_group(0)
        .kill_on_drop(true);
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return (RunStatus::Failed, None, format!("failed to start: {e}")),
    };
    let pid = child.id();

    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => {
            let code = out.status.code();
            let mut msg = format_output(&out.stdout, &out.stderr);
            if code.is_none() {
                msg = format!("terminated by a signal\n{msg}");
            }
            let status = if out.status.success() {
                RunStatus::Success
            } else {
                RunStatus::Failed
            };
            (status, code, msg)
        }
        Ok(Err(e)) => (
            RunStatus::Failed,
            None,
            format!("failed while waiting: {e}"),
        ),
        Err(_) => {
            if let Some(pid) = pid {
                let _ = Command::new("kill")
                    .args(["-KILL", "--", &format!("-{pid}")])
                    .status()
                    .await;
            }
            (
                RunStatus::Failed,
                None,
                format!("timed out after {}s and was killed", timeout.as_secs()),
            )
        }
    }
}

fn tail(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let s = s.trim_end();
    let n = s.chars().count();
    if n > OUTPUT_TAIL_CHARS {
        let skip = n - OUTPUT_TAIL_CHARS;
        format!("…{}", s.chars().skip(skip).collect::<String>())
    } else {
        s.to_owned()
    }
}

fn format_output(stdout: &[u8], stderr: &[u8]) -> String {
    let (o, e) = (tail(stdout), tail(stderr));
    match (o.is_empty(), e.is_empty()) {
        (true, true) => "(no output)".into(),
        (false, true) => o,
        (true, false) => format!("[stderr]\n{e}"),
        (false, false) => format!("[stdout]\n{o}\n[stderr]\n{e}"),
    }
}

/// Like [`auth`], but only for administrators (who may run commands and browse the disk).
pub async fn require_admin(token: &str) -> Result<UserView, ServerFnError> {
    let u = auth(token).await?;
    if u.is_admin {
        Ok(u)
    } else {
        Err(ServerFnError::new("only administrators can do this"))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn job(command: &str, timeout_secs: i32) -> job::Model {
        job::Model {
            id: 1,
            user_id: 1,
            category_id: 1,
            title: "t".into(),
            summary: String::new(),
            description: String::new(),
            first_run: Utc::now(),
            repeat_kind: None,
            repeat_value: None,
            next_run: None,
            cron_expr: String::new(),
            run_mode: "command".into(),
            script_path: String::new(),
            script_args: String::new(),
            working_dir: String::new(),
            notify_on_failure: false,
            retry_on_failure: false,
            retry_count: 3,
            retry_delay_secs: 60,
            retry_at: None,
            retry_attempt: 0,
            command: command.into(),
            timeout_secs,
            timezone: "UTC".into(),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn success_captures_stdout() {
        let (s, c, m) = execute(&job("echo hello", 10)).await;
        assert_eq!((s, c, m.as_str()), (RunStatus::Success, Some(0), "hello"));
    }

    #[tokio::test]
    async fn failure_reports_exit_code_and_stderr() {
        let (s, c, m) = execute(&job("echo oops >&2; exit 3", 10)).await;
        assert_eq!((s, c), (RunStatus::Failed, Some(3)));
        assert_eq!(m, "[stderr]\noops");
    }

    #[tokio::test]
    async fn empty_command_is_a_noop_success() {
        let (s, c, _) = execute(&job("  ", 10)).await;
        assert_eq!((s, c), (RunStatus::Success, None));
    }

    #[tokio::test]
    async fn environment_is_cron_like() {
        std::env::set_var("KAARYASOOCHI_SECRET_TEST", "leak");
        let (_, _, m) = execute(&job(
            "echo \"[$KAARYASOOCHI_SECRET_TEST] $PATH $SHELL\"",
            10,
        ))
        .await;
        assert_eq!(m, "[] /usr/bin:/bin /bin/sh");
    }

    #[tokio::test]
    async fn timeout_kills_the_whole_process_group() {
        // The grandchild `sleep` must die too, not just the shell.
        let started = std::time::Instant::now();
        let (s, _, m) = execute(&job("sleep 30 & sleep 30; wait", 1)).await;
        assert_eq!(s, RunStatus::Failed);
        assert!(m.starts_with("timed out after 1s"), "{m}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn script_mode_quotes_the_path_and_passes_args() {
        let dir = std::env::temp_dir().join(format!("kaarya exec {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("my script.sh");
        std::fs::write(&script, "#!/bin/sh\necho \"cwd=$(pwd) arg=$1\"\n").unwrap();
        std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();

        let mut j = job("", 10);
        j.run_mode = "script".into();
        j.script_path = script.to_string_lossy().into_owned();
        j.script_args = "\"hello world\"".into();
        j.working_dir = dir.to_string_lossy().into_owned();
        let (s, c, m) = execute(&j).await;
        assert_eq!((s, c), (RunStatus::Success, Some(0)), "{m}");
        assert_eq!(
            m,
            format!(
                "cwd={} arg=hello world",
                dir.canonicalize().unwrap().display()
            )
        );

        j.working_dir = "/no/such/dir".into();
        let (s, _, m) = execute(&j).await;
        assert_eq!(s, RunStatus::Failed);
        assert!(m.starts_with("working directory does not exist"), "{m}");
    }

    #[tokio::test]
    async fn non_executable_script_fails_clearly() {
        let f = std::env::temp_dir().join(format!("kaarya-noexec-{}.sh", std::process::id()));
        std::fs::write(&f, "echo hi\n").unwrap();
        let mut j = job("", 10);
        j.run_mode = "script".into();
        j.script_path = f.to_string_lossy().into_owned();
        let (s, c, m) = execute(&j).await;
        assert_eq!((s, c), (RunStatus::Failed, Some(126)), "{m}");
        assert!(m.to_lowercase().contains("permission denied"), "{m}");
    }

    #[test]
    fn long_output_is_truncated_to_the_tail() {
        let big = "x".repeat(OUTPUT_TAIL_CHARS + 500);
        let t = tail(big.as_bytes());
        assert_eq!(t.chars().count(), OUTPUT_TAIL_CHARS + 1);
        assert!(t.starts_with('…'));
    }
}
