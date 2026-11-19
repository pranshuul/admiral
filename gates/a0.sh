#!/usr/bin/env bash
# Phase 0 gate. Prints PASS <n> or FAIL <n> <reason> per gate bullet and exits 0 only if no
# bullet failed. Every bullet of this phase needs no person, so none is skipped.
#
#   1  CI is green on all three targets: the CI definition runs every check on Linux x86_64,
#      Linux aarch64 and macOS aarch64, and every one of those checks passes here.
#   2  The harness drives the two-node fixture workflow through stubbed next and advance.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root" || exit 2

failed=0
log="$(mktemp)"
work="$(mktemp -d)"
trap 'rm -rf "$log" "$work"' EXIT

report() { # report <n> <status> [reason]
  if [ "$2" = pass ]; then
    echo "PASS $1"
  else
    echo "FAIL $1 $3"
    failed=1
  fi
}

# Runs a command quietly; on failure, names it in the reason.
step() {
  if ! "$@" >"$log" 2>&1; then
    reason="$* failed: $(tail -n 1 "$log")"
    return 1
  fi
}

bullet1() {
  step cargo test -p xtask --test ci || return 1
  step cargo build || return 1
  step cargo clippy --all-targets -- -D warnings || return 1
  step cargo fmt --check || return 1
  step cargo test || return 1
  step cargo xtask schemas-check || return 1
}

bullet2() {
  local out
  if ! out="$(cargo run -q -p admiral-harness -- \
    "$root/tests/harness/scenarios/two-node.json" \
    --admiral "$root/tests/harness/fixtures/stub-admiral.sh" \
    --dir "$work" 2>"$log")"; then
    reason="the harness failed: ${out:-$(tail -n 1 "$log")}"
    return 1
  fi
  if [ "$out" != "PASS two-node" ]; then
    reason="the harness reported '$out' instead of 'PASS two-node'"
    return 1
  fi
}

reason=""
if bullet1; then report 1 pass; else report 1 fail "$reason"; fi
reason=""
if bullet2; then report 2 pass; else report 2 fail "$reason"; fi

exit "$failed"
