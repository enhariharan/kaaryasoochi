#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Opens the Linux desktop app, making sure the server is running first.
#
#   scripts/run-desktop.sh         start the server if needed, then the app
#   scripts/run-desktop.sh stop    close the app (an installed server service keeps running)
#   scripts/run-desktop.sh stop-all  also stop the server service / a manually started server
#
# If scripts/install-service.sh has been run, the server is the systemd user service and the app
# is the installed copy. Otherwise this falls back to starting both straight from the build tree.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD="$ROOT/target/dx/kaaryasoochi-app/release"
INSTALLED="$HOME/.local/share/kaaryasoochi/app"
HOST=127.0.0.1
PORT="${PORT:-8080}" # must match the KAARYASOOCHI_SERVER_URL the app was built with
DATA="${KAARYASOOCHI_DATA_DIR:-$HOME/.local/share/kaaryasoochi}"
SERVICE=kaaryasoochi-server.service

have_service() { systemctl --user cat "$SERVICE" >/dev/null 2>&1; }
listening() { ss -ltn | grep -q ":$PORT "; }

# Prefer the installed copy of the app; fall back to the build tree.
if [ -x "$INSTALLED/linux/kaaryasoochi-app" ]; then
  APP_BIN="$INSTALLED/linux/kaaryasoochi-app"; APP_DIR="$INSTALLED/linux"
else
  APP_BIN="$BUILD/linux/app/kaaryasoochi-app"; APP_DIR="$BUILD/linux/app"
fi
SERVER_BIN="$BUILD/web/server"

case "${1:-}" in
  stop|stop-all)
    pkill -f "^$APP_BIN" || true
    if [ "$1" = stop-all ]; then
      have_service && systemctl --user stop "$SERVICE" || true
      pkill -f "^$SERVER_BIN" || true
    fi
    exit 0 ;;
  "") ;;
  *) echo "usage: $0 [stop|stop-all]" >&2; exit 2 ;;
esac

mkdir -p "$DATA"
if listening; then
  echo "server already listening on :$PORT"
elif have_service; then
  systemctl --user start "$SERVICE"
  for _ in $(seq 40); do listening && break; sleep 0.5; done
  echo "server started via systemd ($SERVICE)"
else
  [ -x "$SERVER_BIN" ] || { echo "Not built. Run: cd crates/app && dx build --desktop --release" >&2; exit 1; }
  (cd "$BUILD/web" && KAARYASOOCHI_DATABASE_URL="sqlite://$DATA/kaaryasoochi.db?mode=rwc" IP=$HOST PORT=$PORT \
    setsid nohup "$SERVER_BIN" >"$DATA/server.log" 2>&1 &)
  for _ in $(seq 40); do listening && break; sleep 0.5; done
  echo "server started on $HOST:$PORT (log: $DATA/server.log)"
fi

[ -x "$APP_BIN" ] || { echo "Not built. Run: cd crates/app && dx build --desktop --release" >&2; exit 1; }
# GIO_LAUNCHED_DESKTOP_FILE tells GTK which .desktop file owns this process; GNOME Shell uses
# this to associate the running window with the correct app-grid entry and dock icon.
(cd "$APP_DIR" && GIO_LAUNCHED_DESKTOP_FILE="$HOME/.local/share/applications/kaaryasoochi.desktop" \
  GIO_LAUNCHED_DESKTOP_FILE_PID=$$ setsid nohup "$APP_BIN" >"$DATA/app.log" 2>&1 &)
echo "desktop app started (log: $DATA/app.log)"
