# OSPF Rust Shared BPP

[简体中文](README_ch.md)

Dimension-independent sparse discrete loading. `DiscreteLoadingContext` registers assignments,
whole-position occupancy and optional layout choices into `MetaModel<V>`. Quantity weights are
normalized once through an explicit `PackingValueAdapter`; native values stay generic until an
explicit solver boundary. Item counts remain `u64` and model count coefficients are checked for
exact conversion.

`register_with` atomically combines common limits with downstream variables, constraints and
objectives. A context belongs to one model and cannot be registered twice. Use a fresh model and
context for a retry/fallback. `LoadingCapacityLimit` uses true finite count/weight bounds, never an
arbitrary epsilon or big M. Occupancy may reserve an empty position. Fixed cargo, external fixed
occupancy and any strictly positive additional weight require the entire position to be occupied.
`link_additional_weight` accepts one combined expression per position and stores it for extraction.

Conflict edges are non-transitive. `ConflictIndex` greedily builds actual cliques while preserving
every original edge. Layout selection exists only for explicit groups. `loading_conflicts` sweeps
longitudinal intervals, keeps physical spaces separate, excludes touching boundaries and optionally
delegates exact geometry intersection to an adapter. It never introduces a BPP1D/BPP2D dependency.

`extract_solution` validates the complete expanded model, including extension rows, before
independently checking required/fixed loads, sparse assignment legality, count/weight capacities,
occupancy, conflict edges and layout membership. `verify` is also available for standalone domain
validation. Generic column projection belongs to `ospf-rust-framework`, not a duplicate BPP model.

`PartitionLoadingContext` requires exhaustive disjoint position ownership and explicit downstream
local decisions. It adds cargo uniqueness, local pieces of cross-partition cliques and compatible
layout choices to generic pricing. Complete configuration columns carry validated domain metadata.
Whole-round preparation precedes atomic projection/metadata commit. Dynamic source rows invalidate
old registrations and can reject old columns rather than silently ignoring new business rules.

`DiscreteLoadingApplicationService` supplies ordinary same-domain f64 MILP orchestration from a
complete registered source model. `PartitionColumnGenerationApplicationService` supplies independent
Phase-I/root pricing, whole-round batches and an artificial-free restricted integer final. Actual
LP primal/dual certificates and model mappings are checked through the shared projection. Only
explicitly owned, whitelisted resources can be released inside frozen assignment and
occupancy profiles. Completion preserves every source row/bound and excludes old resource row and
objective contributions; complete new columns are materialized before final. Root and completion
have separate column caps and share a total deadline/cancellation handle.
`contains_configuration` checks candidates against the same normalized domain metadata without
allocating a column ID. Duplicate pricing candidates do not spend the new-column budget, so an
earlier partition cannot hide a genuinely new column in a later partition at the cap.
Completion restores integer resource types as well as all global/helper integers; fractional
integer resources fail full-vector validation before any completed column is committed.
The main CG report certifies source feasibility only; actual `refinement_report` and `final_report`
stay separate. A validated completion incumbent survives deadline/cancellation on return without
starting final. Completion/final selection keeps the best validated source incumbent.

`DiscreteLoadingBendersApplicationService` independently runs the shared fixed-recourse algorithm.
All expanded integers/helpers and, by default, every objective variable remain in master. Explicit
continuous objective recourse requires a finite lower bound derived from the real variable bounds.
All recourse bounds become explicit subproblem rows, preserving their dual/Farkas cut contributions.
Only independently verified certificates from the actual current LP generate cuts. Missing/invalid
evidence, ambiguous/infeasible/unbounded terminals, cancellation and exhausted budgets stay distinct.
Optional ordinary-MILP fallback uses a fresh equivalent factory and the remaining total time only;
invalid input, cancellation and exhausted time never launch fallback. No model, context, pool or
dual credentials are shared with fallback. Preserve the returned decomposition reports for audit.
Structured unsupported-capability, environment/license, backend and numerical errors may trigger
the same explicit fallback policy. Input, modeling, contract and callback errors propagate without
fallback. The original recoverable error is retained in `decomposition_error`. Limited/cancelled
reports keep any fully validated source incumbent, but never continue solves or generate new cuts.
Fallback never replaces a better decomposition incumbent with an empty or worse result. The selected
plan is extracted against its own source registration, while `fallback_report` retains the actual
fresh solve; no old tokens or restricted-master proofs are transported into the fresh model.

`DiscreteLoadingCertificationService` finishes an accelerator result on a fresh equivalent
complete-source MILP within an explicit remaining budget. It validates the old incumbent and
source identity, checks the new numeric model and domain bindings, and supplies only feasible
values as a warm start. It preserves the best plan on an empty/worse limited result or recoverable
backend error. Actual `certification_report` and `certification_error` stay separate; contradictory
proofs and contract errors are rejected. Cancellation/interruption or an exhausted budget cannot
start another solve. Callers must reserve finishing time; Demo2 does so explicitly. Only actual
complete-source proofs certify source optimality. `validate` checks the original registration,
source identity, complete vector and objective without creating a fresh model, invoking a backend
or rewriting termination. Callers independently choose whether to finish and reserve time when
enabled. Demo2 exposes `source_milp_finishing`; disabling it still retains validation.
After a source incumbent exists, recoverable CG-final, certificate-recovery or Benders-fallback
failures preserve it alongside `acceleration_error`, `backend_error` or `decomposition_error`.
Replacing a worse finishing incumbent rebuilds its row/bound audit and clears obsolete trace
summaries. A true complete-source Unbounded terminal is never overwritten by a finite plan.
Already verified complete-source terminal proofs are returned unchanged, even when the new
budget is zero or cancellation is requested; their Completed proof terminal is never rewritten.

Restricted-master optimality/infeasibility is not branch-and-price or an original-domain proof.
Benders merged source reports conservatively remain Feasible even when the verified decomposition
converges; the actual master/sub reports and certified bound remain separate. All application APIs
currently use explicit same-domain f64 callbacks, independently of the framework async feature.
Cross-domain exact Benders is not accepted. BPP1D and BPP2D supply their own shared-domain geometry
adapters without depending on one another. [Demo2](../ospf-rust-example/src/framework/demo2/README.md)
uses these shared services for its three aircraft-loading paths, while keeping aircraft-specific
rules and extraction in its own domain contexts.
