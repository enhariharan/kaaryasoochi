<!-- SPDX-License-Identifier: MIT
     Copyright (c) 2026 Hariharan Narayanan -->

# Packaging

| Format | Files | Published to |
|---|---|---|
| `.deb` (Ubuntu 24.04 `noble`, 26.04 `resolute`) | `debian/`, `debian.sh` | Launchpad PPA; `.deb` on the GitHub release |
| `.rpm` (Fedora) | `rpm/kaaryasoochi.spec` | Fedora COPR; `.rpm` on the GitHub release |
| AUR (Arch) | `aur/PKGBUILD`, `aur/.SRCINFO` | aur.archlinux.org |
| Snap | `../snap/snapcraft.yaml` | Snap Store (see `docs/deploy/snap_store_checklist.md`) |

All of them build with `build.sh` and install with `install.sh`, so every package has the same layout:

```
/usr/bin/kaaryasoochi            launcher: starts the user service, opens the window
/usr/bin/kaaryasoochi-server     server wrapper (per-user data, refuses root)
/usr/bin/kaaryasoochi-admin      operator tool: list accounts, reset a password
/usr/lib/kaaryasoochi/app/       desktop window (WebKitGTK) and its assets
/usr/lib/kaaryasoochi/web/       server binary and public/
/usr/lib/systemd/user/kaaryasoochi-server.service
/usr/share/applications/kaaryasoochi.desktop, icons, metainfo
```

## How the server runs

The scheduler is a systemd **user** service, so jobs run as the user, never as root. Packages do
not enable it for every user. Each user's first `kaaryasoochi` launch enables and starts it (see
`common/kaaryasoochi`), so jobs keep running after the window closes and at every login. To run
jobs at boot without logging in: `sudo loginctl enable-linger $USER`. Data lives in
`~/.local/share/kaaryasoochi/`. Logs: `journalctl --user -u kaaryasoochi-server`.

Limit: the desktop window is built to talk to `127.0.0.1:8080`, so one user per machine can run
the server at a time.

## Offline builds

Launchpad has no network during builds, and COPR and AUR builds are more reliable without it. So
`vendor.sh` produces `dist/kaaryasoochi-<version>-vendor.tar.xz` (about 90 MB): every crate,
plus the source of the pinned `dx` 0.7.9 (verified against the crates.io checksum). With
`vendor/` unpacked at the source root, `build.sh` builds `dx` (with `disable-telemetry` and
`no-downloads`) and then the app without touching the network. Debian packages use it as a second
orig tarball (`orig-vendor`); the rpm spec and `PKGBUILD` download it from the GitHub release.

Toolchains: the workspace needs Rust 1.88. Ubuntu builds use the versioned `rustc-1.91`/`cargo-1.91`
packages (24.04's default `rustc` is 1.75). Fedora and Arch use their default `rust`.

## Release checklist

1. Bump the version everywhere: `Cargo.toml` (workspace), `snap/snapcraft.yaml`, `rpm/kaaryasoochi.spec`
   (`Version`, `%changelog`), `aur/PKGBUILD` (`pkgver`), `debian/changelog` (new top entry), and add
   a `<release>` to `common/io.github.enhariharan.Kaaryasoochi.metainfo.xml`.
2. Commit, tag `v<version>`, push the tag to GitHub, and create the GitHub release.
3. `packaging/vendor.sh`, then attach `dist/kaaryasoochi-<version>-vendor.tar.xz` to the release.
4. **PPA** (one-time setup: a Launchpad account with your OpenPGP key, and a PPA named `kaaryasoochi`):
   ```bash
   DEBSIGN_KEYID=<your key id> packaging/debian.sh source        # noble and resolute
   dput ppa:<launchpad-user>/kaaryasoochi dist/debian/kaaryasoochi_<version>-1~noble1_source.changes
   dput ppa:<launchpad-user>/kaaryasoochi dist/debian/kaaryasoochi_<version>-1~resolute1_source.changes
   ```
   Enable `arm64` in the PPA settings if you want it. Users install with
   `sudo add-apt-repository ppa:<launchpad-user>/kaaryasoochi && sudo apt install kaaryasoochi`.
5. **COPR** (one-time setup: a Fedora account, `copr-cli` with its API token, and a project named
   `kaaryasoochi` with the Fedora releases you want):
   ```bash
   cd packaging/rpm
   spectool -g -C ~/rpmbuild/SOURCES kaaryasoochi.spec
   rpmbuild -bs kaaryasoochi.spec
   copr-cli build kaaryasoochi ~/rpmbuild/SRPMS/kaaryasoochi-<version>-1.*.src.rpm
   ```
   Users install with `sudo dnf copr enable <fedora-user>/kaaryasoochi && sudo dnf install kaaryasoochi`.
6. **AUR** (one-time setup: an AUR account with your SSH key):
   ```bash
   cd packaging/aur && updpkgsums && makepkg --printsrcinfo > .SRCINFO   # commit both here
   git clone ssh://aur@aur.archlinux.org/kaaryasoochi.git /tmp/aur-kaaryasoochi
   cp PKGBUILD .SRCINFO /tmp/aur-kaaryasoochi/
   cd /tmp/aur-kaaryasoochi && git commit -am "kaaryasoochi <version>" && git push
   ```
7. Optional: attach the built `.deb` (`packaging/debian.sh deb` on Ubuntu) and `.rpm` to the
   GitHub release for people who don't use the repositories.

## Testing a package build locally

Each build compiles `dx` and the app, which takes about 20 to 30 minutes. Run it in a clean container
and turn the network off for the build step, as the build farms do:

```bash
UNSIGNED=1 packaging/debian.sh source resolute
# Ubuntu: apt-get build-dep ./dist/debian/kaaryasoochi_*.dsc in ubuntu:26.04, then, with
# --network=none: dpkg-source -x <dsc> && cd kaaryasoochi-* && dpkg-buildpackage -b -us -uc
```

For rpm and AUR, put `git archive --prefix=kaaryasoochi-<version>/ HEAD | gzip > kaaryasoochi-<version>.tar.gz`
and the vendor tarball next to the spec/PKGBUILD (in `~/rpmbuild/SOURCES` for rpm), install the
build dependencies, then run `rpmbuild -bb` or `makepkg` without network.

- **Memory.** `dx` compiles the server and the window at the same time, so a build runs about twice
  `CARGO_BUILD_JOBS` compilers at once (the repository's default is 4). Cap the container
  (`podman run --memory=8g --memory-swap=8g`) so a build that runs out of memory is stopped inside it
  rather than `systemd-oomd` acting on your session.
- **Reusing `dx`.** About half of each build is compiling `dx`. `KAARYASOOCHI_DX=/path/to/dx` makes
  `build.sh` use an existing `dx` 0.7.9 instead, e.g. one copied out of an earlier build's
  `target/dx-cli/bin/dx`. It must run on the build system: one built on Ubuntu 26.04 runs on Arch,
  but not on Fedora 45, which has a newer OpenSSL.
