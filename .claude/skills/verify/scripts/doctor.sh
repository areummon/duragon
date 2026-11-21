#!/usr/bin/env bash
# Read-only: is the instance this run started healthy, and is the user's own session up?
source "$(dirname "$0")/env.sh"
ok=1
pid="$(cat "$VERIFY_DIR/vite.pid" 2>/dev/null || true)"
if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then echo "vite: pid $pid alive"; else echo "vite: not running (run launch.sh)"; ok=0; fi
owner="$(ss -ltnpH "sport = :$PORT" 2>/dev/null | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2)"
if [ -n "$owner" ] && [ -n "$pid" ] && { [ "$owner" = "$pid" ] || [ "$(ps -o pgid= -p "$owner" | tr -d ' ')" = "$pid" ]; }; then
  echo "port $PORT: owned by this run"
else echo "port $PORT: not owned by this run (owner: ${owner:-none})"; ok=0; fi
if curl -fsS "$URL/" | grep -q '<div id="app"\|src/main.ts'; then echo "page: serves the Duragon frontend"; else echo "page: unexpected response"; ok=0; fi
echo "build: $(git -C "$ROOT" rev-parse --short HEAD)$(git -C "$ROOT" diff --quiet || echo ' (+ uncommitted changes)')"
if ss -ltnH 'sport = :1420' | grep -q LISTEN; then echo "user session: cargo tauri dev is up on :1420; do not touch it"; else echo "user session: not running"; fi
[ $ok = 1 ] && echo "doctor: OK" || { echo "doctor: NOT OK"; exit 1; }
