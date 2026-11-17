# Ubuntu Snap Store: Publishing Checklist

<!-- source: https://claude.ai/artifact/UcS4fYe3NynvXzrJXfZYeT -->

## Prerequisites

- [ ] Create or log in to an Ubuntu One account at [snapcraft.io](https://snapcraft.io)
- [ ] Install snapcraft: `sudo snap install snapcraft --classic`
- [ ] Install LXD for reproducible clean builds: `sudo snap install lxd && sudo lxd init --auto`, then `sudo adduser $USER lxd` and re-login
- [ ] Install dioxus-cli at the pinned version: `cargo install dioxus-cli --version 0.7.9`
- [ ] Confirm both release binaries build cleanly via `scripts/run-desktop.sh` (server + desktop)
- [ ] Pick a snap name (`kaaryasoochi` or similar) and verify it is not taken at [snapcraft.io/snaps](https://snapcraft.io/snaps)

## `snapcraft.yaml` Configuration

Create `snap/snapcraft.yaml` at the repo root. Key fields:

| Field | Value |
| --- | --- |
| `name` | `kaaryasoochi` (must match the registered name) |
| `base` | `core24` (Ubuntu 24.04 LTS runtime) |
| `version` | match `version` in `Cargo.toml` (e.g. `0.1.0`) |
| `summary` | ≤ 79 chars — e.g. `Schedule and manage cron-style jobs` |
| `confinement` | `devmode` while testing; switch to `strict` before stable release |
| `grade` | `devel` while testing; `stable` for production |

**Parts** — one part using `plugin: rust` with an `override-build` that:

1. Installs `dioxus-cli` at `0.7.9`
2. Runs `dx build --desktop --release` (produces the desktop binary and the server under `target/dx/kaaryasoochi-app/release/`)
3. Copies both binaries into `$SNAPCRAFT_PART_INSTALL/usr/bin/`

Build packages needed: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libssl-dev`, `pkg-config`.
Stage packages needed: `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libssl3`, `ca-certificates`.

**Apps** — declare two entries:

- `kaaryasoochi` — the desktop binary; plugs: `desktop`, `desktop-legacy`, `wayland`, `x11`, `network`, `home`, `opengl`, `unity7`
- `kaaryasoochi-server` — the server binary; `daemon: simple`; plugs: `network`, `network-bind`, `home`; `environment`: `KAARYASOOCHI_DATA_DIR: $SNAP_USER_DATA`

- [ ] Create `snap/snapcraft.yaml` from the template above
- [ ] Set `KAARYASOOCHI_SERVER_URL=http://127.0.0.1:8080` as a build-time env var in the desktop part (must match how the binary was originally compiled)
- [ ] Confirm `libwebkit2gtk-4.1-dev` is available on the build base (`apt-cache show` on core24)

## Build the Snap

- [ ] From the repo root, log in: `snapcraft login`
- [ ] Build in an LXD container for a reproducible environment:
    ```
    snapcraft --use-lxd
    ```
    This produces `kaaryasoochi_<version>_amd64.snap` in the current directory.
- [ ] On first run, if LXD is not yet initialized for snapcraft: `snapcraft --use-lxd --debug` drops you into the container shell on failure so you can diagnose missing packages.
- [ ] To iterate quickly without a container (accepts host libraries): `snapcraft` (no `--use-lxd`), but always do a final clean LXD build before uploading.
- [ ] Clean between builds when changing `snapcraft.yaml`: `snapcraft clean`

## Local Install & Smoke-Test

- [ ] Install the unsigned snap: `sudo snap install --dangerous kaaryasoochi_*.snap`
- [ ] If using `confinement: devmode`: `sudo snap install --devmode --dangerous kaaryasoochi_*.snap`
- [ ] Start the server service: `sudo snap start kaaryasoochi.kaaryasoochi-server`
- [ ] Check the server is listening: `ss -ltn | grep 8080`
- [ ] Launch the desktop app: `snap run kaaryasoochi`
- [ ] Log in, create a job, verify it shows in run history
- [ ] Check snap logs for errors: `snap logs kaaryasoochi.kaaryasoochi-server -n 50`
- [ ] Confirm data directory resolves to `$SNAP_USER_DATA` (not the bare `~/.local/share/kaaryasoochi` path)
- [ ] Remove before re-installing a new build: `sudo snap remove kaaryasoochi`

## Register & Publish

- [ ] Register the name (one-time): `snapcraft register kaaryasoochi`
- [ ] Upload and release to `edge` for internal testing: `snapcraft upload --release=edge kaaryasoochi_*.snap`
- [ ] Install from the store to verify the upload: `sudo snap install kaaryasoochi --channel=edge`
- [ ] Promote to `beta` once functional: `snapcraft release kaaryasoochi <revision> beta`
- [ ] Promote to `candidate` for wider testing, then `stable` for general availability:
    ```
    snapcraft release kaaryasoochi <revision> candidate
    snapcraft release kaaryasoochi <revision> stable
    ```
- [ ] Request a manual review if `confinement: strict` triggers an auto-review hold (common for `network-bind` and `desktop` plugs together)
- [ ] Check release status: `snapcraft status kaaryasoochi`

## Snap Store Listing

Fill these in at [snapcraft.io/snaps/kaaryasoochi/listing](https://snapcraft.io) after uploading.

| Field | Requirement |
| --- | --- |
| Summary | ≤ 79 characters |
| Description | Markdown supported; explain the server + desktop architecture |
| Category | Productivity or Utilities |
| License | MIT (matches `Cargo.toml`) |
| Screenshots | 1 required, 3–5 recommended; 1280×800 or 480×800 PNG/JPG |
| Icon | 256×256 PNG (the existing `crates/app/assets/icons/` ones may work) |
| Contact URL | GitHub issues page or personal site |
| Website | Repo URL |

- [ ] Write the store description (mention cron scheduling, run history, i18n, systemd integration)
- [ ] Capture screenshots: login screen, job grid, job create dialog, run history, settings
- [ ] Export the existing SVG icon to a 256×256 PNG for the store (project already has `scripts/gen-icons.py`)
- [ ] Set the contact and website URLs
- [ ] Review the listing preview before submitting for review

## Ongoing Maintenance

- [ ] Bump `version` in root `Cargo.toml` before each release; snapcraft reads it via `snapcraftctl set-version`
- [ ] Re-run `snapcraft --use-lxd` and upload: `snapcraft upload --release=edge kaaryasoochi_*.snap`
- [ ] Automate with GitHub Actions: use the `snapcore/action-build` and `snapcore/action-publish` actions; store credentials as a repository secret (`SNAPCRAFT_STORE_CREDENTIALS` from `snapcraft export-login`)
- [ ] Keep `base: core24` current — Canonical releases periodic core updates; rebuilt snaps pick them up automatically
- [ ] Watch for webkit2gtk security updates that require a forced rebuild even with no source change
- [ ] Monitor [forum.snapcraft.io](https://forum.snapcraft.io) for policy or confinement changes that affect desktop or network-bind plugs
