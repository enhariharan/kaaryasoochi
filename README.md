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
| Runs as a systemd user service (Linux); missed runs catch up once after downtime | Done |
| Secret question + answer (mandatory), forgot password, change password / secret in Settings | Done |
| `scripts/reset-password` (operator reset, no old password needed) | Done |
| Right-to-left layout (Urdu) | Done |

## How the pieces fit

Kaaryasoochi is a **server** plus a **client**. The server holds the SQLite database, the scheduler
and the job executor, so jobs run on the machine the server runs on, as the user that runs it. The
client is the UI: a browser tab (web) or a native window (desktop, mobile).

- **Web:** the browser loads the UI from the server and talks to it on the same address. Nothing to configure.
- **Desktop and mobile:** the app has no server of its own. It is built to talk to
  `http://127.0.0.1:8080` unless you set `KAARYASOOCHI_SERVER_URL` **at build time**
  (`KAARYASOOCHI_SERVER_URL=http://host:8080 dx build ...`).
- **The server is Unix-only** (Linux or macOS): it runs jobs with `/bin/sh` and uses Unix file
  permissions. It does not build on native Windows; use WSL or a Linux/macOS machine for the server.
- Job commands run as the server's OS user, so only administrators (the first account) can set them.

## Platform support

"Verified" means I built and ran it. Everything else is the standard `dx` command for that target and
has **not** been tried on this project; expect to fix things.

| Target | Status | Notes |
|---|---|---|
| **Server** (Linux) | Verified | Also serves the web UI; runs as a systemd service |
| **Web** (browser) | Verified: `dx build --web`, checked in Chrome and WebKit | `dx serve --web` and `dx bundle` not run |
| **Linux desktop** | Verified: release build, run, installed as a service | The primary desktop target |
| macOS desktop | Not verified | Same code; needs a Mac. Server side works on macOS in principle |
| Windows desktop | Not verified | The server part cannot build natively; the client would need the server disabled or built elsewhere |
| Android | Not verified | Rust Android targets are installed here, but no build has been attempted |
| iOS | Not verified | Needs macOS and Xcode |

## Accounts, forgotten passwords and resets

- **Sign-up** asks for a username, password, and a **secret question and answer** (both required).
  The first account created is the administrator (only administrators can attach commands to jobs).
- **Forgot password?** (login page): enter your username, answer your secret question, then choose a
  new password (typed twice). Five wrong answers lock recovery for that account for 15 minutes.
- **Settings**: *Change password* needs your current password **and** the secret answer;
  *Secret question and answer* changes them (needs your current password and current answer).
  Accounts created before this feature show a reminder until a secret question is set.
- **Locked out, or no secret question set?** Reset from the server's terminal. The new password is
  typed twice with no echo and never appears in arguments or history:

  ```bash
  scripts/reset-password              # list accounts, and whether each has a secret question
  scripts/reset-password USERNAME     # set a new password (also signs the account out everywhere)
  ```

  It works while the service runs and needs no old password (shell access to the server is the
  proof). The first run builds the small admin tool. Set `KAARYASOOCHI_DATABASE_URL` if your
  database is not at `~/.local/share/kaaryasoochi/kaaryasoochi.db`.

## Prerequisites

Everywhere:

- Rust (stable) with the `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- The Dioxus CLI **at the version the crate is pinned to**: `cargo install dioxus-cli --version 0.7.9 --locked`

Linux (Ubuntu/Debian package names):

```bash
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libxdo-dev libssl-dev lld rsync
```

- `lld` is **required** on Linux: `.cargo/config.toml` links with it (it uses far less memory than the default linker).
- `rsync` is used by `scripts/install-service.sh`. Optional: `sqlite3` (for `scripts/verify-migrated-jobs.sh`) and `libnotify-bin` (desktop notifications on job failure).

macOS, Windows, Android, iOS: install the platform's normal Rust GUI toolchain (Xcode command line
tools; Visual Studio Build Tools and WebView2; Android SDK + NDK and `rustup target add
aarch64-linux-android`; Xcode and `aarch64-apple-ios`). Not verified here.

## Server

Runs the API, the scheduler and (for web) the UI. Data lives in one SQLite file, created on first use.

| Environment variable | Default | Meaning |
|---|---|---|
| `IP` / `PORT` | `127.0.0.1` / `8080` | Address to listen on. Use `IP=0.0.0.0` to accept other machines (see the note under Mobile) |
| `KAARYASOOCHI_DATABASE_URL` | `sqlite://kaaryasoochi.db?mode=rwc` | e.g. `sqlite:///home/you/.local/share/kaaryasoochi/kaaryasoochi.db?mode=rwc` |
| `KAARYASOOCHI_WAIT_FOR_TCP` | unset | Comma-separated `host:port` the scheduler waits for before running anything (up to 3 minutes), e.g. `127.0.0.1:5432` |

The server starts scheduling as soon as it is up. A job that was due while the server was down runs
**once** on return (in the order the jobs were due), then resumes its normal schedule.

Run a built server directly (from its output folder, which also holds the web files):

```bash
cd target/dx/kaaryasoochi-app/release/web && IP=127.0.0.1 PORT=8080 ./server
```

## Web

```bash
cd crates/app
dx serve --web                    # development, with hot reload            (not verified)
dx build --web                    # debug build: target/dx/kaaryasoochi-app/debug/web      (verified)
dx build --web --release          # release build: .../release/web                          (release not run)
dx bundle --web --release         # what CI runs; packaged output                            (not verified)
```

Output is `web/server` (the server) plus `web/public` (the UI). Run `server` from that folder and open
`http://127.0.0.1:8080`.

## Linux desktop

Build (the gentle form caps memory and CPU, which matters on a machine that also does other work;
a plain build once exhausted RAM because `dx` compiles the server and client at the same time):

```bash
cd crates/app
dx build --desktop --release                                   # ~3-4 minutes

# same, but limited to 8 GB of RAM and low priority:
systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 \
  nice -n 15 ionice -c3 dx build --desktop --release
```

Output, under `target/dx/kaaryasoochi-app/release/`: `linux/app/kaaryasoochi-app` (the desktop app)
and `web/server` (the backend). Development with hot reload: `dx serve --desktop` (not verified).

Run it (starts the server first if needed):

```bash
scripts/run-desktop.sh              # start server + app
scripts/run-desktop.sh stop         # close the app; an installed server service keeps running
scripts/run-desktop.sh stop-all     # also stop the server
```

Or run the installed binary directly, once the server is up on `:8080`:
`~/.local/share/kaaryasoochi/app/linux/kaaryasoochi-app`

### Run the server as a service (jobs keep running, and catch up after the PC was off)

A systemd **user** service, so it needs no root. With lingering enabled (`loginctl enable-linger $USER`;
check with `loginctl show-user $USER -p Linger`) it starts at boot without anyone logging in.

```bash
scripts/install-service.sh --wait-for 127.0.0.1:5432   # deploy the last release build as the service
scripts/install-service.sh                             # re-run after every rebuild
scripts/install-service.sh --with-app                  # also open the desktop window at each graphical login
scripts/install-service.sh --uninstall
journalctl --user -u kaaryasoochi-server -f            # logs
systemctl --user status kaaryasoochi-server            # state; also: restart / stop / start
```

- The installer copies the build to `~/.local/share/kaaryasoochi/app`, so rebuilding never touches files the service is using.
- It refuses to start if something else already holds port 8080: stop a manually started server first (`scripts/run-desktop.sh stop-all`).
- `--wait-for` holds the scheduler until an endpoint your jobs depend on (say a database) is reachable, so a catch-up run right after boot does not fail because that service is still starting.
- The service restarts after a crash. Stopping or restarting it ends running jobs; they are recorded as failed ("interrupted").
- Only Linux/systemd is provided. There is no launchd (macOS) or Windows service equivalent.

## macOS and Windows desktop (not verified)

```bash
cd crates/app
dx build --desktop --release     # run on the target OS; a desktop build cannot be cross-compiled
dx bundle --desktop --release    # installer/app bundle
```

Set `KAARYASOOCHI_SERVER_URL` if the server is not on the same machine. On Windows the server cannot
be built natively (Unix-only), so point the client at a server running on Linux, macOS or WSL; the
build may also need `--fullstack false` to skip the server part.

## Mobile: Android and iOS (not verified)

```bash
cd crates/app
KAARYASOOCHI_SERVER_URL=http://<server-address>:8080 dx serve --android      # or --ios
KAARYASOOCHI_SERVER_URL=http://<server-address>:8080 dx build --android --release
KAARYASOOCHI_SERVER_URL=http://<server-address>:8080 dx bundle --android --release   # or --ios
```

- `127.0.0.1` on a phone is the phone itself, so the URL must be an address the device can reach (in an Android emulator, the host machine is `10.0.2.2`).
- For that, the server must listen beyond localhost: `IP=0.0.0.0`. The connection is plain HTTP and the server can run commands, so do this only on a network you trust, or put a TLS reverse proxy in front.

## Test

```bash
cargo test --workspace --features kaaryasoochi-app/server
cargo clippy --workspace --all-targets --features kaaryasoochi-app/server -- -D warnings
cargo fmt --all --check
cargo check -p kaaryasoochi-app --target wasm32-unknown-unknown --features web   # the browser build compiles
```

Unit and service tests do not exercise the UI. After UI changes, build the web target and look at it
in the engine users run (WebKit for the desktop app): see [AGENTS.md](AGENTS.md).

## Layout

```
crates/core   pure domain logic (recurrence, validation, i18n, settings, DTOs)
crates/db     SeaORM entities, migrations, services (auth, recovery, jobs, runs, settings)
crates/admin  operator CLI (list accounts, reset a password)
crates/app    Dioxus UI + server functions
docs/architecture/   architecture, folder structure, data model, CI/CD
```

See [docs/architecture](docs/architecture/) for details and [AGENTS.md](AGENTS.md) for contributor/agent conventions.

## Remotes

Two remotes are used: `gitea` (`ssh://git@localhost:2222/hariharan/kaaryasoochi.git`) and `github`
(`git@github.com:enhariharan/kaaryasoochi.git`). There is no `origin`.
