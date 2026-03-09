# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Sebastien Rousseau

#!/usr/bin/env bash
set -euo pipefail

RESULT_JSON="${PERF_RESULT_JSON:-target/perf-result.json}"
OUT_MD="${PERF_CLAIM_REPORT_MD:-target/perf-claim-report.md}"
BASELINE_FILE="${PERF_BASELINE_FILE:-scripts/perf/baseline.json}"
BASELINE_DIR="${PERF_BASELINE_DIR:-scripts/perf}"

if [[ ! -f "${RESULT_JSON}" ]]; then
  echo "Missing benchmark result JSON: ${RESULT_JSON}"
  exit 1
fi

if [[ -z "${PERF_BASELINE_FILE:-}" && -f "Cargo.toml" ]]; then
  VERSION="$(awk -F '"' '/^version = / {print $2; exit}' Cargo.toml)"
  if [[ -n "${VERSION:-}" && -f "${BASELINE_DIR}/baseline-v${VERSION}.json" ]]; then
    BASELINE_FILE="${BASELINE_DIR}/baseline-v${VERSION}.json"
  fi
fi

RPS="$(jq -r '.rps' "${RESULT_JSON}")"
P99="$(jq -r '.p99_latency_ms' "${RESULT_JSON}")"
CORES="$(jq -r '.cores' "${RESULT_JSON}")"
RPS_PER_CORE="$(jq -r '.rps_per_core' "${RESULT_JSON}")"

BASE_RPS="n/a"
BASE_P99="n/a"
BASE_RPS_PER_CORE="n/a"
if [[ -f "${BASELINE_FILE}" ]]; then
  BASE_RPS="$(jq -r '.rps_min' "${BASELINE_FILE}")"
  BASE_P99="$(jq -r '.p99_latency_ms_max' "${BASELINE_FILE}")"
  BASE_RPS_PER_CORE="$(jq -r '.rps_per_core_min' "${BASELINE_FILE}")"
fi

mkdir -p "$(dirname "${OUT_MD}")"
cat > "${OUT_MD}" <<EOF
# Performance Claim Report

## Measured Result

- rps: ${RPS}
- p99_latency_ms: ${P99}
- cores: ${CORES}
- rps_per_core: ${RPS_PER_CORE}

## Baseline Thresholds

- rps_min: ${BASE_RPS}
- p99_latency_ms_max: ${BASE_P99}
- rps_per_core_min: ${BASE_RPS_PER_CORE}

## Claim Policy

- Performance claims must be supported by \`target/perf-result.json\`.
- Release claims must reference this report artifact and baseline file used in CI.
EOF

echo "Wrote claim report: ${OUT_MD}"
