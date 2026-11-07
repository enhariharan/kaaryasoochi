<!-- SPDX-License-Identifier: MIT
     Copyright (c) 2026 Hariharan Narayanan -->

# Folder structure

```
.
├── Cargo.toml                workspace, shared deps
├── AGENTS.md / CLAUDE.md     agent guidance (CLAUDE.md points to AGENTS.md)
├── crates/
│   ├── core/src/
│   │   ├── lib.rs
│   │   ├── dto.rs            wire types shared by server fns and UI
│   │   ├── i18n.rs           locale loader + tests (strings live in ../locales/<code>.txt)
│   │   ├── limits.rs         field limits
│   │   ├── recurrence.rs     Repeat + next-run computation
│   │   ├── settings.rs       Theme/Layout/TabOrientation/DateFormat/TimeFormat
│   │   └── validation.rs     JobInput and credential validation
│   ├── db/src/
│   │   ├── lib.rs            connect() = open + PRAGMA foreign_keys + migrate
│   │   ├── error.rs
│   │   ├── entity/           one SeaORM entity per table
│   │   ├── migration/        versioned migrations
│   │   ├── services/         auth, category, job, run (+scheduler tick), settings
│   │   └── tests.rs          service tests on in-memory SQLite
│   ├── admin/                operator CLI: list accounts, reset a password (scripts/reset-password)
│   └── app/
│       ├── Dioxus.toml
│       ├── assets/main.css
│       └── src/
│           ├── main.rs       App, Route, authenticated Shell
│           ├── state.rs      context, i18n helper, JS bridge, date formatting
│           ├── api.rs        #[server] functions
│           ├── backend.rs    server-only: DB handle, scheduler, error mapping
│           └── pages/        auth, home, job (form + history), settings
├── docs/architecture/
├── .gitea/workflows/ci.yml
├── deploy/systemd/           user units: kaaryasoochi-server.service, kaaryasoochi-app.service (opt-in)
└── scripts/                  setup-remotes.sh, run-desktop.sh, install-service.sh, reset-password, verify-migrated-jobs.sh
```
