# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

SLA_FILE="security/findings-sla.tsv"
REPORT_MD="${SECURITY_SLA_REPORT_MD:-target/security-sla-report.md}"
failures=()

if [[ ! -f "${SLA_FILE}" ]]; then
  echo "Missing security SLA registry: ${SLA_FILE}"
  exit 1
fi

now_ts="$(date -u +%s)"

append_failure() {
  failures+=("$1")
}

mkdir -p "$(dirname "${REPORT_MD}")"
{
  echo "# Security Findings SLA Report"
  echo
  echo "| id | severity | status | owner | age_days | sla_days |"
  echo "|---|---|---|---|---:|---:|"
} > "${REPORT_MD}"

while IFS=$'\t' read -r id severity status owner sla_days discovered_on last_reviewed notes; do
  [[ -z "${id}" ]] && continue
  [[ "${id}" =~ ^# ]] && continue

  if [[ -z "${severity}" || -z "${status}" || -z "${owner}" || -z "${sla_days}" || -z "${discovered_on}" ]]; then
    append_failure "Invalid finding row '${id}' (severity/status/owner/sla_days/discovered_on required)."
    continue
  fi

  if [[ ! "${severity}" =~ ^(LOW|MEDIUM|HIGH|CRITICAL)$ ]]; then
    append_failure "Invalid severity '${severity}' for ${id}."
    continue
  fi

  if [[ ! "${status}" =~ ^(OPEN|IN_PROGRESS|ACCEPTED_RISK|RESOLVED)$ ]]; then
    append_failure "Invalid status '${status}' for ${id}."
    continue
  fi

  discovered_ts="$(date -u -d "${discovered_on}" +%s 2>/dev/null || true)"
  if [[ -z "${discovered_ts}" ]]; then
    append_failure "Invalid discovered_on date '${discovered_on}' for ${id}."
    continue
  fi

  age_days="$(( (now_ts - discovered_ts) / 86400 ))"
  echo "| ${id} | ${severity} | ${status} | ${owner} | ${age_days} | ${sla_days} |" >> "${REPORT_MD}"

  if [[ "${status}" == "RESOLVED" ]]; then
    continue
  fi

  if (( age_days > sla_days )); then
    append_failure "SLA breach for ${id}: ${age_days}d > ${sla_days}d (owner: ${owner})."
  fi

  if [[ "${severity}" == "HIGH" || "${severity}" == "CRITICAL" ]]; then
    append_failure "Open ${severity} finding ${id} is not allowed by zero-high policy."
  fi
done < "${SLA_FILE}"

if [[ "${#failures[@]}" -gt 0 ]]; then
  echo "Security SLA checks failed:"
  for item in "${failures[@]}"; do
    echo " - ${item}"
  done
  echo
  echo "Rendered SLA report: ${REPORT_MD}"
  exit 1
fi

echo "Security SLA checks passed."
echo "Rendered SLA report: ${REPORT_MD}"
