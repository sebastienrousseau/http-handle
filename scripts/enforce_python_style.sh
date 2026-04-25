#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

set -euo pipefail

mapfile -t py_files < <(rg --files -g '*.py' -g '*.pyi' || true)
if [[ ${#py_files[@]} -eq 0 ]]; then
  echo "No Python files found; Python style gate passes by policy."
  exit 0
fi

for tool in ruff black isort; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing required formatter/linter: $tool"
    echo "Install pinned tooling in CI before running this gate."
    exit 1
  fi
done

ruff check .
black --check .
isort --check-only .

echo "Python style checks passed (Google constraints + strict formatting)."
