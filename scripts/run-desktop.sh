#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Starts the server (if not already listening) and the Linux desktop app.
# Build first:  cd crates/app && dx build --desktop --release
# Stop both:    scripts/run-desktop.sh stop
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REL="$ROOT/target/dx/kaaryasoochi-app/release"
HOST=127.0.0.1
PORT="${PORT:-8080}" # must match the KAARYASOOCHI_SERVER_URL the app was built with
DATA="${KAARYASOOCHI_DATA_DIR:-$HOME/.local/share/kaaryasoochi}"

if [[ "${1:-}" == "stop" ]]; then
  pkill -f "$REL/linux/app/kaaryasoochi-app" || true
  pkill -f "$REL/web/server" || true
  exit 0
fi

[[ -x "$REL/linux/app/kaaryasoochi-app" ]] || { echo "Not built. Run: cd crates/app && dx build --desktop --release" >&2; exit 1; }
mkdir -p "$DATA"

if ss -ltn | grep -q ":$PORT "; then
  echo "server already listening on :$PORT"
else
  (cd "$REL/web" && KAARYASOOCHI_DATABASE_URL="sqlite://$DATA/kaaryasoochi.db?mode=rwc" IP=$HOST PORT=$PORT \
    setsid nohup "$REL/web/server" >"$DATA/server.log" 2>&1 &)
  for _ in $(seq 20); do ss -ltn | grep -q ":$PORT " && break; sleep 0.5; done
  echo "server started on $HOST:$PORT (log: $DATA/server.log)"
fi

(cd "$REL/linux/app" && setsid nohup "$REL/linux/app/kaaryasoochi-app" >"$DATA/app.log" 2>&1 &)
echo "desktop app started (log: $DATA/app.log)"
