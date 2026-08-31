# CP Native Capability Matrix

This document records the evidence boundary for the Rust CP work. It is deliberately
separate from the MIP-backed CP facade and from the unified solver report contract.

## Binding and version

The first implementation targets `russcip 0.9.1` and the `scip-sys 0.1.26` bindings.
The probe test is `ospf-rust-core/tests/scip_cp_probe_integration.rs` and must be run
with an explicit `scip`, `scip-bundled`, or `scip-from-source` feature. A compile-only
feature check is not native evidence.

| Capability | Evidence | Effective status |
| --- | --- | --- |
| Safe linear integer/binary variables | `russcip` model API and probe | Native for the probe subset |
| Indicator constraints | safe `add_cons_indicator` and probe | Native for positive-literal `<=` rows |
| SOS1 | safe `add_cons_sos1` and probe | Native |
| Status, incumbent and best bound | `Status`, `WithSolutions`, `WithSolvingStats` and probe | Native metadata |
| Cancellation | project `SolveHandle` + SCIP event bridge and probe | Native interrupt bridge |
| Event handler | `Eventhdlr` probe | Native, solve-scoped |
| Probing | `Prober` probe and `SCIPinProbing` postcondition | Native, temporary solve/node scope |
| Negative literals, Boolean AND/OR/XOR, full reification | CP AST evaluator, exhaustive formulation oracle, and strict finite lowerer | ExactLowering; not claimed as SCIP native |
| Sparse domains, AllDifferent, Element, Allowed/Forbidden tables | CP snapshot validation and exhaustive lowerer tests | ExactLowering for finite, budgeted formulations |
| Mandatory/optional and fixed/variable-duration NoOverlap | strict finite lowerer, source snapshot revalidation, and Gurobi/SCIP facade tests | ExactLowering when all bounds are finite |
| Circuit, Automaton, Reservoir | per-constraint support analysis and structured lowering error | Unsupported in generic production lowering |
| True incremental CP session | no public `russcip 0.9.1` contract | Unsupported; use snapshot rebuild |
| Optional/variable-duration native interval | no safe binding contract | Unsupported |
| Cumulative | raw C API probe only | Conditional research path, not production |
| Native conflict graph/IIS for CP | no CP-level public contract used by this crate | Unsupported; verified rebuild/deletion fallback |
| Unified report/proof/cancellation/provenance | `ScipConstraintProgrammingSolver` and shared `SolveReport<i64>` contract tests | ExactLowering facade; native CP is not claimed |

## CP delivery boundary

The Rust CP declaration scope is complete when the capability is either implemented and
verified or explicitly frozen as `Conditional`/`Unsupported`. The production SCIP entry point
is therefore the exact finite MIP-backed facade, not a general native SCIP/CIP CP engine.
Backend probes, exact lowering, verified conflict reconstruction, remote materialization,
portable checkpoint rebuilding, and the Gantt differential are covered by their dedicated
sources and tests. A missing native library, license, bundled
download, or source build is recorded as `not executed/unsupported`; it never upgrades a
compile-only result to Native.

## Raw cumulative safety boundary

The probe calls `SCIPcreateConsBasicCumulative` and `SCIPaddCons` in one scope. The
arrays are writable only for the duration of the call, and the SCIP variables remain
owned by the model. After adding the original constraint, the model owner performs
the final release; the probe deliberately does not call `SCIPreleaseCons` because
`russcip 0.9.1` may already have an associated transformed constraint and a second
release is rejected by SCIP. The raw route is version-gated by the `scip-sys`
dependency and is not used by the production CP facade until a dedicated wrapper
adds return-code conversion, flag configuration, ownership handling, overflow/horizon
checks, and success/failure tests.

The fallback remains strict MIP lowering for the exact supported subset, while
Cumulative/Circuit/Automaton/Reservoir remain structured `Unsupported` in the generic
MIP lowerer. No raw probe result is reported as a native production capability.

## Scope of probing

An event handler may create a temporary probing object while SCIP is in `Solving`.
Dropping the object ends probing and the test checks `SCIPinProbing == 0`. Probing
changes are local to that search scope and are not a reusable incremental model or
assumption session. CP sessions therefore continue to use snapshot rebuilds.
