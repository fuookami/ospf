# Solver Native Validation Matrix

:us: English | :cn: [简体中文](solver-native-matrix_ch.md)

This document defines the executable matrix for the Rust solver contract. It is a
validation procedure, not a generated test report. Native results are valid only
when the command was actually run in an environment with the requested backend
library and license.

## Rules

1. `cargo check` proves feature wiring and compilation only; it does not prove a
   native backend is available.
2. A test that cannot compile or load its native backend is not a pass. Record it as
   `unsupported` when the environment is intentionally absent, or as `failed` when
   the gate was requested.
3. Tests marked `ignored` require `-- --include-ignored`. The no-feature contract
   target is expected to report an explicit ignored test rather than zero tests.
4. Native golden/replay comparisons must include problem status, termination,
   incumbent objective, best bound, gap, solution vector, constraint residuals,
   model/configuration/solver fingerprints, and provenance.

## Matrix

| Area | Command shape | Required interpretation |
| --- | --- | --- |
| Core without backend | `cargo test -p ospf-rust-core --test native_contract_suite` | Must compile and report `ignored/unsupported`; it is not a native pass. |
| Gurobi 10/11/12 shared contract | `cargo test -p ospf-rust-core --test native_contract_suite --features gurobi10` (replace feature as needed) | Requires the matching Gurobi installation and license; native contract and replay tests must execute. |
| SCIP shared contract | `cargo test -p ospf-rust-core --test native_contract_suite --features scip` | Requires SCIP headers, library, and runtime setup; bundled/from-source variants must name their feature explicitly. |
| Native release terminal matrix | `cargo test -p ospf-rust-core --test native_release_matrix --features gurobi10 -- --include-ignored` and the corresponding `scip` command | Executes the release fixtures for limits, cancellation, unboundedness, and incumbent preservation. An ignored test without `--include-ignored` is an environment boundary, not a pass. |
| Gurobi diagnostics and QP | `cargo test -p ospf-rust-core --test gurobi_linear_dual_integration --features gurobi10`, `gurobi_linear_farkas_dual_integration`, `gurobi_iis_integration`, `gurobi_quadratic_model_integration` | Native dual, Farkas, IIS, and QP evidence; each target must execute rather than compile only. |
| SCIP report and observer | `cargo test -p ospf-rust-core --test scip_report_integration --features scip` and `scip_native_observer_integration` | Native report, proof, callback, and observer evidence. |
| Core feature wiring | `cargo check -p ospf-rust-core --no-default-features` plus each backend feature | Compilation evidence only. |
| Framework sync | `cargo test -p ospf-rust-framework --no-default-features` | Exercises report-first framework adapters without a native backend. |
| Framework async/remote | `cargo test -p ospf-rust-framework --features async` and `cargo test -p ospf-rust-framework --features remote-solver` | Exercises cancellation, remote DTO, checkpoint, and identity contracts. |
| Network library gate | `cargo test -p ospf-rust-framework-network-scheduling` plus the selected native feature | Offline tests may pass without a solver; native branch-and-price evidence must be recorded separately. |
| Demo5 native gate | Use the commands in `ospf-rust-example/src/framework/demo5/README.md` with `-- --include-ignored` | Parser/adapter targets do not replace the `--lib` direct-MIP, smoke, and strict-proof gates. |

## Recorded evidence (2026-08-13)

The following commands were executed in the current Windows environment. Native
versions are reported by the backend, not inferred from a Cargo feature name:

| Backend/scope | Result |
| --- | --- |
| No feature: `native_contract_suite`, `native_release_matrix` | Each target reported `1 ignored` without `--include-ignored`; no native pass was claimed. |
| Core/framework offline (`--lib`) | Core no-feature `499/499`; core `serde` `508/508`; framework no-feature `194/194`; framework `async` `159/159`; framework `remote-solver` `236/236`. Older `476/476` and `507/507` counts are historical pre-regression baselines. |
| Gurobi `10.0.1` / `gurobi10` | Shared contract/replay `1/1`; release matrix `9/9` in three consecutive full runs; dual `5/5`; Farkas `2/2`; IIS `4/4`; QP `23/23`; native observer `2/2`; Benders `3/3`. |
| SCIP `9.2.4` / non-bundled `scip` | Shared contract/replay `1/1`; release matrix `10/10` including memory limit; report `2/2`; native observer `3/3`. |
| Network Scheduling | Default `50/50`; `serde` `50/50`; `big-decimal` `53/53`; Gurobi and SCIP library gates `52/52` each. Together with the completed cross-plan contract matrix, this closed Network delivery at `99/99`. |
| Demo5 | Gurobi integration target `3/3`, SCIP integration target `2/2`; the required `--lib` direct-MIP, smoke, and strict-proof gates also executed. |

Gurobi 11 and 12 feature checks compile, but no matching native installation is
present, so their native rows are `unsupported`. The installed Gurobi directory
is `gurobi1001`; running it with `gurobi11` would not be Gurobi 11 evidence.

The `scip-bundled` and `scip-from-source` probes are `unsupported` because the
build could not download the Windows SCIP package: the remote TLS connection was
closed. Non-bundled SCIP `9.2.4` is independent evidence and did execute.

The release fixture covers Gurobi/SCIP node, iteration, time, gap, and solution
limits, cancellation, unboundedness, and incumbent/no-incumbent projections. Every
incumbent MIP-limit fixture also checks a finite best bound, the maximization bound
direction, and the absolute/relative gap formulas against the incumbent and bound.
The Gurobi time-limit-with-incumbent fixture uses a fixed seed and a short non-zero
limit; the complete nine-test matrix passed in three consecutive full runs.
SCIP also covers memory limit. The `grb 3.0.1` status enum does not expose a
Gurobi memory-limit terminal, so that row is explicitly unsupported rather than
silently mapped to another termination reason. Ambiguous `InfOrUnbd` handling is
covered by binding mapping and native disambiguation tests; backends do not claim
an unresolved exact conclusion.

Feature compilation, framework sync/async/remote tests, scoped `-D warnings`
Clippy, scoped formatting, missing-docs rustdoc, and `git diff --check` passed on
2026-08-13. The dependency-inclusive Clippy command still reaches pre-existing
`ospf-rust-base`/`ospf-rust-math` warnings; that command is recorded as a
workspace dependency baseline, not as solver-crate evidence.

See the [unified solve contract](solve-contract.md) and
[source traceability](solver-traceability.md).
