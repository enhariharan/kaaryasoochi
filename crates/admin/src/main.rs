// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Operator tool for the Kaaryasoochi database.
//!
//!     kaaryasoochi-admin list
//!     kaaryasoochi-admin reset-password <username>
//!     kaaryasoochi-admin create-job <username> --title TITLE --script PATH
//!                                   [--args ARGS] [--working-dir DIR]
//!                                   [--first-run RFC3339_UTC] [--timeout SECS]
//!
//! The database is `--db <url>`, else `$KAARYASOOCHI_DATABASE_URL`, else the default location
//! (`~/.local/share/kaaryasoochi/kaaryasoochi.db`, never created if missing). The new password is
//! typed at the terminal, twice and without echo; it is not accepted from arguments, the
//! environment or a pipe, so it cannot end up in shell history or process listings.

use std::process::ExitCode;

use chrono::{DateTime, Utc};
use kaaryasoochi_core::{JobInput, RunMode};
use kaaryasoochi_db::services::recovery;

const USAGE: &str = "usage:\n  kaaryasoochi-admin [--db URL] list\n  kaaryasoochi-admin [--db URL] reset-password <username>\n  kaaryasoochi-admin [--db URL] create-job <username> --title TITLE --script PATH [--args ARGS] [--working-dir DIR] [--first-run RFC3339_UTC] [--timeout SECS]";

/// The default database, opened read-write but never created: a typo'd path must not silently
/// produce an empty database that looks like "no such user".
fn default_db_url() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set; pass --db URL".to_string())?;
    Ok(format!(
        "sqlite://{home}/.local/share/kaaryasoochi/kaaryasoochi.db?mode=rw"
    ))
}

struct Args {
    db: Option<String>,
    command: Vec<String>,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut it = args.into_iter();
    let (mut db, mut command) = (None, Vec::new());
    while let Some(a) = it.next() {
        match a.as_str() {
            "--db" => db = Some(it.next().ok_or("--db needs a URL")?),
            "-h" | "--help" => return Err(String::new()),
            _ => command.push(a),
        }
    }
    Ok(Args { db, command })
}

fn prompt_new_password() -> Result<String, String> {
    let first = rpassword::prompt_password("New password: ").map_err(|e| {
        format!("cannot read from the terminal ({e}); run this in an interactive shell")
    })?;
    let again = rpassword::prompt_password("Confirm new password: ").map_err(|e| e.to_string())?;
    if first != again {
        return Err("the passwords do not match; nothing was changed".into());
    }
    Ok(first)
}

async fn run(args: Args) -> Result<(), String> {
    let url = match args
        .db
        .or_else(|| std::env::var("KAARYASOOCHI_DATABASE_URL").ok())
    {
        Some(u) => u,
        None => default_db_url()?,
    };
    let db = kaaryasoochi_db::connect(&url)
        .await
        .map_err(|e| format!("cannot open the database ({url}): {e}"))?;

    match args
        .command
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["list"] => {
            let users = recovery::list_users(&db).await.map_err(|e| e.to_string())?;
            println!("{:<24} {:<6} secret question", "username", "admin");
            for (name, admin, secret) in users {
                println!(
                    "{:<24} {:<6} {}",
                    name,
                    if admin { "yes" } else { "no" },
                    if secret { "set" } else { "NOT SET" }
                );
            }
            Ok(())
        }
        ["reset-password", username] => {
            // Check the account exists before asking for a password.
            let users = recovery::list_users(&db).await.map_err(|e| e.to_string())?;
            let wanted = username.trim().to_lowercase();
            if !users.iter().any(|(n, _, _)| *n == wanted) {
                let known: Vec<_> = users.iter().map(|(n, _, _)| n.as_str()).collect();
                return Err(format!(
                    "no account named {username:?} (accounts: {})",
                    known.join(", ")
                ));
            }
            let password = prompt_new_password()?;
            recovery::admin_reset_password(&db, username, &password)
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "Password for {wanted} changed. Every session for that account was signed out."
            );
            Ok(())
        }
        ["create-job", username, rest @ ..] => {
            let (title, script, args, working_dir, first_run, timeout_secs) =
                parse_create_job_flags(rest)?;
            let user_id = recovery::user_id_by_username(&db, username)
                .await
                .map_err(|_| format!("no account named {username:?}"))?;
            let input = JobInput {
                title,
                category: String::new(),
                summary: String::new(),
                description: String::new(),
                first_run,
                repeat: None,
                run_mode: RunMode::Script,
                command: String::new(),
                script_path: script,
                script_args: args,
                working_dir,
                timeout_secs,
                notify_on_failure: true,
                retry_on_failure: false,
                retry_count: kaaryasoochi_core::limits::RETRY_COUNT_DEFAULT,
                retry_delay_secs: kaaryasoochi_core::limits::RETRY_DELAY_DEFAULT_SECS,
            };
            let job = kaaryasoochi_db::services::job::create(&db, user_id, input)
                .await
                .map_err(|e| e.to_string())?;
            println!("created job #{} {:?}", job.id, job.title);
            Ok(())
        }
        _ => Err(String::new()),
    }
}

/// Parses the flags that follow `create-job <username>`.
/// Returns `(title, script_path, script_args, working_dir, first_run, timeout_secs)`.
fn parse_create_job_flags(
    tokens: &[&str],
) -> Result<(String, String, String, String, DateTime<Utc>, u32), String> {
    let (mut title, mut script, mut args, mut dir, mut first_run_str, mut timeout) = (
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        300u32,
    );
    let mut it = tokens.iter();
    while let Some(flag) = it.next() {
        let mut val = || {
            it.next()
                .copied()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match *flag {
            "--title" => title = val()?.to_owned(),
            "--script" => script = val()?.to_owned(),
            "--args" => args = val()?.to_owned(),
            "--working-dir" => dir = val()?.to_owned(),
            "--first-run" => first_run_str = val()?.to_owned(),
            "--timeout" => {
                timeout = val()?
                    .parse()
                    .map_err(|_| "--timeout must be a number of seconds")?
            }
            other => return Err(format!("unknown flag {other:?}")),
        }
    }
    if title.is_empty() {
        return Err("--title is required".into());
    }
    if script.is_empty() {
        return Err("--script is required".into());
    }
    let first_run = if first_run_str.is_empty() {
        Utc::now()
    } else {
        first_run_str
            .parse::<DateTime<Utc>>()
            .map_err(|_| format!("--first-run must be an RFC3339 UTC timestamp (e.g. 2026-11-10T13:00:00Z), got {first_run_str:?}"))?
    };
    Ok((title, script, args, dir, first_run, timeout))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let result = match parse(std::env::args().skip(1)) {
        Ok(args) => run(args).await,
        Err(e) => Err(e),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.is_empty() => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<Args, String> {
        parse(a.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_commands_and_the_db_option() {
        let a = p(&["reset-password", "alice"]).unwrap();
        assert_eq!(
            (a.db, a.command),
            (None, vec!["reset-password".into(), "alice".into()])
        );
        let a = p(&["--db", "sqlite://x.db", "list"]).unwrap();
        assert_eq!(
            (a.db.as_deref(), a.command.len()),
            (Some("sqlite://x.db"), 1)
        );
        assert!(p(&["--db"]).is_err(), "missing URL");
        assert_eq!(
            p(&["--help"]).err(),
            Some(String::new()),
            "help prints usage"
        );
    }

    #[test]
    fn default_database_is_never_created_implicitly() {
        std::env::set_var("HOME", "/home/someone");
        let url = default_db_url().unwrap();
        assert!(
            url.ends_with("/.local/share/kaaryasoochi/kaaryasoochi.db?mode=rw"),
            "{url}"
        );
        assert!(!url.contains("mode=rwc"));
    }
}
