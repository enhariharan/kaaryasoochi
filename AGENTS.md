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
- Test: `cargo test --workspace`
- Lint: `cargo clippy --workspace --all-targets --features server -- -D warnings` and `cargo fmt --all --check`
- Web build check: `cargo check -p kaaryasoochi-app --target wasm32-unknown-unknown --features web`
- Run: `cd crates/app && dx serve --web`

## Conventions
- Dioxus is pinned to `=0.7.9` to match the `dx` CLI. Bump both together.
- Every service function takes a `user_id` and must scope queries to it. Never fetch by id alone.
- New user-visible text: add a row to `crates/core/src/i18n.rs` with all 5 languages (a test enforces no gaps).
- Field limits live in `crates/core/src/limits.rs`; the UI and validation both use them.
- Schema changes: add a new migration file in `crates/db/src/migration/`; never edit an applied one.
- Times are stored and transported as UTC; conversion to local happens in the UI (`state.rs`).
- In `rsx!`, use `{expr}` for anything non-trivial. Escaped quotes inside `"{...}"` strings do not parse.
- Server errors: user-facing `DbError`s pass through; DB/internal errors are masked in `backend::err`.

## Licensing
MIT. Every source, config, script and doc file starts with the two-line SPDX header (see any `.rs` file). Add it to new files. `LICENSE`, `Cargo.lock` and `.gitignore` are exempt.

## Git
Remotes are `gitea` and `github` (no `origin`). Do not push without being asked.

## Known gaps
Passkeys (table exists, no flow); jobs have no executable command, so the scheduler only records runs.
