# Gurobi Solver Notes

:us: English | :cn: [简体中文](README_ch.md)

## Prerequisites

`ospf-rust-core` Gurobi integration requires:

1. Gurobi installed on the machine.
2. A valid Gurobi license.
3. One of these cargo features enabled:
- `gurobi10`
- `gurobi11`
- `gurobi12`

## Key Capabilities

1. LP/MIP/QP/MIQP solving.
2. Stage callback and telemetry callback.
3. Native callback and native observers.
4. Numeric diagnostics and numeric profile recommendation.

## Native Callback Semantics

1. `add_native_callback` uses override semantics (latest wins).
2. `add_native_observer` uses append/multicast semantics.
3. Native observer can return `Terminate` to request solve termination.

## Minimal Validation Commands

From workspace root:

```bash
cargo test -p ospf-rust-core gurobi_native_observer_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_telemetry_callback_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_stage_callback_integration --features gurobi10 -- --nocapture
```

This workspace is validated against the local Gurobi 10 environment. If your environment is pinned
to Gurobi 11/12, replace `gurobi10` with `gurobi11`/`gurobi12`.
