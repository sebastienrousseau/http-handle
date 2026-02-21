#!/usr/bin/env bash
set -euo pipefail

ADDR="${HTTP_HANDLE_ADDR:-127.0.0.1:8090}"
ROOT_DIR="${HTTP_HANDLE_ROOT:-$(pwd)/target/perf-root}"
MODE="${HTTP_HANDLE_MODE:-high-perf}"
FEATURES="${HTTP_HANDLE_FEATURES:-async,high-perf}"
BOMBARDIER_REQS="${BOMBARDIER_REQS:-20000}"
BOMBARDIER_C="${BOMBARDIER_C:-64}"
MIN_RPS="${MIN_RPS:-1500}"
READY_RETRIES="${READY_RETRIES:-600}"
READY_SLEEP_SECS="${READY_SLEEP_SECS:-0.1}"
PREBUILT_BIN="${PREBUILT_BIN:-target/debug/examples/benchmark_target}"

mkdir -p "${ROOT_DIR}"
python3 - <<'PY'
from pathlib import Path
root = Path("target/perf-root")
root.mkdir(parents=True, exist_ok=True)
(root / "index.html").write_text("x" * 1024)
(root / "large.bin").write_bytes(b"x" * (2 * 1024 * 1024))
(root / "index.html.gz").write_bytes((b"x" * 1024))
(root / "index.html.br").write_bytes((b"x" * 1024))
(root / "index.html.zst").write_bytes((b"x" * 1024))
(root / "404").mkdir(exist_ok=True)
(root / "404/index.html").write_text("404")
PY

export HTTP_HANDLE_ADDR="$ADDR"
export HTTP_HANDLE_ROOT="$ROOT_DIR"
export HTTP_HANDLE_MODE="$MODE"

# Raise file descriptor limit when possible to reduce EMFILE flakiness.
if ulimit -n 65535 >/dev/null 2>&1; then
  :
fi

if [[ -n "${PERF_USE_PREBUILT:-}" && -x "${PREBUILT_BIN}" ]]; then
  env HTTP_HANDLE_ADDR="$ADDR" HTTP_HANDLE_ROOT="$ROOT_DIR" HTTP_HANDLE_MODE="$MODE" \
    "${PREBUILT_BIN}" >/tmp/http_handle_bench.log 2>&1 &
else
  cargo run --example benchmark_target --features "$FEATURES" >/tmp/http_handle_bench.log 2>&1 &
fi
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

for _ in $(seq 1 "${READY_RETRIES}"); do
  if nc -z "${ADDR%:*}" "${ADDR##*:}" >/dev/null 2>&1; then
    break
  fi
  # Exit fast if server process died while bootstrapping.
  if ! kill -0 "${SERVER_PID}" >/dev/null 2>&1; then
    break
  fi
  sleep "${READY_SLEEP_SECS}"
done

if ! nc -z "${ADDR%:*}" "${ADDR##*:}" >/dev/null 2>&1; then
  echo "Benchmark target did not become ready at ${ADDR}"
  echo "Server log:"
  tail -n 200 /tmp/http_handle_bench.log || true
  exit 1
fi

if command -v bombardier >/dev/null 2>&1; then
  OUT=$(bombardier -c "$BOMBARDIER_C" -n "$BOMBARDIER_REQS" "http://$ADDR/")
  echo "$OUT"
  if echo "$OUT" | grep -Eq "connection refused|Errors:.*dial tcp"; then
    echo "Benchmark failed due to connection errors"
    exit 1
  fi
  RPS=$(echo "$OUT" | awk '/Reqs\/sec/ {print int($2)} END {if (NR==0) print ""}')
  if [[ -z "${RPS:-}" ]]; then
    RPS=$(echo "$OUT" | awk '/Throughput:/ {print int($2)}' | head -n1)
  fi
  if [[ -z "${RPS:-}" ]]; then
    echo "Failed to parse bombardier output"
    exit 1
  fi
  if (( RPS < MIN_RPS )); then
    echo "Performance regression: RPS $RPS < threshold $MIN_RPS"
    exit 1
  fi
else
  echo "bombardier not found; skipping RPS assertion"
fi

if command -v wrk >/dev/null 2>&1; then
  wrk -t4 -c128 -d10s "http://$ADDR/" || true
fi

if command -v wrk2 >/dev/null 2>&1; then
  wrk2 -t4 -c128 -d10s -R4000 "http://$ADDR/" || true
fi

echo "Benchmark matrix completed."
