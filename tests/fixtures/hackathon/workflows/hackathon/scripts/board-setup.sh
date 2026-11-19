#!/usr/bin/env bash
# Set up the board: always the local log; in team mode also a board.toml with the
# members and a join command. Exit 0: ready. Exit 2: team mode, not every member joined.
set -euo pipefail

mode="${ADMIRAL_PARAM_MODE:-solo}"
dir="${ADMIRAL_RUN_DIR:?ADMIRAL_RUN_DIR is not set}"
mkdir -p "$dir/board"
: >> "$dir/board/local.jsonl"

if [ "$mode" = "solo" ]; then
  echo "board: local log at $dir/board/local.jsonl"
  exit 0
fi

team="${ADMIRAL_PARAM_TEAM:-}"
{
  echo "members = ["
  IFS=',' read -r -a logins <<< "$team"
  for login in "${logins[@]}"; do
    [ -n "$login" ] && echo "  \"$login\","
  done
  echo "]"
} > "$dir/board/board.toml"
echo "board: join with: admiral run hackathon --join $dir/board"
joined=0
[ -d "$dir/board/members" ] && joined="$(find "$dir/board/members" -type f | wc -l)"
expected="$(grep -c '^  "' "$dir/board/board.toml" || true)"
if [ "$joined" -ge "$expected" ]; then
  exit 0
fi
exit 2
