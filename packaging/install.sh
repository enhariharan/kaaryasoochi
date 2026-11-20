#!/bin/sh
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Installs a packaging/build.sh build into DESTDIR with the layout every package uses.
#
#   packaging/install.sh DESTDIR [PREFIX]      (PREFIX defaults to /usr)
#
#   PREFIX/bin/kaaryasoochi                    launcher (starts the user service, opens the UI)
#   PREFIX/bin/kaaryasoochi-server             server wrapper, run by the user unit
#   PREFIX/bin/kaaryasoochi-admin              operator tool (password reset, accounts)
#   PREFIX/lib/kaaryasoochi/app/               desktop window + its assets (when built)
#   PREFIX/lib/kaaryasoochi/web/               server binary + public/
#   PREFIX/lib/systemd/user/kaaryasoochi-server.service
#   PREFIX/share/{applications,icons,metainfo}/...
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DESTDIR=${1:?usage: install.sh DESTDIR [PREFIX]}
PREFIX=${2:-/usr}
REL="$ROOT/target/dx/kaaryasoochi-app/release"
COMMON="$ROOT/packaging/common"
APPID=io.github.enhariharan.Kaaryasoochi
D="$DESTDIR$PREFIX"

[ -x "$REL/web/server" ] || { echo "install.sh: no build in $REL; run packaging/build.sh" >&2; exit 1; }

install -d "$D/bin" "$D/lib/kaaryasoochi/web"
install -m755 "$REL/web/server" "$D/lib/kaaryasoochi/web/server"
cp -R "$REL/web/public" "$D/lib/kaaryasoochi/web/"
if [ -x "$REL/linux/app/kaaryasoochi-app" ]; then
  install -d "$D/lib/kaaryasoochi/app"
  cp -R "$REL/linux/app/." "$D/lib/kaaryasoochi/app/"
  # The desktop build puts images only next to the window; the server serves them too.
  install -d "$D/lib/kaaryasoochi/web/public/assets"
  cp -R "$REL/linux/app/assets/." "$D/lib/kaaryasoochi/web/public/assets/"
fi
chmod -R u+rwX,go+rX,go-w "$D/lib/kaaryasoochi"

install -m755 "$COMMON/kaaryasoochi" "$D/bin/kaaryasoochi"
install -m755 "$COMMON/kaaryasoochi-server" "$D/bin/kaaryasoochi-server"
install -m755 "$ROOT/target/release/kaaryasoochi-admin" "$D/bin/kaaryasoochi-admin"

install -d "$D/lib/systemd/user"
sed "s|^ExecStart=/usr/bin/|ExecStart=$PREFIX/bin/|" "$COMMON/kaaryasoochi-server.service" \
  >"$D/lib/systemd/user/kaaryasoochi-server.service"
chmod 644 "$D/lib/systemd/user/kaaryasoochi-server.service"

install -Dm644 "$COMMON/kaaryasoochi.desktop" "$D/share/applications/kaaryasoochi.desktop"
install -Dm644 "$COMMON/$APPID.metainfo.xml" "$D/share/metainfo/$APPID.metainfo.xml"
ICONS="$ROOT/crates/app/assets"
install -Dm644 "$ICONS/icon.svg" "$D/share/icons/hicolor/scalable/apps/kaaryasoochi.svg"
for size in 32 48 64 128 256 512; do
  install -Dm644 "$ICONS/icons/$size.png" "$D/share/icons/hicolor/${size}x$size/apps/kaaryasoochi.png"
done
