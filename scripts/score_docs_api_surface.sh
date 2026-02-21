#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

JSON_PATH="target/doc/http_handle.json"

echo "Building rustdoc JSON (nightly, docsrs cfg)..."
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly rustdoc --lib --all-features -- -Z unstable-options --output-format json >/dev/null

if [[ ! -f "${JSON_PATH}" ]]; then
  echo "rustdoc JSON not found at ${JSON_PATH}"
  exit 1
fi

tmp_report="$(mktemp)"

jq -r '
  def is_result_output($t):
    if ($t | type) != "object" then false
    elif ($t.resolved_path? != null) then ($t.resolved_path.path == "Result")
    elif ($t.borrowed_ref? != null) then is_result_output($t.borrowed_ref.type)
    elif ($t.raw_pointer? != null) then is_result_output($t.raw_pointer.type)
    elif ($t.slice? != null) then is_result_output($t.slice)
    elif ($t.array? != null) then is_result_output($t.array.type)
    elif ($t.tuple? != null) then any($t.tuple[]; is_result_output(.))
    elif ($t.qualified_path? != null) then is_result_output($t.qualified_path.self_type)
    else false
    end;

  .index
  | to_entries[]
  | .value as $it
  | select(
      $it.crate_id == 0
      and $it.visibility == "public"
      and ($it.span.filename | startswith("src/"))
      and (($it.span.filename | test("src/(server|request|response|error|lib)\\.rs")) | not)
    )
  | ($it.inner | keys[0]) as $kind
  | select($kind == "struct" or $kind == "enum" or $kind == "trait" or $kind == "function")
  | ($it.docs // "") as $docs
  | ($docs | test("(?m)^# Examples\\b")) as $has_examples
  | ($docs | test("(?m)^# Panics\\b")) as $has_panics
  | (($kind == "function") and is_result_output($it.inner.function.sig.output)) as $needs_errors
  | ($docs | test("(?m)^# Errors\\b")) as $has_errors
  | (
      (($kind == "function") and ($it.inner.function.header.is_unsafe))
      or (($kind == "trait") and ($it.inner.trait.is_unsafe))
    ) as $needs_safety
  | ($docs | test("(?m)^# Safety\\b")) as $has_safety
  | {
      file: $it.span.filename,
      kind: $kind,
      name: ($it.name // "<unnamed>"),
      missing: [
        (if $has_examples then empty else "Examples" end),
        (if $has_panics then empty else "Panics" end),
        (if ($needs_errors and ($has_errors | not)) then "Errors" else empty end),
        (if ($needs_safety and ($has_safety | not)) then "Safety" else empty end)
      ]
    }
  | select(.missing | length > 0)
  | "\(.file)\t\(.kind)\t\(.name)\t\(.missing | join(","))"
' "${JSON_PATH}" > "${tmp_report}"

missing_count="$(wc -l < "${tmp_report}" | tr -d ' ')"

if [[ "${missing_count}" -eq 0 ]]; then
  echo "API Surface Documentation Score: 100/100"
  echo "All checked public non-core items include required sections."
  exit 0
fi

echo "API Surface Documentation Score: 0/100"
echo "Missing required sections on ${missing_count} public non-core items:"
sed -n '1,200p' "${tmp_report}" | awk -F'\t' '{printf " - %s [%s %s] missing: %s\n", $1, $2, $3, $4}'
exit 1
