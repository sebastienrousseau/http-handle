# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Sebastien Rousseau

#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${root_dir}"

status=0

echo "CI workflow audit"
echo "=================="

echo ""
echo "[1/3] Checking top-level permissions in workflows..."
missing_permissions=()
while IFS= read -r wf; do
  if ! rg -q '^\s*permissions:\s*$|^\s*permissions:\s+\S+' "${wf}"; then
    missing_permissions+=("${wf}")
  fi
done < <(find .github/workflows -maxdepth 1 -name '*.yml' -type f | sort)

if [[ ${#missing_permissions[@]} -eq 0 ]]; then
  echo "PASS: all workflows declare permissions"
else
  echo "FAIL: workflows missing permissions:"
  printf ' - %s\n' "${missing_permissions[@]}"
  status=1
fi

echo ""
echo "[2/3] Checking unpinned action references..."
unpinned=()
while IFS= read -r line; do
  # Matches @vN, @main, @master, @nightly style refs.
  unpinned+=("${line}")
done < <(rg -n 'uses:\s+[^#\n]+@(v[0-9]+|main|master|nightly)(\s|$)' .github/workflows -S || true)

if [[ ${#unpinned[@]} -eq 0 ]]; then
  echo "PASS: no unpinned action refs found"
else
  echo "FAIL: unpinned action refs detected:"
  printf ' - %s\n' "${unpinned[@]}"
  status=1
fi

echo ""
echo "[3/3] Checking use of @latest patterns..."
latest_refs=()
while IFS= read -r line; do
  latest_refs+=("${line}")
done < <(rg -n '@latest|go install .*@latest' .github/workflows scripts/perf -S || true)

if [[ ${#latest_refs[@]} -eq 0 ]]; then
  echo "PASS: no @latest patterns found"
else
  echo "FAIL: @latest patterns detected:"
  printf ' - %s\n' "${latest_refs[@]}"
  status=1
fi

echo ""
if [[ ${status} -eq 0 ]]; then
  echo "Audit result: PASS"
else
  echo "Audit result: FAIL"
fi

exit "${status}"
