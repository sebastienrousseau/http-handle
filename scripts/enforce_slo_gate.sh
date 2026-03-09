# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Sebastien Rousseau

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

GATE_FILE="${SLO_GATE_FILE:-slo/release-gate.json}"
SLI_FILE="${SLI_METRICS_FILE:-slo/current-sli.json}"
REPORT_MD="${SLO_GATE_REPORT_MD:-target/slo-gate-report.md}"

if [[ ! -f "${GATE_FILE}" ]]; then
  echo "Missing SLO gate config: ${GATE_FILE}"
  exit 1
fi
if [[ ! -f "${SLI_FILE}" ]]; then
  echo "Missing SLI metrics input: ${SLI_FILE}"
  exit 1
fi

mkdir -p "$(dirname "${REPORT_MD}")"

validate_range() {
  local key="$1" min="$2" max="$3"
  local value
  value="$(jq -r ".${key}" "${GATE_FILE}")"
  awk -v v="${value}" -v lo="${min}" -v hi="${max}" 'BEGIN { if (v < lo || v > hi) exit 1 }' || {
    echo "Invalid SLO config '${key}=${value}' (expected ${min}..${max})"
    exit 1
  }
}

validate_range "availability_percent_min" 0 100
validate_range "latency_p99_ms_max" 1 1000000
validate_range "error_rate_percent_max" 0 100
validate_range "error_budget_remaining_percent_min" 0 100
validate_range "throughput_rps_per_core_min" 0 10000000

availability_min="$(jq -r '.availability_percent_min' "${GATE_FILE}")"
latency_max="$(jq -r '.latency_p99_ms_max' "${GATE_FILE}")"
error_rate_max="$(jq -r '.error_rate_percent_max' "${GATE_FILE}")"
budget_min="$(jq -r '.error_budget_remaining_percent_min' "${GATE_FILE}")"
throughput_min="$(jq -r '.throughput_rps_per_core_min' "${GATE_FILE}")"

availability="$(jq -r '.availability_percent' "${SLI_FILE}")"
latency="$(jq -r '.latency_p99_ms' "${SLI_FILE}")"
error_rate="$(jq -r '.error_rate_percent' "${SLI_FILE}")"
budget="$(jq -r '.error_budget_remaining_percent' "${SLI_FILE}")"
throughput="$(jq -r '.throughput_rps_per_core' "${SLI_FILE}")"

impacted=()
blocked=0
remediation=()

awk -v v="${availability}" -v min="${availability_min}" 'BEGIN { if (v < min) exit 1 }' || {
  impacted+=("availability_percent")
  remediation+=("Increase reliability before promotion (stabilize failure paths and retries).")
}
awk -v v="${latency}" -v max="${latency_max}" 'BEGIN { if (v > max) exit 1 }' || {
  impacted+=("latency_p99_ms")
  remediation+=("Reduce p99 latency by tuning runtime limits, queues, or hot paths.")
}
awk -v v="${error_rate}" -v max="${error_rate_max}" 'BEGIN { if (v > max) exit 1 }' || {
  impacted+=("error_rate_percent")
  remediation+=("Reduce 5xx/transport errors before release promotion.")
}
awk -v v="${throughput}" -v min="${throughput_min}" 'BEGIN { if (v < min) exit 1 }' || {
  impacted+=("throughput_rps_per_core")
  remediation+=("Recover throughput/core to target via perf regression analysis.")
}

awk -v v="${budget}" -v min="${budget_min}" 'BEGIN { if (v <= 0 || v < min) exit 1 }' || {
  impacted+=("error_budget_remaining_percent")
  remediation+=("Error budget is exhausted/low; halt release and recover SLO burn.")
  blocked=1
}

{
  echo "# SLO/SLI Release Gate Report"
  echo
  echo "## Inputs"
  echo "- Gate config: \`${GATE_FILE}\`"
  echo "- SLI metrics: \`${SLI_FILE}\`"
  echo
  echo "## Thresholds"
  echo "- availability_percent_min: ${availability_min}"
  echo "- latency_p99_ms_max: ${latency_max}"
  echo "- error_rate_percent_max: ${error_rate_max}"
  echo "- error_budget_remaining_percent_min: ${budget_min}"
  echo "- throughput_rps_per_core_min: ${throughput_min}"
  echo
  echo "## Measured"
  echo "- availability_percent: ${availability}"
  echo "- latency_p99_ms: ${latency}"
  echo "- error_rate_percent: ${error_rate}"
  echo "- error_budget_remaining_percent: ${budget}"
  echo "- throughput_rps_per_core: ${throughput}"
  echo
  if [[ "${#impacted[@]}" -eq 0 ]]; then
    echo "## Result"
    echo "PASS: SLO gate satisfied; release promotion allowed."
  else
    echo "## Result"
    echo "FAIL: SLO gate blocked."
    echo
    echo "Impacted SLI dimensions:"
    for sli in "${impacted[@]}"; do
      echo "- ${sli}"
    done
    echo
    echo "Remediation guidance:"
    for step in "${remediation[@]}"; do
      echo "- ${step}"
    done
  fi
} > "${REPORT_MD}"

if [[ "${#impacted[@]}" -gt 0 ]]; then
  cat "${REPORT_MD}"
  exit 1
fi

cat "${REPORT_MD}"
echo "SLO gate passed."
