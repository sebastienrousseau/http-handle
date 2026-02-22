# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Sebastien Rousseau

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

passed=0
total=0
failures=()

has_rg() {
  command -v rg >/dev/null 2>&1
}

contains_regex() {
  local pattern="$1"
  local file="$2"
  if has_rg; then
    rg -q --pcre2 "${pattern}" "${file}"
  else
    grep -Eq "${pattern}" "${file}"
  fi
}

add_check() {
  local description="$1"
  local cmd="$2"
  total=$((total + 1))
  if eval "${cmd}" >/dev/null 2>&1; then
    passed=$((passed + 1))
    printf '[PASS] %s\n' "${description}"
  else
    failures+=("${description}")
    printf '[FAIL] %s\n' "${description}"
  fi
}

add_check "README has Architectural Overview" \
  "contains_regex '^## Architectural Overview$' README.md"
add_check "README has Feature List" \
  "contains_regex '^## Feature List$' README.md"
add_check "README has Quick Start" \
  "contains_regex '^## Quick Start$' README.md"
add_check "README has Platform Support Matrix" \
  "contains_regex '^## Platform Support Matrix$' README.md"
add_check "lib.rs includes README as crate docs" \
  "contains_regex '#!\\[doc = include_str!\\(\"\\.\\./README\\.md\"\\)\\]' src/lib.rs"
add_check "lib.rs has rustdoc branding metadata" \
  "contains_regex 'html_favicon_url|html_logo_url|html_root_url' src/lib.rs"
add_check "lib.rs enables docsrs doc_cfg" \
  "contains_regex '#!\\[cfg_attr\\(docsrs, feature\\(doc_cfg\\)\\)\\]' src/lib.rs"
add_check "Cargo.toml docs.rs has all-features + cfg docsrs" \
  "contains_regex '^\\[package\\.metadata\\.docs\\.rs\\]' Cargo.toml && contains_regex '^all-features = true' Cargo.toml && contains_regex '^rustdoc-args = \\[\"--cfg\", \"docsrs\"\\]' Cargo.toml"
add_check "Cargo.toml docs.rs targets include macOS + Linux" \
  "contains_regex 'targets = \\[\"x86_64-apple-darwin\", \"x86_64-unknown-linux-gnu\"\\]' Cargo.toml"
add_check "Feature-gated modules in lib.rs expose doc(cfg)" \
  "if has_rg; then rg -n '#\\[cfg\\(feature = \"' src/lib.rs | wc -l | xargs -I{} test {} -gt 0 && rg -n '#\\[cfg_attr\\(docsrs, doc\\(cfg\\(feature = \"' src/lib.rs | wc -l | xargs -I{} test {} -gt 0; else grep -En '#\\[cfg\\(feature = \"' src/lib.rs | wc -l | xargs -I{} test {} -gt 0 && grep -En '#\\[cfg_attr\\(docsrs, doc\\(cfg\\(feature = \"' src/lib.rs | wc -l | xargs -I{} test {} -gt 0; fi"
add_check "README links tutorials guide" \
  "contains_regex 'docs/TUTORIALS\\.md' README.md && test -f docs/TUTORIALS.md"
add_check "README links architecture diagrams" \
  "contains_regex 'docs/ARCHITECTURE\\.md' README.md && test -f docs/ARCHITECTURE.md"
add_check "README links benchmark reproducibility guide" \
  "contains_regex 'docs/BENCHMARK_REPRODUCIBILITY\\.md' README.md && test -f docs/BENCHMARK_REPRODUCIBILITY.md"

score=$((passed * 100 / total))
printf '\nDocumentation Score: %d/100\n' "${score}"
printf 'Checks: %d/%d passed\n' "${passed}" "${total}"

if [[ "${score}" -lt 100 ]]; then
  printf '\nMissing checks:\n'
  for item in "${failures[@]}"; do
    printf ' - %s\n' "${item}"
  done
  exit 1
fi
