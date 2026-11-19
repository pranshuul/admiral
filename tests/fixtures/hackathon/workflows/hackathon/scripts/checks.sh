#!/usr/bin/env bash
# Detect the stack of the repository in the working directory and check that the
# kickoff context is complete. Prints one stack name per line.
# Exit 0: a stack was found. Exit 1: none was, so kickoff asks again.
set -euo pipefail

found=0
detect() {
  if [ -e "$1" ]; then
    echo "$2"
    found=1
  fi
}
detect Cargo.toml rust
detect pyproject.toml python
detect package.json node
detect go.mod go

if [ "$found" = 0 ]; then
  echo "checks: no known stack manifest in $(pwd)" >&2
  exit 1
fi
if [ -n "${ADMIRAL_ARTIFACTS:-}" ] && [ -f "$ADMIRAL_ARTIFACTS/context.json" ]; then
  python3 -c 'import json,sys; c=json.load(open(sys.argv[1])); sys.exit(0 if c.get("problem") else 1)' \
    "$ADMIRAL_ARTIFACTS/context.json" || { echo "checks: context has no problem statement" >&2; exit 1; }
fi
exit 0
