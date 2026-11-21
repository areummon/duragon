#!/usr/bin/env bash
# Stops only what launch.sh started and closes this run's browser session. Evidence stays in $EVIDENCE.
source "$(dirname "$0")/env.sh"
agent-browser close >/dev/null 2>&1 || true
pid="$(cat "$VERIFY_DIR/vite.pid" 2>/dev/null || true)"
if [ -n "$pid" ]; then kill -- "-$pid" 2>/dev/null || kill "$pid" 2>/dev/null || true; fi
rm -f "$VERIFY_DIR/vite.pid"
for _ in $(seq 1 20); do ss -ltnH "sport = :$PORT" | grep -q LISTEN || break; sleep 0.25; done
if ss -ltnH "sport = :$PORT" | grep -q LISTEN; then echo "port $PORT still open" >&2; exit 1; fi
echo "stopped; port $PORT closed; evidence kept in $EVIDENCE"
