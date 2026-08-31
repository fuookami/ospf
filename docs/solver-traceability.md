# Solver Source Traceability

:us: English | :cn: [简体中文](solver-traceability_ch.md)

This document is the final source traceability record for the eleven Kotlin
solver-contract commits in the completed unified solver migration. The source repository is
`E:\workspace\ospf\ospf-kotlin`; the Rust repository is the current workspace.

The `Files` column gives the exact file count recorded by `git show` and the
command that reproduces the complete commit manifest. The following list is
the complete solver-relevant subset of that manifest. Files outside that
subset are domain documentation, benchmark/example plumbing, or CP-specific
work and are covered by the boundary matrix below.

| # | Kotlin commit | Date and subject | Files | Solver-relevant Kotlin files | Rust implementation and tests | Final net behavior | Disposition |
| --- | --- | --- | ---: | --- | --- | --- | --- |
| 1 | `b8d67c96be2d29e6477838adbbb3ee6fec27ddf5` | 2026-07-28, `feat(solver): add unified solve reporting and progress contracts` | 29 | `ospf-kotlin-core/src/main/.../solver/LinearSolver.kt`; `QuadraticSolver.kt`; `SolveOptions.kt`; `config/SolverConfig.kt`; `progress/ProgressAdapters.kt`; `progress/SolverProgress.kt`; `report/NormalizedModel.kt`; `report/OfflineExperiment.kt`; `report/SolveReport.kt`; matching core progress/report tests; Gurobi/SCIP linear and quadratic adapters; framework `ColumnGenerationSolver.kt`, serial/parallel combinatorial solvers, remote linear/quadratic clients and `RemoteReportMapping.kt` | `ospf-rust-core/src/solver/report/mod.rs`, `progress.rs`, `solver.rs`, `solver_ext.rs`; `ospf-rust-framework/src/solver/column_generation_solver.rs`, serial/parallel wrappers and remote serializer; core/framework report and progress fixtures | Introduced one validated report/progress contract and the first report-first adapter paths. | Implemented for Gurobi/SCIP and framework scope; non-Gurobi/SCIP adapters are excluded. |
| 2 | `5f61674788ae9543c4769b1eacbc74d24296012b` | 2026-07-29, `fix(solver): propagate termination status through all adapters` | 24 | COPT `CoptLinearSolver.kt`/`CoptQuadraticSolver.kt`; CPLEX equivalents; Hexaly equivalents; MindOPT equivalents; SCIP column-generation adapter; network status/algorithm/tests; framework Benders/ColumnGeneration/remote clients and tests | `ospf-rust-core/src/solver/report/mod.rs`, `solver.rs`; framework report gates, fallback policies and remote legacy projection tests | Preserved terminal reason, incumbent presence and fallback behavior across adapter boundaries. | COPT, CPLEX, Hexaly and MindOPT source behavior is recorded as excluded; Rust acceptance covers only the declared Gurobi/SCIP backend scope. |
| 3 | `25bcb176ebe3c4380f84eac6c3777f930f9ba47e` | 2026-07-29, `fix(solver): enforce terminal-state contracts in Benders and branch-and-price` | 11 | example Benders solver; network `BranchNodeSolver.kt`; framework `ColumnGenerationSolver.kt`, solver READMEs and value-conversion tests | `ospf-rust-framework/src/solver/linear_benders_decomposition_solver.rs`, `column_generation_solver.rs`, `core_extensions.rs`; solver contract fixtures | Exact master/subproblem gates reject incomplete proofs and preserve structured terminal states. | Part II algorithm behavior implemented; CP remains Part III. |
| 4 | `32f7d5aa76fb9f7f5982d856497b480bbf7f3b3f` | 2026-07-29, `fix(solver): preserve Gurobi LP infeasibility through branch-and-price` | 9 | Gurobi/Gurobi11/SCIP column-generation adapters; network B&P regression; example Benders terminal test | Gurobi/SCIP report and infeasibility certificate mapping; framework Benders/column-generation certificate gates and tests | Verified LP infeasibility is retained as a mathematical terminal and is not rewritten as node/backend failure. | Gurobi/SCIP only; CP plan files are not implemented by this row. |
| 5 | `e0bca1eb4d04e99fa8048deb9b2bb1731a1ac6b4` | 2026-08-09, `feat(contract): complete solve report identity and remote adapter migration` | 196 | Core `model/intermediate/*`; `solver/CoreSolverAsync.kt`, `LinearSolver.kt`, `QuadraticSolver.kt`, `ModelingPreparation.kt`, `SolverExt.kt`, `report/*`, `output/SolverOutput.kt`, `value/*`; Gurobi/SCIP solver, Benders and column-generation adapters/tests; framework Benders/ColumnGeneration/combinatorial/remote sources and tests; remote serialized-model fixture | `ospf-rust-core/src/solver/audit.rs`, `fingerprint/`, `report/`, `checkpoint.rs`; `ospf-rust-framework/src/solver/remote/domain.rs`, `ospf_serializer.rs`, `client.rs`; stable identity, fingerprint, checkpoint and remote tests | Stable model identity, report provenance/fingerprints, remote schema validation, artifact integrity and cancellation/checkpoint identity are retained. | CPLEX, COPT, Hexaly, MindOPT, MOSEK and CP implementation files are excluded from Rust backend acceptance; CP-specific checkpoint delivery is tracked separately in Part III and is complete there. |
| 6 | `4efac629a57571497695352cf8448980be4e418b` | 2026-08-09, `fix(solver-report): complete capability, attempt trace, and provenance metadata` | 23 | Gurobi/SCIP runtime solver metadata and status tests; core identity/provenance sources/tests; framework combinatorial and remote clients/tests | `ospf-rust-core/src/solver/solver.rs`, `report/mod.rs`, `fingerprint/`; framework attempt aggregation, provenance and remote metadata paths | Runtime capabilities, effective configuration, provenance and attempt identity are explicit and deterministic within the declared backend scope. | Implemented for Gurobi/SCIP and backend-neutral framework; other native backends remain excluded. |
| 7 | `589307646757d7f43afda299b866b2cfcf874ac2` | 2026-08-09, `fix(solver-report): preserve aggregate provenance and attempt traces` | 15 | Core intermediate identity/report sources and tests; framework `CombinatorialSolveSupport.kt`, serial/parallel combinatorial solvers and identity/selection tests | `ospf-rust-framework/src/solver/column_generation_solver.rs`, serial/parallel combinatorial linear/quadratic and column-generation wrappers; aggregation tests | Aggregate reports retain parent/child attempt identity, provenance, selected attempt and deterministic trace order. | Implemented in framework; no CP or new backend is implied. |
| 8 | `e5089f1886b0fb924b8f721966cf1bb511be7395` | 2026-08-10, `fix(solver-report): close aggregate identity and attempt cancellation boundaries` | 9 | Core identity propagation tests; framework serial/parallel combinatorial solvers and identity tests | Framework cancellation linearization, completion snapshots and aggregate validation tests | A late shared cancellation cannot overwrite a backend-completed result; aggregate identity remains stable. | Implemented in framework. |
| 9 | `b9db32a86af51e8ea976b81c2c15cbc3126dd006` | 2026-08-10, `fix(solver-report): preserve in-flight cancellation reasons` | 6 | Framework combinatorial support and serial/parallel combinatorial solvers/tests | `ospf-rust-framework/src/solver/column_generation_solver.rs`, serial/parallel wrappers and cancellation tests | In-flight cancellation origin is preserved in the child attempt and aggregate report. | Implemented in framework. |
| 10 | `ae0b01fbb516a4fbd834adda5643b9a16d4b8041` | 2026-08-10, `fix(solver-report): freeze combinatorial cancellation metadata at backend completion` | 6 | Framework combinatorial support and serial/parallel combinatorial solvers; parallel selection tests | Framework completion marker and stop-condition tests | Cancellation metadata is frozen at the backend completion linearization point. | Implemented in framework. |
| 11 | `b5b83d7d6f470c363e1044cd6b0266604ad5aaa1` | 2026-08-10, `test(solver-report): cover parallel cancellation reason propagation` | 10 | Core report identity source; framework combinatorial support and parallel selection test; deleted Kotlin CP plan files and updated release/solver CP plans | Framework parallel cancellation regression and shared report fixture | Parallel loser cancellation is observable and does not change the selected completed attempt. | Solver behavior implemented; deleted/CP-only Kotlin plan files are explicitly excluded from Rust delivery. |

## Complete manifests

For each row, the exact complete file list, including non-solver files, is
reproducible from the immutable commit hash with:

```text
git -C E:\workspace\ospf\ospf-kotlin show --no-renames --format="COMMIT %H%nSUBJECT %s%nDATE %ad" --date=short --name-status <full-hash>
```

The recorded manifest counts are, in row order: `29, 24, 11, 9, 196,
23, 15, 9, 6, 6, 10`. This prevents a shortened display hash or a filtered
table from being mistaken for the source manifest.

The immutable full hashes and the reproduction command above are the
version-controlled manifest evidence. The `git show` output is generated
verification material and is intentionally not committed.

## Rust ownership and exclusion matrix

| Source area | Rust disposition | Evidence boundary |
| --- | --- | --- |
| Core report, proof, diagnostics, identity, fingerprint, progress, cancellation and checkpoint | Implemented in `ospf-rust-core` | Fake contract fixtures, report/proof tests and feature checks |
| Gurobi | Implemented and feature-gated | Native/shared contract tests when Gurobi is available; otherwise explicitly unsupported |
| SCIP | Implemented and feature-gated | Native/shared contract tests when SCIP is available; otherwise explicitly unsupported |
| Framework serial/parallel, Benders, column generation and remote | Implemented | Framework solver tests and remote round-trip/legacy tests |
| Network/Gantt consumers | Consumer integration only | Network is owned by the crate README and closed at `99/99`; Gantt remains owned by its domain README and tests. |
| CPLEX, COPT, MindOPT, Hexaly, MOSEK | Excluded | No Rust adapter, placeholder capability or acceptance claim |
| PSO | Existing unrelated heuristic backend | Not counted as a native solver-contract backend |
| CP, Logic-Based Benders and portable checkpoint rebuild | Part III Rust delivery | The declared CP scope is complete (`66/66`); exact lowering, verified conflict, remote materialization, portable rebuild, and Gantt differential are covered by their own CP sources/tests. Native enhancements remain explicitly `Conditional`/`Unsupported`, not falsely marked as Native. |

## Compatibility evidence

The compatibility window is exercised by the following source-level regression
targets in addition to the in-module report tests:

```text
cargo test -p ospf-rust-core --test solver_legacy_api_compat
cargo test -p ospf-rust-framework --test solver_legacy_api_compat
cargo test -p ospf-rust-framework --features remote-solver --test solver_remote_legacy_api_compat
```

The core target verifies legacy solver traits, deprecated `solver_output` and
`solver_config` paths, typed conversion, and the loss-aware
`SolveReport -> SolverOutput -> SolveReport` projection. The framework target
verifies `FeasibleSolution` and `LPResult` round trips, including rejection of
non-certifying LP reports. The remote target verifies legacy
`SerializedSolution` JSON, infeasible/unbounded helpers, report projection,
unknown report schema rejection, and the `stop_legacy` boolean facade.

These tests preserve the legacy read/forwarding window without allowing new
algorithm code to extend the legacy terminal model; new code consumes
`SolveReport` and its certificate gates.

## Audit rules

- A source commit is mapped to implementation, test, or an explicit exclusion;
  a source diff alone is not completion evidence.
- Full manifests are reproducible from the immutable hashes above; the table's
  solver subset is not a claim that non-solver files were ignored. Generated
  manifest output is intentionally excluded from version control.
- Native rows follow [`solver-native-matrix.md`](solver-native-matrix.md);
  compile-only checks are not native evidence.
- This document is the authoritative source-migration status and traceability
  record. Public behavior is defined by [`solve-contract.md`](solve-contract.md),
  and Network delivery/validation is defined by the
  [Network Scheduling README](../ospf-rust-framework-network-scheduling/README.md).
