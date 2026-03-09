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
  if command -v rg >/dev/null 2>&1; then
    if rg -q --pcre2 "${pattern}" "${file}"; then
      return 0
    fi
  else
    if PATTERN="${pattern}" perl -ne 'BEGIN { $p = $ENV{"PATTERN"}; $found = 0 } $found = 1 if /$p/; END { exit($found ? 0 : 1) }' "${file}"; then
      return 0
    fi
  fi

  failures+=("${msg} (${file})")
}

expect_file() {
  local file="$1"
  local msg="$2"
  if [[ ! -f "${file}" ]]; then
    failures+=("${msg} (${file})")
  fi
}

expect_file ".github/workflows/release-readiness-gate.yml" \
  "Release readiness aggregate gate workflow must exist"
expect_file ".github/workflows/slo-release-gate.yml" \
  "Dedicated SLO/SLI gate workflow must exist"

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
  "Release readiness aggregate gate workflow name must be present"
expect_grep "verify_execution_plan_p0\\.sh" ".github/workflows/release-readiness-gate.yml" \
  "Release readiness gate must verify execution-plan P0 controls"
expect_grep "validate_docs_ia\\.sh" ".github/workflows/release-readiness-gate.yml" \
  "Release readiness gate must validate docs IA coverage"
expect_grep "enforce_slo_gate\\.sh" ".github/workflows/release-readiness-gate.yml" \
  "Release readiness gate must enforce SLO/SLI release policy"
expect_grep "SLO/SLI Release Gate" ".github/workflows/slo-release-gate.yml" \
  "Dedicated SLO/SLI gate workflow name must be present"

if [[ "${#failures[@]}" -gt 0 ]]; then
  echo "Execution plan P0 verification failed:"
  for item in "${failures[@]}"; do
    echo " - ${item}"
  done
  exit 1
fi

echo "Execution plan P0 verification passed."
