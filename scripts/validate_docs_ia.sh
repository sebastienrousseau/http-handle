# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

README_FILE="README.md"
failures=()

require_link() {
  local path="$1"
  if ! grep -q "${path}" "${README_FILE}"; then
    failures+=("README missing docs link: ${path}")
  fi
  if [[ ! -f "${path}" ]]; then
    failures+=("Document file missing: ${path}")
  fi
}

require_link "docs/TUTORIALS.md"
require_link "docs/ARCHITECTURE.md"
require_link "docs/BENCHMARK_REPRODUCIBILITY.md"
require_link "docs/BENCHMARK_CLAIM_GOVERNANCE.md"
require_link "docs/SECURITY_SLA_POLICY.md"
require_link "docs/DEPRECATION_POLICY.md"
require_link "docs/LTS_POLICY.md"
require_link "docs/MIGRATION_GUIDE.md"
require_link "docs/RECIPES.md"
require_link "docs/PORTABILITY_MATRIX.md"
require_link "docs/EXECUTION_PLAN_v0.0.4.md"

if [[ "${#failures[@]}" -gt 0 ]]; then
  echo "Docs IA validation failed:"
  for item in "${failures[@]}"; do
    echo " - ${item}"
  done
  exit 1
fi

echo "Docs IA validation passed."
