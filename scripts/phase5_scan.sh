#!/usr/bin/env bash
set -euo pipefail

legacy_hits="$(rg -n "crate::model::flatten|symbol::functions" ospf-rust-core/src || true)"
legacy_hits="$(printf "%s\n" "${legacy_hits}" | rg -v "ospf-rust-core/src/symbol/flatten/mod.rs|ospf-rust-core/src/symbol/functions/mod.rs" || true)"
if [[ -n "${legacy_hits}" ]]; then
  echo "legacy path scan failed:"
  echo "${legacy_hits}"
  exit 1
fi

boundary_hits="$(rg -n "to_f64_with_policy|from_f64_with_policy" ospf-rust-core/src/solver || true)"
boundary_hits="$(printf "%s\n" "${boundary_hits}" | rg -v "ospf-rust-core/src/solver/value/boundary.rs|ospf-rust-core/src/solver/value/solve_value.rs" || true)"
if [[ -n "${boundary_hits}" ]]; then
  echo "solver boundary scan failed:"
  echo "${boundary_hits}"
  exit 1
fi

f64_leak_hits="$(rg -n "pub\\s+(fn|type|struct|enum).*f64" ospf-rust-core/src/solver/value || true)"
f64_leak_hits="$(printf "%s\n" "${f64_leak_hits}" | rg -v "ospf-rust-core/src/solver/value/boundary.rs|ospf-rust-core/src/solver/value/solve_value.rs" || true)"
if [[ -n "${f64_leak_hits}" ]]; then
  echo "public API f64 leak scan failed:"
  echo "${f64_leak_hits}"
  exit 1
fi

echo "phase5 scans passed"
