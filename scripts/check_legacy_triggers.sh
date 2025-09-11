#!/usr/bin/env bash
set -euo pipefail

echo "🔍 Scanning for legacy flattened trigger key usage (frontend/src only)…"

# 1) String-literal usage of legacy flattened key "trigger_type"
# Use a non-failing pipeline under strict mode
set +o pipefail
MATCHES=$(rg -n --color never -e '"trigger_type"' frontend/src || true)
COUNT=$(printf "%s" "$MATCHES" | rg -v 'tests|generated' | wc -l | tr -d ' \n')
set -o pipefail
if [ "$COUNT" -gt 0 ]; then
  echo "❌ Found legacy trigger_type usage in frontend/src. Use typed config.trigger instead."
  rg -n --color never -e '"trigger_type"' frontend/src | rg -v 'tests|generated' || true
  exit 1
fi

# 2) Writes to dynamic_props with trigger_* keys
set +o pipefail
MATCHES2=$(rg -n --color never -e 'dynamic_props\.insert\("trigger_' frontend/src || true)
COUNT2=$(printf "%s" "$MATCHES2" | rg -v 'tests|generated' | wc -l | tr -d ' \n')
set -o pipefail
if [ "$COUNT2" -gt 0 ]; then
  echo "❌ Found writes to dynamic_props with trigger_* keys. Typed meta must be canonical."
  rg -n --color never -e 'dynamic_props\.insert\("trigger_' frontend/src | rg -v 'tests|generated' || true
  exit 1
fi

echo "✅ No legacy trigger key patterns detected in frontend/src"
