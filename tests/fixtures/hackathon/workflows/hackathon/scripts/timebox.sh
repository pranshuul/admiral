#!/usr/bin/env bash
# Read the run's budget state. Exit 0: time remains. Exit 2: the deadline budget is
# spent. Exit 3: the per-slice round budget is spent.
set -euo pipefail

state="${ADMIRAL_RUN_DIR:?ADMIRAL_RUN_DIR is not set}/run.json"
[ -f "$state" ] || exit 0
python3 - "$state" <<'PY'
import json, sys
budgets = json.load(open(sys.argv[1])).get("budgets", {})
if budgets.get("deadline", {}).get("spent"):
    sys.exit(2)
if budgets.get("rounds", {}).get("spent"):
    sys.exit(3)
PY
