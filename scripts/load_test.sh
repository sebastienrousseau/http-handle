#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle
#
# External-tool load test for the high-perf async server. Drives a fresh
# `benchmark_target` example with `bombardier` (https://github.com/codesenberg/bombardier)
# at sustained 256-connection keep-alive concurrency, captures the
# throughput and latency distribution, and prints a summary.
#
# Pre-conditions:
# - bombardier on $PATH (Homebrew: `brew install bombardier`).
# - cargo build of the workspace must be possible from the project root.
#
# Usage:
#   ./scripts/load_test.sh                # high-perf mode, 30 s, c=256
#   ./scripts/load_test.sh sync 60 128    # sync server, 60 s, 128 conns
#
# Updates docs/PERFORMANCE.md with the latest numbers.

set -euo pipefail
IFS=$'\n\t'

MODE="${1:-high-perf}"
DURATION="${2:-30}"
CONNECTIONS="${3:-256}"

BIN="bombardier"
if ! command -v "$BIN" >/dev/null 2>&1; then
    echo "error: $BIN not found on PATH" >&2
    echo "install with 'brew install bombardier' (macOS) or your distro's package manager." >&2
    exit 1
fi

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

# Probe-bind a free loopback port; the server thread will reuse it.
PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
ADDR="127.0.0.1:${PORT}"

# Document root: a tempdir with a small static asset.
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
echo '<html><body>Test Content</body></html>' > "$ROOT/test.html"
mkdir -p "$ROOT/404"
echo '404' > "$ROOT/404/index.html"

# Build with all relevant features so the mode arg actually resolves.
cargo build --release --example benchmark_target \
    --features 'async,high-perf,http2' >/dev/null

HTTP_HANDLE_ADDR="$ADDR" \
HTTP_HANDLE_ROOT="$ROOT" \
HTTP_HANDLE_MODE="$MODE" \
    cargo run --release --example benchmark_target \
        --features 'async,high-perf,http2' >/tmp/load_test_server.log 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true; rm -rf "$ROOT"' EXIT

# Wait for the server to bind.
for _ in $(seq 1 100); do
    if curl -sf "http://${ADDR}/test.html" >/dev/null; then
        break
    fi
    sleep 0.05
done

if ! curl -sf "http://${ADDR}/test.html" >/dev/null; then
    echo "error: server never bound on ${ADDR}" >&2
    cat /tmp/load_test_server.log >&2
    exit 1
fi

echo "▶ benchmark_target running on ${ADDR} (mode=${MODE})"
echo "▶ ${BIN} -c ${CONNECTIONS} -d ${DURATION}s -l http://${ADDR}/test.html"
echo

"$BIN" -c "$CONNECTIONS" -d "${DURATION}s" -l "http://${ADDR}/test.html"

echo
echo "✓ load test complete; server log: /tmp/load_test_server.log"
