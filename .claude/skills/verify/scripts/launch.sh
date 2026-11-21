#!/usr/bin/env bash
# Starts a Vite dev server for the frontend on $PORT (default 5199). Outside Tauri the app uses the
# in-memory mock backend (src/lib/api/mock.ts), seeded fresh on every page load.
set -euo pipefail
source "$(dirname "$0")/env.sh"
mkdir -p "$VERIFY_DIR" "$EVIDENCE"
if [ -f "$VERIFY_DIR/vite.pid" ] && kill -0 "$(cat "$VERIFY_DIR/vite.pid")" 2>/dev/null; then
  echo "already running: pid $(cat "$VERIFY_DIR/vite.pid") on :$PORT"; exit 0
fi
if ss -ltn "sport = :$PORT" | grep -q LISTEN; then echo "port $PORT is taken by something this run did not start" >&2; exit 1; fi
[ -d "$ROOT/node_modules" ] || (cd "$ROOT" && npm ci)
cd "$ROOT"
setsid npx vite --port "$PORT" --strictPort >"$VERIFY_DIR/vite.log" 2>&1 < /dev/null &
echo $! > "$VERIFY_DIR/vite.pid"
for _ in $(seq 1 60); do
  if curl -fsS "$URL/src/main.ts" >/dev/null 2>&1; then echo "ready: $URL (pid $(cat "$VERIFY_DIR/vite.pid"))"; exit 0; fi
  sleep 0.5
done
echo "vite did not come up; see $VERIFY_DIR/vite.log" >&2; exit 1
