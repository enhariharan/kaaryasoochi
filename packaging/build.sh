#!/bin/sh
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Release build used by every package (deb/PPA, rpm, AUR, snap).
#
#   packaging/build.sh [--web]
#
# Produces target/dx/kaaryasoochi-app/release/{linux/app,web} and target/release/kaaryasoochi-admin.
# The default builds the WebKitGTK window and the server; --web builds the server with the browser
# UI instead (needs the wasm32-unknown-unknown target).
#
# With vendor/ present (packaging/vendor.sh) the build is fully offline: crates come from
# vendor/crates and dx is compiled from vendor/dioxus-cli-* with downloads disabled, so a --web
# build then also needs wasm-bindgen (the version in Cargo.lock) and wasm-opt on PATH. Without
# vendor/, a dx of the pinned version on PATH is used, or installed into target/dx-cli, and dx
# fetches those tools itself.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DX_VERSION=0.7.9
PLATFORM=--desktop
[ "${1:-}" = --web ] && PLATFORM=--web
export DX_TELEMETRY_ENABLED=false
export KAARYASOOCHI_SERVER_URL="${KAARYASOOCHI_SERVER_URL:-http://127.0.0.1:8080}"
cd "$ROOT"

if [ -d vendor/crates ]; then
  export CARGO_HOME="${CARGO_HOME:-$ROOT/target/cargo-home}"
  export CARGO_NET_OFFLINE=true
  mkdir -p "$CARGO_HOME"
  cat >"$CARGO_HOME/config.toml" <<EOF
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "$ROOT/vendor/crates"
EOF
fi

dx_ok() { [ -x "$1" ] && "$1" --version 2>/dev/null | grep -Eq "^dioxus $DX_VERSION( |$)"; }

# KAARYASOOCHI_DX: use this dx binary (must be $DX_VERSION) instead of building one, e.g. a cached
# copy in CI or local test builds.
if [ -n "${KAARYASOOCHI_DX:-}" ]; then
  dx_ok "$KAARYASOOCHI_DX" || {
    echo "build.sh: KAARYASOOCHI_DX=$KAARYASOOCHI_DX is not a working dx $DX_VERSION" >&2
    exit 1
  }
  DX=$KAARYASOOCHI_DX
elif DX="$ROOT/target/dx-cli/bin/dx" && dx_ok "$DX"; then
  :
elif [ -d "vendor/dioxus-cli-$DX_VERSION" ]; then
  cargo install --locked --offline --path "vendor/dioxus-cli-$DX_VERSION" \
    --features disable-telemetry,no-downloads \
    --root "$ROOT/target/dx-cli" --target-dir "$ROOT/target/dx-cli/build"
elif command -v dx >/dev/null 2>&1 && dx_ok "$(command -v dx)"; then
  DX=$(command -v dx)
else
  cargo install --locked dioxus-cli --version "$DX_VERSION" \
    --features disable-telemetry \
    --root "$ROOT/target/dx-cli" --target-dir "$ROOT/target/dx-cli/build"
fi
dx_ok "$DX" || { echo "build.sh: dx $DX_VERSION is not usable at $DX" >&2; exit 1; }

(cd crates/app && "$DX" build "$PLATFORM" --release)
cargo build --release --locked -p kaaryasoochi-admin
