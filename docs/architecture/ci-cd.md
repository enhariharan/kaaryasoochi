<!-- SPDX-License-Identifier: MIT
     Copyright (c) 2026 Hariharan Narayanan -->

# CI/CD

## Remotes

Two remotes, no `origin`: `gitea` (https://localhost:3000) and `github`. `scripts/setup-remotes.sh` creates them; set `GITEA_REPO` / `GITHUB_REPO` to override the default `<user>/kaaryasoochi` paths.

## Pipeline (`.gitea/workflows/ci.yml`, Gitea Actions)

Runs on push and pull request to `main`, on an `act_runner` with a label matching `runs-on: ubuntu-latest`.

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --features server -D warnings`
3. `cargo test --workspace --features kaaryasoochi-app/server` (the feature enables the executor tests, which spawn real `/bin/sh` processes)
4. `cargo check` for the wasm client (`--target wasm32-unknown-unknown --features web`)
5. On `main` only: `dx bundle --web --release`, uploaded as an artifact

Gitea Actions is largely GitHub-Actions compatible, so the same file could be copied to `.github/workflows/` if GitHub CI is wanted later. That is intentionally not enabled now.

## Runner requirements

Rust stable with the wasm target; `webkit2gtk`/`libxdo` are needed only for desktop builds and are not part of CI. The `dx` CLI version must match the pinned `dioxus` crate (`0.7.9`).

## Not covered yet

Release tagging, desktop/mobile packaging, and deployment (container image or systemd unit) are not scripted.
