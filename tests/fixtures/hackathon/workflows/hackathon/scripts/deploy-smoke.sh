#!/usr/bin/env bash
# Smoke-test the deployed demo: its URL must answer with a success status.
# Exit 0: pass. Exit 1: fail or no URL.
set -euo pipefail

url="${ADMIRAL_PARAM_DEPLOY_URL:-}"
if [ -z "$url" ]; then
  echo "deploy-smoke: ADMIRAL_PARAM_DEPLOY_URL is not set" >&2
  exit 1
fi
code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 20 "$url" || true)"
case "$code" in
  2??) echo "deploy-smoke: $url answered $code" ;;
  *) echo "deploy-smoke: $url answered ${code:-nothing}" >&2; exit 1 ;;
esac
