#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2023 - 2026 HTTP Handle

set -euo pipefail

status=0

# Core layer must remain independent from higher-level orchestration layers.
core_files=(
  src/error.rs
  src/request.rs
  src/response.rs
  src/language.rs
  src/optimized.rs
  src/streaming.rs
  src/batch.rs
)

if rg -n "crate::(server|perf_server|enterprise|distributed_rate_limit|tenant_isolation|runtime_autotune|http2_server|http3_profile|async_server)" "${core_files[@]}"; then
  echo "Architecture independence violation: core layer imports higher-level layers."
  status=1
fi

# Enterprise/feature modules should not depend on bin entrypoints.
if rg -n "crate::bin::|src/bin" src/enterprise.rs src/distributed_rate_limit.rs src/tenant_isolation.rs src/runtime_autotune.rs; then
  echo "Architecture independence violation: library modules depend on binaries."
  status=1
fi

if [[ $status -ne 0 ]]; then
  exit 1
fi

echo "Architecture independence checks passed."
