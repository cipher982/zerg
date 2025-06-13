#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# asyncapi-validate.sh  – lightweight wrapper used by pre-commit.
# ---------------------------------------------------------------------------
# Attempts to validate `asyncapi/chat.yml` using the official AsyncAPI CLI
# (`npx asyncapi validate …`).  If the CLI cannot be fetched (offline CI or
# sandboxed agent environment) the script prints a warning **but exits 0** so
# as not to block commits that do not depend on spec changes.

set -euo pipefail

SPEC="asyncapi/chat.yml"

if [[ ! -f "$SPEC" ]]; then
  echo "❌ $SPEC not found" >&2
  exit 1
fi

# Silently skip when Node is missing.
if ! command -v node >/dev/null 2>&1; then
  echo "⚠️  Node.js not available – skipping AsyncAPI validation." >&2
  exit 0
fi

# Safe tmp creation even in sandboxed envs.
tmp_file() {
  local t
  t=$(mktemp 2>/dev/null) || t=".tmp_asyncapi_validate.log"
  echo "$t"
}

TMP=$(tmp_file)
set +e
# Use new CLI; fall back to legacy if unavailable.
_run_with_timeout() {
  local seconds="$1"; shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "$seconds" "$@"
  else
    "$@"
  fi
}

if ! _run_with_timeout 20 npx --yes @asyncapi/cli validate "$SPEC" >"$TMP" 2>&1; then
  _run_with_timeout 20 npx --yes asyncapi validate "$SPEC" >"$TMP" 2>&1
fi
STATUS=$?
set -e

if [[ $STATUS -eq 0 ]]; then
  echo "✅ AsyncAPI spec valid."
  rm -f "$TMP"
  exit 0
fi

# Detect network-related failure messages and downgrade to warning.
if grep -qE 'ENOTFOUND|network|ETIMEDOUT|could not determine executable|timed out' "$TMP"; then
  echo "⚠️  AsyncAPI CLI unreachable – skipping validation." >&2
  rm -f "$TMP"
  exit 0
fi

# Genuine validation error – show details and fail.
cat "$TMP"
rm -f "$TMP"
exit $STATUS