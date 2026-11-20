# Ubuntu Snap Store: Publishing Checklist

<!-- source: https://claude.ai/artifact/UcS4fYe3NynvXzrJXfZYeT -->

## How the snap works

- **Classic confinement.** Kaaryasoochi runs the user's own commands and scripts, with the host's
  tools, like a crontab. Under strict confinement jobs would see core24's `/usr` instead of the
  host's, and could not read dot files or run host programs, so classic is the only workable mode.
  Classic snaps need a manual Snap Store review before they can be released (below).
- **Browser UI.** The snap ships the server with the web UI; `kaaryasoochi` opens
  `http://127.0.0.1:8080` in the default browser. The WebKitGTK window is not bundled: a classic
  snap cannot carry WebKitGTK, because its helper processes are looked up at the host's paths. The
  deb, rpm and AUR packages keep the native window.
- **Per-user service, never root.** No snap daemon (those run as root). The launcher writes
  `~/.config/systemd/user/kaaryasoochi-server.service` (`ExecStart=/snap/bin/kaaryasoochi.server`),
  enables it on first launch and starts it, so jobs run as the user. Data:
  `~/.local/share/kaaryasoochi`, the same place as the other packages.
- **Commands:** `kaaryasoochi` (start and open), `kaaryasoochi.server` (what the unit runs),
  `kaaryasoochi.admin` (list accounts, reset a password).
- The build runs `packaging/build.sh --web` from the **committed** tree (`source-type: git`), so
  commit before building. `enable-patchelf` points the binaries at core24's loader and libraries,
  so they run on hosts with an older glibc.

## Prerequisites

- [ ] Ubuntu One account at [snapcraft.io](https://snapcraft.io)
- [ ] `sudo snap install snapcraft --classic`
- [ ] LXD for clean builds: `sudo snap install lxd && sudo lxd init --auto`, then `sudo adduser $USER lxd` and log in again

## Build

- [ ] From a normal clone (not a `git worktree`; the build clones the repository inside the LXD container):
    ```
    snapcraft pack --use-lxd
    ```
    This produces `kaaryasoochi_<version>_amd64.snap`. `snapcraft pack --use-lxd --debug` opens a shell in the container on failure.
- [ ] arm64: `snapcraft remote-build` (builds on Launchpad), or build on an arm64 machine.
- [ ] After changing `snapcraft.yaml`: `snapcraft clean --use-lxd`

## Local install and smoke test

- [ ] `sudo snap install --classic --dangerous kaaryasoochi_*.snap`
- [ ] `kaaryasoochi`: the browser opens the sign-up page at `http://127.0.0.1:8080`
- [ ] `systemctl --user status kaaryasoochi-server` shows the unit active, running `/snap/kaaryasoochi/<rev>/usr/lib/kaaryasoochi/web/server`
- [ ] `ps -o user= -p "$(systemctl --user show -p MainPID --value kaaryasoochi-server)"` prints your user, not root
- [ ] Create a job that uses a host tool (e.g. `rsync --version` or a script in your home); "Run now" succeeds and shows its output
- [ ] Refresh to a new local build: the next `kaaryasoochi` launch restarts the server on the new revision
- [ ] `sudo snap remove kaaryasoochi`: the leftover user unit is skipped (`ConditionPathExists`). To delete it:
    `systemctl --user disable kaaryasoochi-server; rm ~/.config/systemd/user/kaaryasoochi-server.service; systemctl --user daemon-reload`

## Register, classic review, publish

- [ ] Register the name (one-time): `snapcraft register kaaryasoochi`
- [ ] Request classic confinement: post the request below in the
      [store-requests](https://forum.snapcraft.io/c/store-requests/19) category of the Snapcraft forum
      (follow the template in the forum's "About the classic-confinement category" post)
- [ ] Upload: `snapcraft upload kaaryasoochi_*.snap`. A classic snap is held for manual review until the request is approved.
- [ ] After approval, release: `snapcraft release kaaryasoochi <revision> edge`, test with `sudo snap install kaaryasoochi --classic --edge`, then promote to `candidate` and `stable`
- [ ] `snapcraft status kaaryasoochi`

### Classic confinement request (draft)

> **Snap:** kaaryasoochi ([source](https://github.com/enhariharan/kaaryasoochi), I am the upstream author)
>
> **What it does:** a cron-style job scheduler. A per-user server runs the user's own commands and
> scripts on a schedule (backups, rsync, database dumps, build scripts), records each run's exit
> code and output, and serves a web UI on 127.0.0.1.
>
> **Why classic:** running arbitrary, user-defined commands with the host's tools and the user's
> files is the core function, exactly what a crontab entry does. Under strict confinement (tried
> with `home`, `network` and `network-bind`), jobs see core24's `/usr` instead of the host's, cannot
> run host programs (rsync, pg_dump, git, the user's own toolchains), and cannot read dot files or
> config. No interface grants this, and the programs a user schedules are unknown at build time.
>
> **Category:** an agent that runs user-defined workloads on the user's machine. The system itself is
> managed outside the agent: Kaaryasoochi installs nothing and changes no system configuration.
>
> **What it does not do:** no root or system daemon (the server runs as the user through a systemd
> user unit the app writes to `~/.config/systemd/user`), no sudo or pkexec, no running other snaps,
> no host libraries (binaries are patched to core24 with `enable-patchelf`), and it listens only on
> 127.0.0.1:8080.

## Snap Store listing

Fill these in at `https://snapcraft.io/kaaryasoochi/listing` after the first upload.

| Field | Value |
| --- | --- |
| Title / summary | From `snapcraft.yaml` (summary ≤ 79 characters) |
| Description | From `snapcraft.yaml`; mention the browser UI and the per-user service |
| Category | Productivity or Utilities |
| License | MIT |
| Screenshots | 1 required, 3 to 5 recommended (job grid, schedule editor, run history, settings) |
| Icon | `snap/gui/kaaryasoochi.png` (256×256) |
| Contact / website | GitHub issues / repository (in `snapcraft.yaml`) |

## Ongoing maintenance

- [ ] Bump `version` in `snap/snapcraft.yaml` together with the workspace version (see `packaging/README.md`, "Release checklist")
- [ ] Rebuild and upload for each release; rebuilt snaps pick up core24 updates
- [ ] CI option: `snapcore/action-build` and `snapcore/action-publish` with `SNAPCRAFT_STORE_CREDENTIALS` from `snapcraft export-login`
