#!/usr/bin/env bash
# Merge the slice branch into main once green, then smoke-test the redeployed build.
# Exit 0: merged. Exit 1: merge conflict. Exit 2: smoke test failed.
set -uo pipefail

branch="${ADMIRAL_PARAM_SLICE_BRANCH:?ADMIRAL_PARAM_SLICE_BRANCH is not set}"
git checkout -q main || exit 1
if ! git merge --no-ff -q -m "Merge $branch" "$branch"; then
  git merge --abort
  exit 1
fi
if [ -n "${ADMIRAL_PARAM_DEPLOY_URL:-}" ]; then
  "$(dirname "$0")/deploy-smoke.sh" || exit 2
fi
exit 0
