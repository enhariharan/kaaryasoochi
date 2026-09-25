<!-- SPDX-License-Identifier: MIT
     Copyright (c) 2026 Hariharan Narayanan -->

# AGENTS.md

Guidance for AI agents and contributors. `CLAUDE.md` just points here.

## Project
Kaaryasoochi: cron-job scheduler/manager. Rust workspace, Dioxus 0.7 fullstack, SQLite + SeaORM. See `README.md` and `docs/architecture/`.

## Layout
- `crates/core`: pure logic, no I/O. Put anything unit-testable here (recurrence, validation, i18n, DTOs).
- `crates/db`: SeaORM entities, migrations, `services::*`. All business rules and authorization (user-scoped queries) live here.
- `crates/app`: Dioxus UI (`pages/`, `state.rs`) and `#[server]` functions (`api.rs`). `backend.rs` is server-only (feature `server`).

## Commands
- Test: `cargo test --workspace --features kaaryasoochi-app/server` (the feature enables the executor tests)
- Lint: `cargo clippy --workspace --all-targets --features server -- -D warnings` and `cargo fmt --all --check`
- Web build check: `cargo check -p kaaryasoochi-app --target wasm32-unknown-unknown --features web`
- Run: `cd crates/app && dx serve --web`

## Conventions
- Dioxus is pinned to `=0.7.9` to match the `dx` CLI. Bump both together.
- Every service function takes a `user_id` and must scope queries to it. Never fetch by id alone.
- New user-visible text: add the key to **every** `crates/core/locales/<code>.txt` (en, ta, ml, te, hi, sa, ur). Tests enforce identical key sets, no empty/untranslated text, and that every `t("key")` used by the UI exists.
- Field limits live in `crates/core/src/limits.rs`; the UI and validation both use them.
- Schema changes: add a new migration file in `crates/db/src/migration/`; never edit an applied one.
- Times are stored and transported as UTC. Calendar recurrence is evaluated in the job's stored IANA zone (`Repeat::next_after(.., tz)`); the UI converts using the user's effective zone (`state.rs`).
- Layout must mirror for right-to-left: use logical CSS properties only (`margin-inline-start`, `inset-inline-end`, `text-align: start`); a test rejects `left`/`right`. Wrap dates in `bdi { dir: "ltr" }`, give user-typed text `dir: "auto"`, keep paths/commands/cron text LTR (`.mono`).
- Check UI in the engine users run. Desktop = WebKitGTK, which differs from Chrome (e.g. it drew `<select>` as a light native widget, making values unreadable in dark themes; DOM values looked fine, only pixels showed it). Reading DOM state is not enough for visual bugs: render and look, or sample pixels. An offscreen `WebKit2.WebView` in a `Gtk.OffscreenWindow` (Python `gi`, `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11`, ephemeral context with no proxy) runs the web build without opening a window on the desktop; test dark, light and RTL.
- The UI text guard (`i18n::every_key_used_by_the_ui_exists`) fails on any `t("key")` not in `en.txt`; a missing key renders as blank. When scripting edits, assert that the text you replace was found (rustfmt reflows code, so silent no-op replaces have bitten us twice).
- Web-only failures are invisible to native tests (e.g. `Utc::now()` panics on wasm without chrono `wasmbind`). After UI changes, build `dx build --web`, serve it against a scratch DB, and drive it with headless Chrome (CDP) rather than trusting `cargo check`.
- Job commands run arbitrary code as the server user: keep the admin-only check in `services::job`, and never log command text.
- In `rsx!`, use `{expr}` for anything non-trivial. Escaped quotes inside `"{...}"` strings do not parse.
- Server errors: user-facing `DbError`s pass through; DB/internal errors are masked in `backend::err`.

## Licensing
MIT. Every source, config, script and doc file starts with the two-line SPDX header (see any `.rs` file). Add it to new files. `LICENSE`, `Cargo.lock` and `.gitignore` are exempt.

## Git
Remotes are `gitea` and `github` (no `origin`). Do not push without being asked.

## Known gaps
Passkeys (table exists, no flow). The scheduler runs only while the server process is up (no system service yet).
