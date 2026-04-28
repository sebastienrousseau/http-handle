#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle
#
# Run an http-handle example by name, auto-resolving the required
# Cargo features. Equivalent to `cargo run --example <name>` with the
# right `--features` flag attached.
#
# Usage:
#   ./scripts/example.sh <name> [extra cargo args]
#   ./scripts/example.sh enterprise
#   ./scripts/example.sh dhat --release
#
# Listing all known examples:
#   ./scripts/example.sh --list

set -euo pipefail
IFS=$'\n\t'

cd "$(dirname "$0")/.."

# name => feature flag(s). Examples with no entry need no features.
declare_features() {
    case "$1" in
        async)         echo "async" ;;
        batch)         echo "batch" ;;
        streaming)     echo "streaming" ;;
        optimized)     echo "optimized" ;;
        observability) echo "observability" ;;
        http2)         echo "http2" ;;
        http3)         echo "http3-profile" ;;
        perf)          echo "high-perf" ;;
        multi)         echo "high-perf-multi-thread" ;;
        autotune)      echo "autotune" ;;
        ratelimit)     echo "distributed-rate-limit" ;;
        tenant)        echo "multi-tenant" ;;
        tls|auth|config|enterprise) echo "enterprise" ;;
        dhat)          echo "high-perf" ;;
        *)             echo "" ;;
    esac
}

readonly KNOWN_EXAMPLES=(
    # Core
    hello builder request response errors policies pool shutdown keepalive language
    # Per feature
    async batch streaming optimized observability http2 http3
    perf multi autotune ratelimit tenant tls auth config enterprise
    # Tooling
    full all bench dhat
)

usage() {
    cat <<EOF
Usage: $(basename "$0") <name> [extra cargo args]

Runs \`cargo run --example <name>\` with the required Cargo features
already attached. Pass --list to print every example name.

Examples:
  $(basename "$0") hello
  $(basename "$0") enterprise
  $(basename "$0") dhat --release
EOF
}

if [[ $# -lt 1 ]]; then
    usage >&2
    exit 1
fi

name="$1"
shift

if [[ "$name" == "--list" || "$name" == "-l" ]]; then
    printf '%s\n' "${KNOWN_EXAMPLES[@]}"
    exit 0
fi

if [[ "$name" == "--help" || "$name" == "-h" ]]; then
    usage
    exit 0
fi

# Validate name against the known list so a typo gets caught early
# rather than landing on Cargo's "no such target" later.
if ! printf '%s\n' "${KNOWN_EXAMPLES[@]}" | grep -qx "$name"; then
    echo "error: unknown example '$name'" >&2
    echo "       run \`$(basename "$0") --list\` to see all examples" >&2
    exit 2
fi

features="$(declare_features "$name")"

if [[ -n "$features" ]]; then
    set -x
    exec cargo run --example "$name" --features "$features" "$@"
else
    set -x
    exec cargo run --example "$name" "$@"
fi
