# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

DOC_FILE="docs/PORTABILITY_MATRIX.md"
WF_FILE=".github/workflows/portability.yml"

if [[ ! -f "${DOC_FILE}" || ! -f "${WF_FILE}" ]]; then
  echo "Missing required files for portability validation."
  exit 1
fi

failures=()

add_failure() {
  failures+=("$1")
}

while IFS= read -r target; do
  [[ -z "${target}" ]] && continue
  if ! grep -q "${target}" "${DOC_FILE}"; then
    add_failure "Target '${target}' is in portability workflow but missing from ${DOC_FILE}."
  fi
done < <(awk '/^\s*-\s+[A-Za-z0-9_]+-[A-Za-z0-9_]+-[A-Za-z0-9_]+/ {gsub(/^\s*-\s+/, "", $0); print $0}' "${WF_FILE}" | sort -u)

for os in ubuntu-latest macos-latest windows-latest; do
  if ! grep -q "${os}" "${WF_FILE}"; then
    add_failure "Host matrix OS '${os}' is missing from portability workflow."
  fi
done

for mode in check cross-check conformance; do
  if ! grep -q "${mode}" "${DOC_FILE}"; then
    add_failure "Validation mode '${mode}' missing from ${DOC_FILE}."
  fi
done

if [[ "${#failures[@]}" -gt 0 ]]; then
  echo "Portability matrix validation failed:"
  for item in "${failures[@]}"; do
    echo " - ${item}"
  done
  exit 1
fi

echo "Portability matrix validation passed."
