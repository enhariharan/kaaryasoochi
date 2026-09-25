<!-- SPDX-License-Identifier: MIT
     Copyright (c) 2026 Hariharan Narayanan -->

# Kaaryasoochi

*Kaarya* (job) + *Soochi* (list): a multi-platform app to schedule and manage cron-style jobs.

Licensed under MIT (see [LICENSE](LICENSE)).

Rust · [Dioxus](https://dioxuslabs.com) 0.7 (fullstack) · SQLite via [SeaORM](https://www.sea-ql.org/SeaORM/)

## Status

| Area | State |
|---|---|
| Username/password auth (Argon2id, revocable sessions) | Done |
| Passkeys | Schema only; WebAuthn flow not implemented |
| Dashboard: category tabs (horizontal/vertical), grid/list, add-job button | Done |
| Job create/edit/delete, searchable-or-new category, repeat rules | Done |
| Run history (paged, page size configurable) | Done |
| Settings (theme, layout, formats, page size, language, name, password) | Done |
| i18n: English, Tamil, Malayalam, Telugu, Hindi, Sanskrit, Urdu (right-to-left) (`crates/core/locales/`) | Done |
| Retry on failure (count and delay configurable per job) | Done |
| Scheduler and command execution | Jobs can run a shell command (admin-only), with timeout, exit code and output capture |
| Full crontab schedules (Simple / Advanced / Raw builder, live preview) | Done |
| Script picker (server-side file browser), Run now, failure notification | Done |
| Run history folded by default, last result in the header | Done |
| Time zone setting (default: system) | Done |

## Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target
- Dioxus CLI matching the crate version: `cargo install dioxus-cli --version 0.7.9 --locked`
- Linux desktop builds: `webkit2gtk-4.1` and `libxdo` dev packages

## Run

```bash
cd crates/app
dx serve --web              # fullstack: server + browser client
dx serve --desktop          # dev: desktop client with hot reload
```

Release desktop build and launch (Linux):

```bash
(cd crates/app && dx build --desktop --release)   # ~3 min, capped to 4 jobs
scripts/run-desktop.sh                            # starts server + app
scripts/run-desktop.sh stop                       # stops both
```

The server creates `./kaaryasoochi.db` on first use. Override with
`KAARYASOOCHI_DATABASE_URL=sqlite:///path/to/file.db?mode=rwc`.

## Test

```bash
cargo test --workspace --features kaaryasoochi-app/server
cargo clippy --workspace --all-targets --features server -- -D warnings
```

## Layout

```
crates/core   pure domain logic (recurrence, validation, i18n, settings, DTOs)
crates/db     SeaORM entities, migrations, services (auth, jobs, runs, settings)
crates/app    Dioxus UI + server functions
docs/architecture/   architecture, folder structure, data model, CI/CD
```

See [docs/architecture](docs/architecture/) for details and [AGENTS.md](AGENTS.md) for contributor/agent conventions.

## Remotes

Two remotes are used: `gitea` (https://localhost:3000) and `github`. There is no `origin`.
