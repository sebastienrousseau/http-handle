#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Sebastien Rousseau

set -euo pipefail

mapfile -t py_files < <(rg --files -g '*.py' -g '*.pyi' || true)
if [[ ${#py_files[@]} -eq 0 ]]; then
  echo "No Python files found; Python style gate passes by policy."
  exit 0
fi

python3 -m pip install --quiet black isort ruff

ruff check .
black --check .
isort --check-only .

echo "Python style checks passed (Google constraints + strict formatting)."
