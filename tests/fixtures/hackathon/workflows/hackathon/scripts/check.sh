#!/usr/bin/env bash
# The slice check: the repository's tests, a secret scan of the working tree, and
# ownership of the changed files. Exit 0: pass. Exit 1: fail.
set -uo pipefail

fail=0
if [ -f Cargo.toml ]; then cargo test -q || fail=1; fi
if [ -f package.json ]; then npm test --silent || fail=1; fi
if [ -f go.mod ]; then go test ./... || fail=1; fi
if [ -f pyproject.toml ]; then python3 -m pytest -q || fail=1; fi

if grep -rEi --exclude-dir=.git --exclude-dir=.admiral --exclude-dir=node_modules --exclude-dir=target \
  "(api[_-]?key|secret|token)[\"']? *[:=] *[\"'][A-Za-z0-9_-]{20,}" . >/dev/null 2>&1; then
  echo "check: a secret-looking literal is in the working tree" >&2
  fail=1
fi

owned="${ADMIRAL_PARAM_OWNED_FOLDERS:-}"
if [ -n "$owned" ]; then
  while IFS= read -r file; do
    ok=0
    IFS=',' read -r -a folders <<< "$owned"
    for folder in "${folders[@]}"; do
      case "$file" in "$folder"/*) ok=1 ;; esac
    done
    if [ "$ok" = 0 ]; then
      echo "check: $file is outside the owned folders ($owned)" >&2
      fail=1
    fi
  done < <(git diff --name-only HEAD 2>/dev/null)
fi
exit "$fail"
