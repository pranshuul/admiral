#!/usr/bin/env bash
# Check the demo is submittable: a clean clone has a README and the backup video,
# and no secret-looking literal appears anywhere in history.
# Exit 0: pass. Exit 1: fail.
set -euo pipefail

fail=0
[ -f README.md ] || { echo "demo-check: README.md is missing" >&2; fail=1; }
[ -n "$(find . -maxdepth 2 -name 'demo.*' \( -name '*.mp4' -o -name '*.webm' -o -name '*.gif' \) -print -quit)" ] \
  || { echo "demo-check: no backup video (demo.mp4, demo.webm or demo.gif)" >&2; fail=1; }
if git rev-parse --git-dir >/dev/null 2>&1; then
  if git log -p --all 2>/dev/null | grep -qEi "(api[_-]?key|secret|token)[\"']? *[:=] *[\"'][A-Za-z0-9_-]{20,}"; then
    echo "demo-check: a secret-looking literal appears in history" >&2
    fail=1
  fi
fi
exit "$fail"
