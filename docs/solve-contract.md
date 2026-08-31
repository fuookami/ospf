# Unified Solve Contract

This document describes the public solver contract shared by `ospf-rust-core` and
`ospf-rust-framework`. The Chinese version is available at
[`solve-contract_ch.md`](./solve-contract_ch.md).

## Delivery Status

The unified solver migration closed on 2026-08-14 with all `201/201` implementation
and acceptance items complete: Part I public execution contracts `42/42`, Part II
Gurobi/SCIP and algorithm terminal-certificate migration `66/66`, Part III
CP/Logic-Based Benders `66/66`, and 27 cross-part acceptance criteria. Gurobi and
SCIP remain the only native backend scope. Capabilities frozen as `Conditional` or
`Unsupported` in the validation matrices are explicit product boundaries, not
unexecuted work reported as native support.

## Report First

New solver-facing code should consume `ospf_rust_core::solver::SolveReport<V>`.
`SolverOutput`, `FeasibleSolution`, `SolveResult`, and `SerializedSolution` remain
compatibility projections and must not be used to infer proof or cancellation
semantics in new algorithm code.

A report separates:

- `problem_status`: `Feasible`, `Unknown`, `Infeasible`, `Unbounded`, or
  `InfeasibleOrUnbounded`;
- `termination_reason`: completion, a backend limit, cancellation, interruption,
  numerical failure, or backend failure;
- `solution`: an incumbent and its objective when one was actually validated;
- `proof`: an optimality, infeasibility, or unboundedness certificate;
- `statistics`, `diagnostics`, `provenance`, `fingerprints`, and algorithm trace.

Progress callbacks use the validated `SolveProgressSnapshot` and stable stage paths.
Known percentages are clamped to `0..=100`; unknown totals are reported as
indeterminate. New code should keep the report and progress paths separate:

```rust
let report = solver.solve_linear_report(&model)?;
if report.is_optimal() {
    consume_verified_report(&report)?;
}
```

The legacy `solve_linear`/`SolverOutput` projection is retained for compatibility;
it does not carry proof, provenance, fingerprint, or cancellation details and must
not be used by new exact algorithm gates.

Reports are built through the validated builder. A feasible report requires an
incumbent, an infeasible or unbounded report cannot carry one, and a verified
optimality proof requires completed termination and a reliable complete proof.
Gap fields are accepted only when both incumbent objective and best bound are
present and consistent.

## Error Boundary

`SolverErrorClass` is the stable classification boundary for failures:
`INPUT`, `MODELING`, `ENVIRONMENT`, `LICENSE`, `CALLBACK`, `BACKEND`, `PARSING`,
`NUMERICAL`, `INTERNAL_CONTRACT`, `TERMINAL_PROJECTION`, and `UNSUPPORTED`.
Native Gurobi/SCIP execution failures remain `BACKEND` errors. Model assembly,
invalid reports, callback failures, malformed artifacts, and runtime misuse keep
their own classes. A normal terminal state such as cancellation is represented by
`SolveReport`; legacy methods may expose it as `SolverError::Cancelled`, which is
a terminal projection and not a backend failure.

## Certificates

Exact algorithms must use the certificate helpers in
`ospf_rust_core::solver`:

- optimal LP consumers require a completed feasible report with a verified
  optimality proof, matching model fingerprint, dual vector, dimension, objective,
  and residual checks;
- infeasibility consumers require a completed infeasible report with a verified
  complete proof and matching model fingerprint;
- IIS and Farkas diagnostics are evidence attached to a report. Diagnostic failure
  does not replace an already established mathematical conclusion.

An incumbent returned at a limit or interruption is usable as a candidate, but it
does not close an exact bound or create an optimality certificate.

## Cancellation And Async Execution

Each solve owns an independent `SolveHandle`. Cancellation is idempotent, records
the first `CancellationOrigin` and timestamp, and invokes every registered backend
interrupter. `spawn_solve_report_with_options` runs blocking work on Tokio's
blocking pool. Its `cancel`/`abort` methods request backend interruption first;
`cancel_and_wait` must be used when the caller needs resource-release completion.

Combinatorial wrappers preserve cancellation in the child attempt and keep loser
attempts in the final trace. A cancellation that arrives after an attempt's
`completedAt` linearization point must not rewrite a completed report.

## Identity And Replay

Reports carry stable model-element mappings and separate model, effective
configuration, and solver-environment fingerprints. Fingerprints use deterministic
canonical encodings, sorted sparse entries, explicit numeric tags, and SHA-256
domain separation. Object addresses, process-global IDs, default hash iteration,
callback addresses, and callback debug text are not replay identity.

Callbacks are execution behavior, not replayable configuration. Backend provenance
records callback presence and reports add a `NonReplayableCallback` warning when a
callback is registered.

## Remote Boundary

The remote report DTO preserves the report schema, run/attempt identity, artifact
digest, provenance, fingerprints, diagnostics, proof, incumbent, bound, and pool.
Task lifecycle is separate from the mathematical solve conclusion. Unknown future
report schema versions, identity mismatches, and artifact mismatches are rejected
structurally. `stop` returns `StopAcknowledgement`; the boolean `stop_legacy`
facade exists only for compatibility.

Portable checkpoint artifacts use `RemoteCheckpointArtifactDto` with schema
`1.0`. `checkpoint_artifact_to_json` and `checkpoint_artifact_from_json` validate
the state digest and schema. `store_checkpoint_artifact` and
`load_checkpoint_artifact` provide the object-storage boundary; loading rejects a
missing object, a mismatching ETag, an invalid artifact, or a run/model/config/
solver fingerprint mismatch supplied through `CheckpointResumeExpectation`.
That type and the legacy `validate_resume` entry are compatibility projections and
do not establish the source attempt. Exact recovery uses
`CheckpointResumeExpectationWithAttempt`, `load_checkpoint_artifact_from`, and
`validate_resume_from`; these validate the source attempt, parent link, preserved
provenance, matching fingerprints, and the prefix of the cancellation chain.
The HTTP execution port performs artifact identity validation before sending the
server resume request, so a rejected artifact cannot start a remote attempt.
`StopAcknowledgement` carries the same run/attempt identity, fingerprint,
provenance, cancellation origin, and message metadata when the remote action
provides them. A child attempt is a new attempt with an explicit parent; it must
not overwrite the source checkpoint identity.

## Backend Scope

This migration's exact backend scope is Gurobi and SCIP. Cargo feature presence
does not prove that a native library or license is available. Runtime descriptors
therefore report conditional capabilities until a backend probe succeeds. Missing
native environments are recorded as ignored or unsupported in the validation
matrix and are never counted as passed.

Native backend errors are classified separately from ordinary backend execution
failures: Gurobi license errors (including native code `10009`) use the stable
`LICENSE` class, while missing libraries and environment setup remain
`ENVIRONMENT`.

See also:

- [`ospf-rust-core` README](../ospf-rust-core/README.md)
- [`ospf-rust-framework` README](../ospf-rust-framework/README.md)
- [native validation matrix](./solver-native-matrix.md)
- [source traceability](./solver-traceability.md)
- [CP native capability matrix](./constraint-programming-native-matrix.md)
