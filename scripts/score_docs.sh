#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

passed=0
total=0
failures=()

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
  "rg -q '^## Architectural Overview$' README.md"
add_check "README has Feature List" \
  "rg -q '^## Feature List$' README.md"
add_check "README has Quick Start" \
  "rg -q '^## Quick Start$' README.md"
add_check "README has Platform Support Matrix" \
  "rg -q '^## Platform Support Matrix$' README.md"
add_check "lib.rs includes README as crate docs" \
  "rg -q '#!\\[doc = include_str!\\(\"\\.\\./README\\.md\"\\)\\]' src/lib.rs"
add_check "lib.rs has rustdoc branding metadata" \
  "rg -q 'html_favicon_url|html_logo_url|html_root_url' src/lib.rs"
add_check "lib.rs enables docsrs doc_cfg" \
  "rg -q '#!\\[cfg_attr\\(docsrs, feature\\(doc_cfg\\)\\)\\]' src/lib.rs"
add_check "Cargo.toml docs.rs has all-features + cfg docsrs" \
  "rg -q '^\\[package\\.metadata\\.docs\\.rs\\]' Cargo.toml && rg -q '^all-features = true' Cargo.toml && rg -q '^rustdoc-args = \\[\"--cfg\", \"docsrs\"\\]' Cargo.toml"
add_check "Cargo.toml docs.rs targets include macOS + Linux" \
  "rg -q 'targets = \\[\"x86_64-apple-darwin\", \"x86_64-unknown-linux-gnu\"\\]' Cargo.toml"
add_check "Feature-gated modules in lib.rs expose doc(cfg)" \
  "rg -n '#\\[cfg\\(feature = \"' src/lib.rs | wc -l | xargs -I{} test {} -gt 0 && rg -n '#\\[cfg_attr\\(docsrs, doc\\(cfg\\(feature = \"' src/lib.rs | wc -l | xargs -I{} test {} -gt 0"
add_check "README links tutorials guide" \
  "rg -q 'docs/TUTORIALS\\.md' README.md && test -f docs/TUTORIALS.md"
add_check "README links architecture diagrams" \
  "rg -q 'docs/ARCHITECTURE\\.md' README.md && test -f docs/ARCHITECTURE.md"
add_check "README links benchmark reproducibility guide" \
  "rg -q 'docs/BENCHMARK_REPRODUCIBILITY\\.md' README.md && test -f docs/BENCHMARK_REPRODUCIBILITY.md"

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
