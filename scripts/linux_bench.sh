#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle
#
# In-container bombardier driver for Linux numbers in PERFORMANCE.md.
# Designed to be invoked from the host via:
#
#   podman run --rm --platform linux/arm64 \
#       -v "$(pwd):/work" -w /work \
#       docker.io/library/rust:1.88-slim \
#       bash scripts/linux_bench.sh
#
# Builds the `bench` example with the high-perf feature set, then runs
# `bombardier` against three modes (sync, high-perf, high-perf-mt) for
# 30 s at 256-conn keep-alive each, and prints the result table.

set -euo pipefail
IFS=$'\n\t'

export PATH=/usr/local/cargo/bin:${PATH}
export CARGO_HOME=${CARGO_HOME:-/work/.cargo-linux}
export CARGO_TARGET_DIR=/tmp/target-linux

apt-get update -qq
apt-get install -y -qq curl ca-certificates python3 >/dev/null

ARCH="$(uname -m)"
case "${ARCH}" in
    aarch64) BOMB_BIN=bombardier-linux-arm64 ;;
    x86_64)  BOMB_BIN=bombardier-linux-amd64 ;;
    *) echo "unsupported arch ${ARCH}" >&2; exit 1 ;;
esac
BOMB=/usr/local/bin/bombardier
if [[ ! -x ${BOMB} ]]; then
    curl -sSL "https://github.com/codesenberg/bombardier/releases/download/v1.2.6/${BOMB_BIN}" -o "${BOMB}"
    chmod +x "${BOMB}"
fi
"${BOMB}" --version 2>&1 | head -1

cargo build --release --example bench \
    --features 'async,high-perf,high-perf-multi-thread,http2' >/dev/null

ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
echo '<html><body>Test Content</body></html>' > "$ROOT/test.html"
mkdir -p "$ROOT/404"
echo '404' > "$ROOT/404/index.html"

run_one() {
    local mode=$1
    local port
    port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
    local addr="127.0.0.1:${port}"

    HTTP_HANDLE_ADDR="$addr" \
    HTTP_HANDLE_ROOT="$ROOT" \
    HTTP_HANDLE_MODE="$mode" \
        "${CARGO_TARGET_DIR}/release/examples/bench" \
        > "/tmp/server-${mode}.log" 2>&1 &
    local pid=$!

    for _ in $(seq 1 200); do
        if curl -sf "http://${addr}/test.html" >/dev/null; then break; fi
        sleep 0.05
    done

    if ! curl -sf "http://${addr}/test.html" >/dev/null; then
        echo "error: ${mode} failed to bind on ${addr}" >&2
        cat "/tmp/server-${mode}.log" >&2
        kill -9 "$pid" 2>/dev/null || true
        return 1
    fi

    echo "▶ ${mode} (linux/${ARCH})"
    "${BOMB}" -c 256 -d 30s -l "http://${addr}/test.html" 2>&1 \
        | grep -E "Reqs/sec|50%|75%|90%|99%|Throughput|HTTP codes" \
        | sed 's/^/  /'
    kill -9 "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    echo
}

echo
echo "=== Linux bombardier numbers (arch=${ARCH}, kernel=$(uname -r)) ==="
echo
run_one sync
run_one high-perf
run_one high-perf-mt
echo "=== complete ==="
