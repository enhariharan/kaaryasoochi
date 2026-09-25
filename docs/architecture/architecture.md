<!-- SPDX-License-Identifier: MIT
     Copyright (c) 2026 Hariharan Narayanan -->

# Architecture

## Overview

One Rust workspace, three crates. The UI is Dioxus fullstack: the browser (or desktop/mobile webview) renders the UI and calls **server functions** (`#[server]`) over HTTP. The server owns the SQLite database.

```
 UI (Dioxus, wasm / webview)
        │  server functions (serde over HTTP)
        ▼
 crates/app  api.rs ── backend.rs (server only: DB handle, scheduler, error masking)
        │
        ▼
 crates/db   services::{auth, category, job, run, settings}  ── SeaORM ── SQLite
        │
        ▼
 crates/core pure logic: validation, recurrence, i18n, settings enums, DTOs
```

`core` has no I/O and is compiled into both server and client. `db` is server-only.

## Multi-platform

| Target | How it runs |
|---|---|
| Web | `dx serve --web`: server + wasm client |
| Desktop | Dioxus desktop webview calling the server URL |
| Mobile | Dioxus mobile webview calling the server URL |

Desktop and mobile do not embed the database; they need a reachable server. Embedding the server in the desktop binary is a possible later step. All targets render through a webview/browser, which is why `state.rs` uses JS (`localStorage`, `Intl`) uniformly.

## Authentication

- Passwords: Argon2id (PHC string in `user.password_hash`). Login on an unknown user still performs a hash to flatten timing.
- Sessions: 256-bit random opaque token returned by `login`. Only its SHA-256 is stored (`session.token_hash`), 30-day expiry. Changing the password revokes all sessions.
- The client keeps the token in `localStorage` and passes it to each server function. Trade-off: readable by any script in the page (XSS), in exchange for one mechanism that works on web, desktop and mobile. An HttpOnly cookie for the web target is a candidate hardening.
- Passkeys: the `passkey` table exists; the WebAuthn registration/authentication ceremonies (planned: `webauthn-rs`) are not implemented.

## Authorization

Every service function takes `user_id` and filters by it; a job id belonging to another user yields `NotFound`, not `Forbidden`, so ids are not enumerable.

## Scheduling and execution

`backend::scheduler` starts with the first DB access and ticks every 5 s. Each tick calls `services::run::claim_due`, which for every job with `next_run <= now`:

1. inserts a `job_run` row with status `running` (or, if the previous run is still going, a failed row `skipped: previous run still running`, so a slow job never overlaps itself), and
2. advances `next_run` with `Repeat::next_after` *before* the job runs, so a long job can neither block the scheduler nor be picked up twice.

**Retry on failure (opt-in per job).** When enabled, a failed run (non-zero exit, timeout or failure to start) is retried up to `retry_count` more times (default 3, max 10), `retry_delay_secs` apart (default 60, max 3600). Attempt 1 is the scheduled run; retries are further `job_run` rows with `attempt = 2..`. `run::finish` queues the next attempt by setting `job.retry_at` (persisted, so a pending retry survives a server restart) and `claim_due` starts it once due. A new scheduled occurrence, a manual "Run now", rescheduling, or turning retries off cancels a pending retry; success ends the sequence. Each attempt has its own timeout. A run interrupted by a server restart is marked failed and is not retried. The failure notification is sent only after the last attempt fails. Timing is only as fine as the 5 s scheduler tick.

The backend then runs each claimed job in its own task and reports back through `run::finish` (status, exit code, output tail). On startup, `run::recover_interrupted` marks any run left `running` by a previous process as failed.

Missed occurrences (server was down) are skipped rather than replayed: the next run is the first slot after `now`. **The scheduler only runs while the server process is up**, unlike cron, which runs as a system service.

### Running commands

A job runs either a typed shell **command** or a **script file** (absolute path, shell-syntax arguments, optional working directory); both become one `/bin/sh -c` line (`core::exec::shell_line`, script paths are single-quoted). Scripts are chosen with an in-app file browser over the *server's* filesystem (`fsbrowse.rs`: names and metadata only, admin-only, canonicalised paths, 2000-entry cap). "Run now" starts the saved job immediately without touching its schedule (refused while it is already running). A failed run raises a best-effort desktop notification (`notify-send`) when the job's *notify on failure* option is on. The command runs as `/bin/sh -c <command>` in its own process group, with a cron-like environment (cleared, then `HOME`, `LOGNAME`, `USER`, `SHELL=/bin/sh`, `PATH=/usr/bin:/bin`), working directory `$HOME`, stdin closed. Success means exit status 0. It is killed (whole process group) after `timeout_secs` (default 60, max 86400; jobs created before the default changed keep the value they were saved with). Only the last 8000 characters of stdout/stderr are stored. A job with an empty command just records a no-op success.

Because this executes arbitrary code as the server's OS user, **only administrators may set or change a command** (enforced in `services::job`, not just hidden in the UI). The first account created is the administrator (`user.is_admin`); the migration promotes the lowest-id user on existing databases. Other users can still schedule command-less jobs. The server should stay bound to localhost or sit behind authentication you trust.

## Schedules

A job runs **once** (first-run time), **every n seconds**, or on a **cron schedule**. The cron expression (`core::cronexpr::CronExpr`) is the canonical form for everything calendar-based; the schedule editor's Simple / Advanced / Raw tabs are three views of the same expression (Simple = `SimplePreset`, Advanced = per-field pickers, Raw = text with live validation), plus a plain-English description and a preview of the next runs in the user's time zone (`core::crontext`).

**Own parser, not a crate.** We evaluated `cron_tab` (a thread-based in-memory scheduler over the `cron` crate) and `cron` itself (0.16 and 0.17). They were rejected because they: reject standard 5-field crontab lines (seconds required), use Quartz weekday numbering (`1-5` = Sun-Thu), AND day-of-month with weekday where crontab ORs them, skip a run whose local time falls in a DST gap, and run a fall-back hour twice. `cron_tab`'s runtime also has no persistence, missed-run policy, overlap protection or history, all of which the database-backed scheduler already provides. `CronExpr` follows Vixie cron: `0`/`7` = Sunday, lists/ranges/steps/names, `@daily`-style macros (not `@reboot`), the OR rule, a skipped local time runs at the first valid time after the gap, and a repeated one runs once.

The older `Repeat` rules (every n days/weeks/months/minutes, day-of-month, weekday sets) still decode and run. When such a job is opened, rules with an exact cron equivalent (`Repeat::to_cron`) are shown as cron; the rest are kept as an "existing repeat rule" until the user picks a new schedule.

## Recurrence rules

`core::recurrence::Repeat`: every n seconds/minutes/days/weeks/months, on one weekday, on several weekdays (bitmask, e.g. Mon-Fri), on the nth day of the month, or `Cron`. For `Cron`, the first-run time is only a "start from" bound.

Calendar rules (days, weeks, months, weekdays, day-of-month) are evaluated in the **job's time zone**, so "07:01 on weekdays" stays 07:01 local across DST changes; second/minute intervals are fixed durations. A local time that does not exist (spring-forward gap) runs at the first valid time after the gap; an ambiguous one (fall-back) runs at its first occurrence. Months derive from the anchor each time (Jan 31 → Feb 28 → Mar 31, no drift).

## Time zones

`user_settings.timezone` is `system` (default) or an IANA name. While it is `system`, the UI reports the device's zone to the server (`report_system_timezone`, stored in `user_settings.system_timezone`); the effective zone falls back to the server's own zone, then UTC.

A job's zone is **snapshotted into `job.timezone` when it is created or rescheduled** (first run or repeat changed). Changing the setting afterwards does not shift existing jobs. The UI shows and accepts times in the effective zone; storage and the API use UTC instants.

## Dashboard tab order

Categories are listed **alphabetically with `Default` always last** (`services::category::list`, covered by a test) and the dashboard renders them in that order. Tab strips follow the writing direction, so `Default` is the far end of the strip: right-most in left-to-right languages, left-most in right-to-left ones, and at the bottom of a vertical strip. `Default` is still the tab selected on load, and the active tab is scrolled into view when the strip overflows.

## Field limits

Title 100, category 50, summary 200 (single line), description 5000 (characters). Username 3-32, password 10-128. Defined in `core/src/limits.rs`.

## i18n

Seven languages: English, Tamil (`ta`), Malayalam (`ml`), Telugu (`te`), Hindi (`hi`), Sanskrit (`sa`) and Urdu (`ur`, right-to-left). Each is one plain-text file, `crates/core/locales/<code>.txt`, with a `key = text` line per string, compiled in with `include_str!`; `en.txt` is the reference. Tests require every locale to define exactly the English keys with no empty or untranslated text, and every literal `t("key")` in the UI to exist. Lookup falls back to English, then to an empty string. Only static UI text is translated: user data, the plain-English schedule description and cron parse errors are English.

### Right-to-left

Every locale file declares `@dir = ltr|rtl`. `Language::dir()` reads it, and the UI puts `dir` and `lang` on the page root and on `<html>` (so scrollbars, native pickers and margins mirror too). The stylesheet uses only logical properties (`margin-inline-start`, `inset-inline-end`, `text-align: start`, ...), so flex/grid rows, tab strips, the add button, tables and segmented controls mirror without per-language code; a test fails if a physical `left`/`right` property is added. Content that is always left-to-right (paths, commands, cron text, monospace output) stays LTR, dates are isolated (`<bdi dir="ltr">`), and user-typed text uses `dir="auto"` so mixed-direction titles render correctly. The "collapsed" chevron is flipped for RTL. Urdu (`ur.txt`, `@dir = rtl`) is the shipped right-to-left language and was used to verify the mirrored layout in a browser (dashboard, job page, settings, vertical tabs), with the Nastaliq font stack in `main.css`. The login and register pages show English/LTR because the language is a per-user setting that is only known after sign-in.

To add a language: create `locales/<code>.txt` (copy `en.txt`), add a `Language` variant and its `code`/`native_name`/`ALL` entries and a line in `LOCALE_SOURCES` in `core/src/i18n.rs`. The translations in the non-English files are best-effort drafts and should be reviewed by a native speaker.

## Settings

Stored per user in `user_settings` and applied immediately in the UI: theme, tab orientation, job layout, date/time format, page size, language, time zone. `System` date/time format resolves on the device via `Intl` at startup.
