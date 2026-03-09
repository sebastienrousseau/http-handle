# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Sebastien Rousseau

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

failures=()

expect_grep() {
  local pattern="$1"
  local file="$2"
  local msg="$3"
  if ! rg -q --pcre2 "${pattern}" "${file}"; then
    failures+=("${msg} (${file})")
  fi
}

expect_grep "PERF_BASELINE_DIR" ".github/workflows/perf-regression.yml" \
  "Perf regression workflow must use version-aware baseline directory"
expect_grep "perf-claim-report" ".github/workflows/perf-regression.yml" \
  "Perf regression workflow must upload claim report artifact"

expect_grep "attest-build-provenance" ".github/workflows/release-artifacts.yml" \
  "Release artifacts workflow must attest provenance"
expect_grep "^\\s*release:\\s*$" ".github/workflows/sbom-attestation.yml" \
  "SBOM attestation must declare release trigger"
expect_grep "^\\s*types:\\s*\\[published\\]" ".github/workflows/sbom-attestation.yml" \
  "SBOM attestation must run on published releases"

expect_grep "cargo audit --deny warnings" ".github/workflows/security-zero-high.yml" \
  "Security workflow must deny cargo-audit warnings"
expect_grep "findings-sla" ".github/workflows/security-zero-high.yml" \
  "Security workflow must enforce findings SLA tracking"

expect_grep "enforce_docs_governance\\.sh" ".github/workflows/release.yml" \
  "Release workflow must enforce docs governance"
expect_grep "Release Readiness Gate" ".github/workflows/release-readiness-gate.yml" \
  "Release readiness aggregate gate workflow must exist"
expect_grep "verify_execution_plan_p0\\.sh" ".github/workflows/release-readiness-gate.yml" \
  "Release readiness gate must verify execution-plan P0 controls"
expect_grep "validate_docs_ia\\.sh" ".github/workflows/release-readiness-gate.yml" \
  "Release readiness gate must validate docs IA coverage"

if [[ "${#failures[@]}" -gt 0 ]]; then
  echo "Execution plan P0 verification failed:"
  for item in "${failures[@]}"; do
    echo " - ${item}"
  done
  exit 1
fi

echo "Execution plan P0 verification passed."
