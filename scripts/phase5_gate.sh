#!/usr/bin/env bash
set -euo pipefail

cargo fmt -p ospf-rust-core -p ospf-rust-framework --check
cargo check -p ospf-rust-core
cargo check -p ospf-rust-framework
cargo test -p ospf-rust-core --lib

if [[ "${OSPF_CHECK_SCIP_FEATURE:-}" == "1" ]]; then
  cargo check -p ospf-rust-core --features scip
  cargo check -p ospf-rust-framework --features scip
  cargo test -p ospf-rust-core --features scip scip
fi

if [[ -n "${OSPF_GUROBI_FEATURE:-}" ]]; then
  cargo check -p ospf-rust-core --features "${OSPF_GUROBI_FEATURE}"
  cargo check -p ospf-rust-framework --features "${OSPF_GUROBI_FEATURE}"
fi

bash scripts/phase5_scan.sh
