#!/usr/bin/env bash
#
# Regenerate openapi.json and ui/src/lib/api-types.ts from the running daemon
# or by building and briefly starting it.
#
# Usage:
#   ./scripts/sync-openapi.sh          # auto-detect running daemon or start one
#   ./scripts/sync-openapi.sh --check   # just verify files are up to date (for CI/hooks)
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OPENAPI_JSON="$ROOT/openapi.json"
API_TYPES="$ROOT/ui/src/lib/api-types.ts"
CHECK_ONLY=false
DAEMON_PID=""

if [[ "${1:-}" == "--check" ]]; then
    CHECK_ONLY=true
fi

cleanup() {
    if [[ -n "$DAEMON_PID" ]]; then
        kill "$DAEMON_PID" 2>/dev/null || true
        wait "$DAEMON_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# ── Find or start the daemon ──────────────────────────────────────────

find_daemon_port() {
    # Try common ports
    for port in 3000 3030; do
        if curl -sf "http://localhost:$port/api/openapi.json" >/dev/null 2>&1; then
            echo "$port"
            return 0
        fi
    done
    return 1
}

PORT=""
if PORT=$(find_daemon_port); then
    echo "Found running daemon on port $PORT"
else
    echo "No running daemon found. Building and starting one..."
    cargo build -p daemon --quiet 2>&1
    
    # Start daemon on a random-ish port to avoid conflicts
    PORT=13579
    "$ROOT/target/debug/daemon" --api-addr "127.0.0.1:$PORT" --foreground &
    DAEMON_PID=$!
    
    # Wait for it to be ready (up to 10s)
    for i in $(seq 1 40); do
        if curl -sf "http://localhost:$PORT/api/health" >/dev/null 2>&1; then
            echo "Daemon started on port $PORT (pid $DAEMON_PID)"
            break
        fi
        if [[ $i -eq 40 ]]; then
            echo "ERROR: Daemon failed to start within 10s" >&2
            exit 1
        fi
        sleep 0.25
    done
fi

BASE_URL="http://localhost:$PORT"

# ── Fetch the live spec ───────────────────────────────────────────────

LIVE_SPEC=$(curl -sf "$BASE_URL/api/openapi.json")
if [[ -z "$LIVE_SPEC" ]]; then
    echo "ERROR: Failed to fetch OpenAPI spec from $BASE_URL" >&2
    exit 1
fi

# Normalize JSON (sort keys, consistent formatting) for stable diffs
LIVE_SPEC_NORMALIZED=$(echo "$LIVE_SPEC" | python3 -c "import sys,json; json.dump(json.load(sys.stdin),sys.stdout,sort_keys=True)" 2>/dev/null || echo "$LIVE_SPEC")

# ── Check or update openapi.json ─────────────────────────────────────

if [[ -f "$OPENAPI_JSON" ]]; then
    EXISTING_NORMALIZED=$(python3 -c "import sys,json; json.dump(json.load(open(sys.argv[1])),sys.stdout,sort_keys=True)" "$OPENAPI_JSON" 2>/dev/null || cat "$OPENAPI_JSON")
else
    EXISTING_NORMALIZED=""
fi

SPEC_CHANGED=false
if [[ "$LIVE_SPEC_NORMALIZED" != "$EXISTING_NORMALIZED" ]]; then
    SPEC_CHANGED=true
    if $CHECK_ONLY; then
        echo "STALE: openapi.json differs from live spec"
    else
        echo "$LIVE_SPEC_NORMALIZED" > "$OPENAPI_JSON"
        echo "Updated openapi.json"
    fi
else
    echo "openapi.json is up to date"
fi

# ── Regenerate TypeScript types ───────────────────────────────────────

# Save current api-types.ts for comparison
if [[ -f "$API_TYPES" ]]; then
    EXISTING_TYPES=$(cat "$API_TYPES")
else
    EXISTING_TYPES=""
fi

# Generate into a temp file
TEMP_TYPES=$(mktemp)
cd "$ROOT/ui"
npx openapi-typescript "$BASE_URL/api/openapi.json" -o "$TEMP_TYPES" 2>/dev/null
NEW_TYPES=$(cat "$TEMP_TYPES")
rm -f "$TEMP_TYPES"

TYPES_CHANGED=false
if [[ "$NEW_TYPES" != "$EXISTING_TYPES" ]]; then
    TYPES_CHANGED=true
    if $CHECK_ONLY; then
        echo "STALE: ui/src/lib/api-types.ts differs from generated output"
    else
        echo "$NEW_TYPES" > "$API_TYPES"
        echo "Updated ui/src/lib/api-types.ts"
    fi
else
    echo "api-types.ts is up to date"
fi

# ── Summary ───────────────────────────────────────────────────────────

if $CHECK_ONLY; then
    if $SPEC_CHANGED || $TYPES_CHANGED; then
        echo ""
        echo "OpenAPI types are out of date. Run: ./scripts/sync-openapi.sh"
        exit 1
    else
        echo "All OpenAPI files are up to date."
        exit 0
    fi
else
    if $SPEC_CHANGED || $TYPES_CHANGED; then
        echo ""
        echo "Files updated. Don't forget to stage them:"
        $SPEC_CHANGED && echo "  git add openapi.json"
        $TYPES_CHANGED && echo "  git add ui/src/lib/api-types.ts"
    fi
fi
