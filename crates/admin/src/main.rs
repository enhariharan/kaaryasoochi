// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Operator tool for the Kaaryasoochi database.
//!
//!     kaaryasoochi-admin list
//!     kaaryasoochi-admin reset-password <username>
//!
//! The database is `--db <url>`, else `$KAARYASOOCHI_DATABASE_URL`, else the default location
//! (`~/.local/share/kaaryasoochi/kaaryasoochi.db`, never created if missing). The new password is
//! typed at the terminal, twice and without echo; it is not accepted from arguments, the
//! environment or a pipe, so it cannot end up in shell history or process listings.

use std::process::ExitCode;

use kaaryasoochi_db::services::recovery;

const USAGE: &str = "usage:\n  kaaryasoochi-admin [--db URL] list\n  kaaryasoochi-admin [--db URL] reset-password <username>";

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
        _ => Err(String::new()),
    }
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
