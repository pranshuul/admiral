#!/usr/bin/env bash
# Write receipts.json and receipts.md into the artifacts directory, stamped with the
# hash of the source tree at HEAD.
set -euo pipefail

out="${ADMIRAL_ARTIFACTS:?ADMIRAL_ARTIFACTS is not set}"
mkdir -p "$out"
hash="$(git rev-parse 'HEAD^{tree}' 2>/dev/null || echo none)"
printf '{\n  "source_hash": "%s"\n}\n' "$hash" > "$out/receipts.json"
printf '# Receipts\n\nSource tree: `%s`\n' "$hash" > "$out/receipts.md"
echo "scoreboard: stamped with $hash"
