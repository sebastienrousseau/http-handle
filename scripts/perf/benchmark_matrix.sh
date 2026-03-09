# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Sebastien Rousseau

#!/usr/bin/env bash
set -euo pipefail

ADDR="${HTTP_HANDLE_ADDR:-127.0.0.1:8090}"
ROOT_DIR="${HTTP_HANDLE_ROOT:-$(pwd)/target/perf-root}"
MODE="${HTTP_HANDLE_MODE:-high-perf}"
FEATURES="${HTTP_HANDLE_FEATURES:-async,high-perf}"
BOMBARDIER_REQS="${BOMBARDIER_REQS:-20000}"
BOMBARDIER_C="${BOMBARDIER_C:-64}"
MIN_RPS="${MIN_RPS:-1500}"
MIN_RPS_PER_CORE="${MIN_RPS_PER_CORE:-300}"
MAX_P99_MS="${MAX_P99_MS:-100}"
READY_RETRIES="${READY_RETRIES:-600}"
READY_SLEEP_SECS="${READY_SLEEP_SECS:-0.1}"
PREBUILT_BIN="${PREBUILT_BIN:-target/debug/examples/benchmark_target}"
BASELINE_FILE="${PERF_BASELINE_FILE:-scripts/perf/baseline.json}"
BASELINE_DIR="${PERF_BASELINE_DIR:-scripts/perf}"
RESULT_JSON="${PERF_RESULT_JSON:-target/perf-result.json}"

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

if [[ -z "${PERF_BASELINE_FILE:-}" && -f "Cargo.toml" ]]; then
  VERSION="$(awk -F '"' '/^version = / {print $2; exit}' Cargo.toml)"
  if [[ -n "${VERSION:-}" && -f "${BASELINE_DIR}/baseline-v${VERSION}.json" ]]; then
    BASELINE_FILE="${BASELINE_DIR}/baseline-v${VERSION}.json"
    echo "Using versioned perf baseline: ${BASELINE_FILE}"
  fi
fi

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
  P99_MS=$(echo "$OUT" | awk '/99th/ {gsub("ms","",$2); print $2; exit}')
  if [[ -z "${P99_MS:-}" ]]; then
    P99_MS=$(echo "$OUT" | awk '/99\.000%/ {gsub("ms","",$2); print $2; exit}')
  fi
  if [[ -z "${P99_MS:-}" ]]; then
    P99_MS="0"
  fi
  if command -v nproc >/dev/null 2>&1; then
    CORE_COUNT=$(nproc)
  else
    CORE_COUNT=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)
  fi
  if [[ -z "${CORE_COUNT:-}" || "${CORE_COUNT}" -lt 1 ]]; then
    CORE_COUNT=1
  fi
  RPS_PER_CORE=$(awk -v rps="$RPS" -v c="$CORE_COUNT" 'BEGIN { printf "%.2f", rps/c }')

  mkdir -p "$(dirname "$RESULT_JSON")"
  cat >"$RESULT_JSON" <<JSON
{"rps": $RPS, "p99_latency_ms": $P99_MS, "cores": $CORE_COUNT, "rps_per_core": $RPS_PER_CORE}
JSON
  cat "$RESULT_JSON"

  if [[ -f "$BASELINE_FILE" ]]; then
    MIN_RPS=$(jq -r '.rps_min' "$BASELINE_FILE")
    MIN_RPS_PER_CORE=$(jq -r '.rps_per_core_min' "$BASELINE_FILE")
    MAX_P99_MS=$(jq -r '.p99_latency_ms_max' "$BASELINE_FILE")
  fi

  if (( RPS < MIN_RPS )); then
    echo "Performance regression: RPS $RPS < threshold $MIN_RPS"
    exit 1
  fi
  awk -v v="$RPS_PER_CORE" -v m="$MIN_RPS_PER_CORE" 'BEGIN { if (v < m) { exit 1 } }' || {
    echo "Performance regression: RPS/core $RPS_PER_CORE < threshold $MIN_RPS_PER_CORE"
    exit 1
  }
  awk -v v="$P99_MS" -v m="$MAX_P99_MS" 'BEGIN { if (v > m) { exit 1 } }' || {
    echo "Performance regression: p99 ${P99_MS}ms > threshold ${MAX_P99_MS}ms"
    exit 1
  }
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
