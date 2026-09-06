#!/usr/bin/env bash
set -euo pipefail

workspace_root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")"/../../../../../ && pwd)}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ -z "${SCIPOPTDIR:-}" ]]; then
  echo "SCIPOPTDIR is not set. Please set SCIPOPTDIR to your SCIP installation root." >&2
  exit 1
fi

if [[ ! -d "$workspace_root" ]]; then
  echo "Workspace root not found: $workspace_root" >&2
  exit 1
fi

cd "$workspace_root"

bash "$script_dir/check_non_bundled_env.sh"

echo "[scip] workspace root: $workspace_root"
echo "[scip] SCIPOPTDIR: $SCIPOPTDIR"
if [[ -n "${LIBCLANG_PATH:-}" ]]; then
  echo "[scip] LIBCLANG_PATH: $LIBCLANG_PATH"
fi

commands=(
  "cargo test -p ospf-rust-core --test scip_native_observer_integration --features scip -- --nocapture"
  "cargo test -p ospf-rust-framework --test scip_native_callback_integration --features scip -- --nocapture"
  "cargo test -p ospf-rust-framework --test scip_native_callback_async_integration --features 'scip async' -- --nocapture"
)

for command in "${commands[@]}"; do
  echo "[scip] running: $command"
  bash -lc "$command"
done

echo "[scip] non-bundled regression passed."
