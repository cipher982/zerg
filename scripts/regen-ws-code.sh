#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# regen-ws-code.sh  – regenerate WebSocket contract artefacts from AsyncAPI.
# ---------------------------------------------------------------------------
# This script is invoked by `make regen-ws-code` **and** by CI to ensure that
# the generated code is always in sync with `asyncapi/chat.yml`.
#
# Prerequisites (not installed automatically):
#   • Node ≥ 16 (for npx)
#   • @asyncapi/generator  (fetched transparently via npx)
#   • @asyncapi/typescript-codegen  (ditto)
#
# Nothing here starts the backend server; it is safe in CI and pre-commit.
set -euo pipefail

SPEC_FILE="$(git rev-parse --show-toplevel)/asyncapi/chat.yml"

# Ensure we run from repo root regardless of cwd.
cd "$(git rev-parse --show-toplevel)"

if [[ ! -f "$SPEC_FILE" ]]; then
  echo "❌ AsyncAPI spec not found at $SPEC_FILE" >&2
  exit 1
fi

echo "🔄 Validating AsyncAPI spec…"
# Try validation with npx.  If the network is unavailable (common in CI
# sandboxes) we emit a warning but continue so the task is non-blocking.
n_tmp() {
  local tmp
  tmp=$(mktemp 2>/dev/null) || tmp=".tmp_asyncapi_validate.log"
  echo "$tmp"
}

set +e
VALID_OUT=$(n_tmp)
npx --yes asyncapi validate "$SPEC_FILE" >"$VALID_OUT" 2>&1
VALID_EXIT=$?
set -e

if [[ $VALID_EXIT -ne 0 ]]; then
  if grep -qE 'ENOTFOUND|network|could not determine executable' "$VALID_OUT"; then
    echo "⚠️  AsyncAPI CLI unreachable (likely offline) – skipping validation & codegen."
    rm -f "$VALID_OUT"
    exit 0
  else
    cat "$VALID_OUT"
    rm -f "$VALID_OUT"
    exit $VALID_EXIT
  fi
fi
rm -f "$VALID_OUT"

echo "🛠  Generating Rust (backend) types…"
# Output to a temporary dir then move (safer than deleting first).
BACKEND_OUT="backend/zerg/ws_schema"
rm -rf "$BACKEND_OUT.tmp" && mkdir -p "$BACKEND_OUT.tmp"
npx --yes @asyncapi/generator "$SPEC_FILE" asyncapi-rust -o "$BACKEND_OUT.tmp"
rm -rf "$BACKEND_OUT"
mv "$BACKEND_OUT.tmp" "$BACKEND_OUT"

echo "🛠  Generating TypeScript (frontend) types…"
FRONTEND_OUT="frontend/generated"
rm -rf "$FRONTEND_OUT.tmp" && mkdir -p "$FRONTEND_OUT.tmp"
npx --yes @asyncapi/typescript-codegen --input "$SPEC_FILE" --output "$FRONTEND_OUT.tmp"
rm -rf "$FRONTEND_OUT"
mv "$FRONTEND_OUT.tmp" "$FRONTEND_OUT"

echo "✅ WebSocket code regenerated.  Remember to commit changes if any."
