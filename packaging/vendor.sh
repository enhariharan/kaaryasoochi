#!/bin/sh
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Creates dist/kaaryasoochi-<version>-vendor.tar.xz: every crate the build needs plus the source
# of the pinned dioxus-cli (dx), so packages build with no network (Launchpad, COPR, AUR).
#
#   packaging/vendor.sh
#
# Layout, unpacked at the source root as vendor/:
#   vendor/crates/              `cargo vendor` output for the workspace and for dx
#   vendor/dioxus-cli-<ver>/    dx source, built by packaging/build.sh
# Attach the tarball to the GitHub release of the same tag; the rpm spec and PKGBUILD download it.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DX_VERSION=0.7.9
# From the crates.io index entry for dioxus-cli 0.7.9.
DX_SHA256=d2dc7a9f1b9fc2fb649fea3d3d644d2869e39d3c295813917ee9f2626bd97289
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/Cargo.toml" | head -n1)
OUT="$ROOT/dist/kaaryasoochi-$VERSION-vendor.tar.xz"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM
mkdir -p "$work/vendor" "$ROOT/dist"

crate="$work/dioxus-cli-$DX_VERSION.crate"
curl -fsSL -o "$crate" "https://static.crates.io/crates/dioxus-cli/dioxus-cli-$DX_VERSION.crate"
echo "$DX_SHA256  $crate" | sha256sum -c --quiet
tar -xzf "$crate" -C "$work/vendor"

# Both lockfiles are honoured (--locked), so the vendored versions are the tested ones.
cargo vendor --quiet --locked --versioned-dirs \
  --manifest-path "$ROOT/Cargo.toml" \
  --sync "$work/vendor/dioxus-cli-$DX_VERSION/Cargo.toml" \
  "$work/vendor/crates" >/dev/null

# Reproducible archive: fixed order, owner and timestamps.
tar -C "$work" --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -cf - vendor | xz -T0 -9 >"$OUT"
echo "$OUT ($(du -h "$OUT" | cut -f1))"
