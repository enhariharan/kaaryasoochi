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

## Scheduling

`backend::scheduler` starts with the first DB access and ticks every 15 s. `services::run::tick` finds jobs with `next_run <= now`, records a `job_run`, and advances `next_run` with `Repeat::next_after`. Missed occurrences (server was down) are skipped rather than replayed. There is no job command yet, so the executor is a no-op that records success; this is the single seam to replace.

## Recurrence rules

`core::recurrence::Repeat`: every n seconds/minutes/days/weeks/months, on a weekday, on the nth day of the month. Fixed intervals step from the first run; months always derive from the anchor (Jan 31 → Feb 28 → Mar 31, no drift); day-of-month clamps in short months. All in UTC.

## Field limits

Title 100, category 50, summary 200 (single line), description 5000 (characters). Username 3-32, password 10-128. Defined in `core/src/limits.rs`.

## i18n

`core::i18n::TABLE` maps key → `[en, ta, ml, te, hi]`. A test fails if any cell is empty, so a new key cannot ship without all five translations. Lookup falls back to English. Only static UI text is translated; user data is not.

## Settings

Stored per user in `user_settings` and applied immediately in the UI: theme, tab orientation, job layout, date/time format, page size, language. `System` date/time format resolves on the device via `Intl` at startup.
