#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

set -euo pipefail

mode="${1:-check}"

rust_header='// SPDX-License-Identifier: AGPL-3.0-only\n// Copyright (c) 2023 - 2026 HTTP Handle\n'
shell_header='# SPDX-License-Identifier: AGPL-3.0-only\n# Copyright (c) 2023 - 2026 HTTP Handle\n'

status=0

apply_header() {
  local file="$1"
  local header="$2"
  local tmp
  tmp="$(mktemp)"
  {
    printf "%b\n" "$header"
    cat "$file"
  } >"$tmp"
  mv "$tmp" "$file"
}

check_or_apply() {
  local file="$1"
  local prefix="$2"
  local header="$3"
  if ! head -n 2 "$file" | grep -q "$prefix"; then
    if [[ "$mode" == "apply" ]]; then
      apply_header "$file" "$header"
      echo "Added AGPL header: $file"
    else
      echo "Missing AGPL header: $file"
      status=1
    fi
  fi
}

while IFS= read -r f; do
  check_or_apply "$f" "SPDX-License-Identifier: AGPL-3.0-only" "$rust_header"
done < <(find src tests examples benches fuzz -type f -name '*.rs' | sort)

while IFS= read -r f; do
  check_or_apply "$f" "SPDX-License-Identifier: AGPL-3.0-only" "$shell_header"
done < <(find scripts -type f -name '*.sh' | sort)

check_or_apply build.rs "SPDX-License-Identifier: AGPL-3.0-only" "$rust_header"

if [[ $status -ne 0 ]]; then
  echo "AGPL header enforcement failed."
  exit 1
fi

echo "AGPL headers validated."
