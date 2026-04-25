#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

set -euo pipefail

status=0

# Apple/Google naming convention baseline: descriptive, lowercase, snake_case for Rust files.
while IFS= read -r f; do
  base="$(basename "$f" .rs)"
  if [[ "$f" == src/bin/* ]]; then
    if [[ ! "$base" =~ ^[a-z0-9_-]+$ ]]; then
      echo "Invalid Rust binary filename (snake_case or kebab-case required): $f"
      status=1
    fi
    continue
  fi
  if [[ ! "$base" =~ ^[a-z0-9_]+$ ]]; then
    echo "Invalid Rust filename (snake_case required): $f"
    status=1
  fi
done < <(find src tests examples benches fuzz -type f -name '*.rs' | sort)

# Workflow names should be lowercase kebab-case.
while IFS= read -r f; do
  base="$(basename "$f" .yml)"
  if [[ ! "$base" =~ ^[a-z0-9-]+$ ]]; then
    echo "Invalid workflow filename (kebab-case required): $f"
    status=1
  fi
done < <(find .github/workflows -type f -name '*.yml' | sort)

if [[ $status -ne 0 ]]; then
  echo "Naming convention violations found."
  exit 1
fi

echo "Naming convention checks passed."
