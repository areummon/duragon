# Sourced by the other scripts. Override PORT or VERIFY_DIR in the environment.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
PORT="${PORT:-5199}"
VERIFY_DIR="${VERIFY_DIR:-${TMPDIR:-/tmp}/duragon-verify}"
EVIDENCE="$VERIFY_DIR/evidence"
URL="http://localhost:$PORT"
export AGENT_BROWSER_SESSION="${AGENT_BROWSER_SESSION:-duragon-verify}"
if [ "$PORT" = 1420 ]; then echo "refusing port 1420: it belongs to the user's cargo tauri dev" >&2; exit 2; fi
