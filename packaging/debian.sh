#!/bin/sh
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Debian/Ubuntu packages from packaging/debian.
#
#   packaging/debian.sh source [SERIES...]   signed source packages for a Launchpad PPA
#                                            (default series: noble resolute)
#   packaging/debian.sh deb                  binary .deb for this machine (needs the Build-Depends)
#
# The source is `git archive` of tag v<version> (HEAD if there is no tag yet, for testing) plus the
# vendor tarball as a second orig component, so Launchpad builds offline. Output: dist/debian/.
#
#   DEBSIGN_KEYID=<key>  sign with this key (default: debuild's default key)
#   UNSIGNED=1           do not sign (local testing)
#   NODEPS=1             skip the Build-Depends check
#   DEBREV=1             Debian revision
#
# Upload each series:  dput ppa:<launchpad-user>/kaaryasoochi dist/debian/*~<series>1_source.changes
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/Cargo.toml" | head -n1)
DEBREV=${DEBREV:-1}
OUT="$ROOT/dist/debian"
VENDOR="$ROOT/dist/kaaryasoochi-$VERSION-vendor.tar.xz"
ORIG="$OUT/kaaryasoochi_$VERSION.orig.tar.xz"

mode=${1:-}
[ $# -gt 0 ] && shift
case "$mode" in
  source | deb) ;;
  *) sed -n '5,19p' "$0" >&2; exit 2 ;;
esac

[ -f "$VENDOR" ] || "$ROOT/packaging/vendor.sh"
mkdir -p "$OUT"

ref="v$VERSION"
if ! git -C "$ROOT" rev-parse -q --verify "refs/tags/$ref" >/dev/null; then
  ref=HEAD
  echo "debian.sh: no tag v$VERSION; packaging HEAD (fine for testing, not for an upload)" >&2
fi
git -C "$ROOT" archive --format=tar --prefix="kaaryasoochi-$VERSION/" "$ref" | xz -T1 -9 >"$ORIG"
cp "$VENDOR" "$OUT/kaaryasoochi_$VERSION.orig-vendor.tar.xz"

# Unpacks the orig tarballs into $1 and adds debian/ from packaging/debian.
prepare() {
  rm -rf "$1"
  mkdir -p "$1"
  tar -xJf "$ORIG" -C "$1" --strip-components=1
  tar -xJf "$VENDOR" -C "$1"
  cp -R "$1/packaging/debian" "$1/debian"
}

sign_opts() {
  if [ "${UNSIGNED:-}" = 1 ]; then
    echo "-us -uc"
  elif [ -n "${DEBSIGN_KEYID:-}" ]; then
    echo "-k$DEBSIGN_KEYID"
  fi
}

deps_opt() { [ "${NODEPS:-}" = 1 ] && echo "-d" || true; }

if [ "$mode" = deb ]; then
  tree="$OUT/build-local"
  prepare "$tree"
  # shellcheck disable=SC2046
  (cd "$tree" && dpkg-buildpackage -b -us -uc $(deps_opt))
  rm -rf "$tree"
  ls "$OUT"/*.deb
  exit 0
fi

[ $# -gt 0 ] || set -- noble resolute
maint=$(sed -n 's/^Maintainer: //p' "$ROOT/packaging/debian/control")
for series in "$@"; do
  tree="$OUT/build-$series"
  prepare "$tree"
  # The top entry becomes this series' build (a newer entry would sort above the PPA version).
  version="$VERSION-$DEBREV~${series}1"
  sed -i -e "1s/^kaaryasoochi ([^)]*) [^;]*;/kaaryasoochi ($version) $series;/" \
    -e "0,/^ -- .*/s// -- $maint  $(date -R)/" "$tree/debian/changelog"
  # -sa: include both orig tarballs; Launchpad needs them with the first upload of a version.
  # shellcheck disable=SC2046
  (cd "$tree" && dpkg-buildpackage -S -sa -d $(sign_opts))
  rm -rf "$tree"
done
ls "$OUT"/*_source.changes
