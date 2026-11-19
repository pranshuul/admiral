#!/usr/bin/env bash
# Phase 0 operational steps. Idempotent; read-only against GitHub; --dry-run prints
# what would run and calls nothing.
#
#   release-prereqs  the public Homebrew tap repository exists and the repository
#                    has the HOMEBREW_TAP_TOKEN secret that the release workflow uses.
set -euo pipefail

usage() {
  echo "usage: ops/a0.sh [--dry-run]" >&2
  exit 2
}

dry_run=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry_run=1 ;;
    *) usage ;;
  esac
done

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
gh="${GH:-gh}"

tap="$(sed -n 's/^tap *= *"\(.*\)"/\1/p' "$root/dist-workspace.toml")"
repo="$(sed -n 's|^repository *= *"https://github.com/\(.*\)"|\1|p' "$root/Cargo.toml")"
secret="HOMEBREW_TAP_TOKEN"

if [ -z "$tap" ] || [ -z "$repo" ]; then
  echo "a0: cannot read tap from dist-workspace.toml or repository from Cargo.toml" >&2
  exit 1
fi

echo "step release-prereqs: tap $tap, secret $secret on $repo"

if [ "$dry_run" = 1 ]; then
  echo "would run: $gh repo view $tap"
  echo "would run: $gh secret list --repo $repo"
  exit 0
fi

if ! "$gh" repo view "$tap" >/dev/null 2>&1; then
  echo "a0: Homebrew tap repository $tap (homebrew-tap) does not exist or the token cannot see it" >&2
  exit 1
fi
if ! "$gh" secret list --repo "$repo" 2>/dev/null | grep -q "^$secret\b"; then
  echo "a0: repository secret $secret is not set on $repo" >&2
  exit 1
fi
echo "ok: $tap exists and $secret is set"
