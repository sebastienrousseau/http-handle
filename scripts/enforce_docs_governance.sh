# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

GOV_FILE="docs/DOCS_GOVERNANCE.tsv"
failures=()

if [[ ! -f "${GOV_FILE}" ]]; then
  echo "Missing governance file: ${GOV_FILE}"
  exit 1
fi

append_failure() {
  failures+=("$1")
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

resolve_base_sha() {
  if [[ -n "${GITHUB_BASE_SHA:-}" ]]; then
    printf '%s\n' "${GITHUB_BASE_SHA}"
    return 0
  fi

  if [[ -n "${GITHUB_EVENT_PATH:-}" ]] && has_cmd jq; then
    local pr_base
    pr_base="$(jq -r '.pull_request.base.sha // empty' "${GITHUB_EVENT_PATH}" 2>/dev/null || true)"
    if [[ -n "${pr_base}" ]]; then
      printf '%s\n' "${pr_base}"
      return 0
    fi
  fi

  if git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
    git rev-parse HEAD~1
    return 0
  fi

  printf '\n'
}

collect_changed_files() {
  local base_sha head_sha
  base_sha="$(resolve_base_sha)"
  head_sha="${GITHUB_SHA:-$(git rev-parse HEAD)}"

  if [[ -n "${base_sha}" ]] && git rev-parse --verify "${base_sha}" >/dev/null 2>&1; then
    git diff --name-only "${base_sha}" "${head_sha}"
  else
    git show --pretty="" --name-only "${head_sha}"
  fi
}

enforce_docs_change_coupling() {
  local changed has_src has_docs
  changed="$(collect_changed_files || true)"
  has_src=0
  has_docs=0

  while IFS= read -r file; do
    [[ -z "${file}" ]] && continue
    if [[ "${file}" == src/* ]]; then
      has_src=1
    fi
    if [[ "${file}" == docs/* || "${file}" == README.md || "${file}" == CHANGELOG.md ]]; then
      has_docs=1
    fi
  done <<< "${changed}"

  if [[ "${has_src}" -eq 1 && "${has_docs}" -eq 0 ]]; then
    append_failure "Source changed without docs updates (docs/**, README.md, or CHANGELOG.md)."
  fi
}

enforce_staleness_and_ownership() {
  local now_ts
  now_ts="$(date -u +%s)"

  while IFS=$'\t' read -r path owner max_age_days last_reviewed; do
    [[ -z "${path}" ]] && continue
    [[ "${path}" =~ ^# ]] && continue

    if [[ -z "${owner}" || -z "${max_age_days}" || -z "${last_reviewed}" ]]; then
      append_failure "Invalid governance row for ${path} (owner/max_age_days/last_reviewed required)."
      continue
    fi

    if [[ ! -f "${path}" ]]; then
      append_failure "Governance path missing: ${path} (owner: ${owner})."
      continue
    fi

    if [[ ! "${last_reviewed}" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
      append_failure "Invalid last_reviewed date for ${path}: ${last_reviewed} (expected YYYY-MM-DD)."
      continue
    fi

    local reviewed_ts age_days
    reviewed_ts="$(date -u -d "${last_reviewed}" +%s 2>/dev/null || true)"
    if [[ -z "${reviewed_ts}" ]]; then
      append_failure "Unparseable last_reviewed date for ${path}: ${last_reviewed}."
      continue
    fi

    age_days="$(( (now_ts - reviewed_ts) / 86400 ))"
    if (( age_days > max_age_days )); then
      append_failure "Stale docs: ${path} is ${age_days}d old (max ${max_age_days}d). Owner: ${owner}."
    fi
  done < "${GOV_FILE}"
}

enforce_docs_change_coupling
enforce_staleness_and_ownership

if [[ "${#failures[@]}" -gt 0 ]]; then
  echo "Documentation governance checks failed:"
  for item in "${failures[@]}"; do
    echo " - ${item}"
  done
  exit 1
fi

echo "Documentation governance checks passed."
