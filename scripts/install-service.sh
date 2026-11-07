#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Installs the release build as systemd *user* services and starts the server.
#
#   (cd crates/app && dx build --desktop --release)   # build first
#   scripts/install-service.sh [--wait-for HOST:PORT[,HOST:PORT...]] [--with-app]
#   scripts/install-service.sh --uninstall
#
# The build is copied to ~/.local/share/kaaryasoochi/app, so rebuilding never touches the files a
# running service is using. Re-run this script after each rebuild to deploy it.
#
#   --wait-for   hold the scheduler until these endpoints accept connections (e.g. 127.0.0.1:5432
#                for a database the jobs need), so a catch-up run right after boot does not fail
#                because a dependency is still starting. Written as a drop-in; re-running without
#                the flag keeps the existing value.
#   --with-app   also enable the desktop app to open at each graphical login (off by default).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REL="$ROOT/target/dx/kaaryasoochi-app/release"
DEST="$HOME/.local/share/kaaryasoochi/app"
UNITS="$HOME/.config/systemd/user"
SERVER=kaaryasoochi-server.service
APP=kaaryasoochi-app.service
PORT="${PORT:-8080}"

wait_for="" with_app=0 uninstall=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --wait-for) wait_for="${2:?--wait-for needs HOST:PORT}"; shift 2 ;;
    --with-app) with_app=1; shift ;;
    --uninstall) uninstall=1; shift ;;
    -h|--help) sed -n '5,22p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

if [[ $uninstall -eq 1 ]]; then
  systemctl --user disable --now "$APP" "$SERVER" 2>/dev/null || true
  rm -f "$UNITS/$SERVER" "$UNITS/$APP"; rm -rf "$UNITS/$SERVER.d"
  systemctl --user daemon-reload
  rm -f "$HOME/.local/share/applications/kaaryasoochi.desktop"
  rm -f "$HOME/.local/share/icons/hicolor/scalable/apps/kaaryasoochi.svg"
  for sz in 32 48 64 128 256 512; do
    rm -f "$HOME/.local/share/icons/hicolor/${sz}x${sz}/apps/kaaryasoochi.png"
  done
  gtk-update-icon-cache -qtf "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
  echo "services and desktop integration removed (build copy and database left in ~/.local/share/kaaryasoochi)"
  exit 0
fi

[[ -x "$REL/web/server" && -x "$REL/linux/app/kaaryasoochi-app" ]] || {
  echo "No release build found. Run: cd crates/app && dx build --desktop --release" >&2; exit 1; }

# The files cannot be replaced while running, and a second server would fight over the port.
systemctl --user stop "$SERVER" "$APP" 2>/dev/null || true
if ss -ltn | grep -q ":$PORT "; then
  echo "Port $PORT is in use by something other than the service (a manually started server?)." >&2
  echo "Stop it first: scripts/run-desktop.sh stop" >&2
  exit 1
fi

mkdir -p "$DEST" "$UNITS"
rsync -a --delete "$REL/web/" "$DEST/web/"
rsync -a --delete "$REL/linux/app/" "$DEST/linux/"
# The web server embeds CSS but serves other assets (images) from public/assets/.
# dx build --desktop places asset files only under linux/app/assets/, not web/public/,
# so copy them across so the server can serve them to the WebKit window.
rsync -a "$REL/linux/app/assets/" "$DEST/web/public/assets/"
cp "$ROOT/deploy/systemd/$SERVER" "$ROOT/deploy/systemd/$APP" "$UNITS/"

# Desktop integration: app-grid icon and launcher entry
ICON_SRC="$ROOT/crates/app/assets"
ICON_DEST="$HOME/.local/share/icons/hicolor"
install -Dm644 "$ICON_SRC/icon.svg" "$ICON_DEST/scalable/apps/kaaryasoochi.svg"
for sz in 32 48 64 128 256 512; do
  install -Dm644 "$ICON_SRC/icons/${sz}.png" "$ICON_DEST/${sz}x${sz}/apps/kaaryasoochi.png"
done
gtk-update-icon-cache -qtf "$ICON_DEST" 2>/dev/null || true
install -Dm644 <(sed "s|INSTALL_DIR|$DEST|g" "$ROOT/deploy/kaaryasoochi.desktop") \
  "$HOME/.local/share/applications/kaaryasoochi.desktop"

if [[ -n "$wait_for" ]]; then
  mkdir -p "$UNITS/$SERVER.d"
  printf '[Service]\nEnvironment=KAARYASOOCHI_WAIT_FOR_TCP=%s\n' "$wait_for" > "$UNITS/$SERVER.d/wait-for.conf"
fi

systemctl --user daemon-reload
systemctl --user enable --now "$SERVER"
[[ $with_app -eq 1 ]] && systemctl --user enable "$APP"

for _ in $(seq 40); do ss -ltn | grep -q ":$PORT " && break; sleep 0.5; done
systemctl --user --no-pager --lines=0 status "$SERVER" | sed -n '1,4p'
echo "linger (start at boot without login): $(loginctl show-user "$USER" -p Linger --value)"
[[ "$(loginctl show-user "$USER" -p Linger --value)" == yes ]] || \
  echo "  -> not enabled: run  sudo loginctl enable-linger $USER  so the server starts at boot" >&2
echo "logs: journalctl --user -u $SERVER -f"
