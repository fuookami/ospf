# Demo 5: VRPTW Branch-and-Price

[简体中文](README_ch.md)

`framework:demo5` is a small inline Solomon-style fixture wired to the Rust network-scheduling crate. It exercises parsing, input adaptation, Phase I/II route compilation, ESPPRC pricing, and the application Branch-and-Price flow without changing the existing `core:demo5` knapsack example.

Run a backend-gated build or execution with one of:

```powershell
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-scip-bp --test demo5_scip_bp -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored"
pwsh -NoProfile -Command "cargo run -p ospf-rust-example --features demo5-scip-bp -- framework:demo5"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --test demo5_gurobi_bp -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored"
pwsh -NoProfile -Command "cargo run -p ospf-rust-example --features demo5-gurobi-bp -- framework:demo5"
```

The parser accepts standard Solomon `VEHICLE` and `CUSTOMER` sections, validates seven-column node rows, preserves the final row, and reports bilingual line-aware errors. The Demo17 fixture exposes 25- and 100-customer instances. The integration targets cover their own parser/adapter and target harness; the `--lib` commands above are required for the direct-MIP cross-check, 100-customer smoke, strict-proof fixture, and SCIP legal-terminal validator gate. The direct-MIP oracle currently supports one vehicle type. Native solver tests are ignored by default and must be explicitly run with `-- --include-ignored`; a missing local solver or license fails the gate instead of silently passing.
