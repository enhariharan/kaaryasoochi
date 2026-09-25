#!/usr/bin/env bash
# Creates the `gitea` and `github` remotes (idempotent). No `origin` is created.
set -euo pipefail

OWNER="${REMOTE_OWNER:-$(git config user.name | tr -d ' ' | tr '[:upper:]' '[:lower:]')}"
GITEA_URL="${GITEA_URL:-https://localhost:3000/${GITEA_REPO:-$OWNER/kaaryasoochi}.git}"
GITHUB_URL="${GITHUB_URL:-https://github.com/${GITHUB_REPO:-$OWNER/kaaryasoochi}.git}"

set_remote() {
  if git remote get-url "$1" >/dev/null 2>&1; then git remote set-url "$1" "$2"; else git remote add "$1" "$2"; fi
  echo "$1 -> $2"
}
set_remote gitea "$GITEA_URL"
set_remote github "$GITHUB_URL"
