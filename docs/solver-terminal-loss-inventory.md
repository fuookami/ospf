# Solver Terminal-Loss Inventory

:us: English | :cn: [简体中文](solver-terminal-loss-inventory_ch.md)

This inventory records the terminal-state information that was previously lost at
Rust boundaries and the current owner of the replacement contract. It is an audit
map; native-environment rows remain subject to their explicit capability boundary.

| Boundary | Previous loss or ambiguity | Current Rust owner | Regression surface |
| --- | --- | --- | --- |
| Core status | One `SolverStatus` mixed problem status, termination, and execution phase. | `ospf-rust-core/src/solver/report/` with `ProblemStatus`, `TerminationReason`, `SolveReport`, and validated builders. | Core report contract tests and shared fixtures. |
| Typed/value projection | `FeasibleSolution`/typed conversion discarded status, proof, and provenance. | Report-first solver extensions plus explicit legacy projections. | Core solver extension and typed conversion tests. |
| Column Generation | LP/MILP entry points could return only a feasible DTO, making duals look like proof. | Framework column-generation report aggregation and optimal-LP certificate helpers. | Column-generation, serial/parallel, and dual-gate tests. |
| Benders | A solution alone could enter a subproblem or create a cut without a reliable proof. | Master/subproblem report gates, dual/Farkas evidence helpers, and structured stop results. | Linear/quadratic sync/async Benders contract tests. |
| Branch-and-Price | Node bounds and incomplete pricing were not orthogonal to solver termination. | Node conclusion, solver report, pricing-complete flag, inherited/certified bound, and certificate gates. | Gantt and Network branch-node tests; Network cross-plan closure is complete. |
| Combinatorial wrappers | Fallback and First/Best selection compressed attempts and cancellation reasons. | Serial/parallel attempt traces, parent/child identity, completion linearization, loser traces, and cancellation snapshots. | Combinatorial selection, cancellation, and progress tests. |
| Remote boundary | Task lifecycle and solve conclusion were mixed; legacy status fields lost structured metadata. | Versioned report/stop DTOs, stable identities, fingerprints, provenance, and artifact validation. | Remote serializer/client/HTTP tests. |
| Checkpoint/resume | Run, source attempt, parent, provenance, and cancellation origin could be lost. | Portable checkpoint artifact, strict `*_from` resume APIs, object-storage ETag checks, and stop acknowledgement metadata. | Core checkpoint, remote checkpoint chain, and HTTP tests. |

## Explicit remaining boundaries

- Stable `CoreError`/`SolverError` classification is complete and covers input,
  modeling, environment, license, callback, backend, parsing,
  numerical, internal-contract, terminal-projection, and unsupported cases;
  exhaustive regressions make the `Err` versus normal-terminal projection
  boundary explicit.
- The available release rows have recorded Gurobi `10.0.1` and SCIP `9.2.4`
  native evidence. Gurobi 11/12 and
  SCIP bundled/from-source remain explicitly `unsupported`; feature compilation
  is not native evidence. The cross-plan Network final matrix is closed.
- Part III CP/Logic-Based Benders has a versioned remote result materialization path and a
  portable `RebuildFromSnapshot` checkpoint path. Neither path serializes backend pointers or
  claims native search-tree resume; true incremental/native resume remains explicitly
  `Unsupported` in the CP capability matrix.
- The declared Rust CP capability scope is complete (`66/66`). Native library, bundled download,
  or license rows that were not executed remain `not executed/unsupported` and are not counted
  as native passes.
- Network delivery is closed at `99/99`; its long-lived contract and validation
  boundary are maintained in the Network Scheduling README and solver native matrix.
