# OSPF BPP1D

[简体中文](README_ch.md)

Quantity-aware one-dimensional exact-demand bin packing. Length and weight have
independent unit parameters. `PackingContext<V, L, W>` normalizes quantities through
explicit adapters and retains the original numeric domain, including BigDecimal
and BigRational. Item identities are never merged by dimension equality.

## Domain And Registration

- `PackingProblem`, `Item`, `BinType`, `PackingPattern`: exact `u64` demand and
  optional inventory, length/weight capacities, and inter-piece gaps without a
  trailing gap. Pattern arithmetic does not expand large counts.
- `generate_patterns`: bounded iterative enumeration with explicit `complete`
  and node counts. `initial_patterns` supplies FFD/BFD seeds under a separate
  expansion cap; seeds do not prove candidate completeness or stock feasibility.
- `PatternAssignmentContext`: one registration path for MILP, LP and final.
  Every expanded integer, including extension helpers, is relaxed for LP while
  retaining explicit and implicit bounds. Final restores integer domains.
- `PatternModelingExtension`: append variables, rows, objectives and extracted
  indicators. It is a pure, reentrant model-building contract: no old column
  indices or externally shared mutable state. Adding/removing columns prepares a
  complete fresh model before swapping model, pool and aggregation atomically.
- `pack` / `verify_plans`: independently check exact demand, inventory, actual
  coordinates, gaps, weight, dimensions, units and materials. Free intervals and
  native-domain packed/consumed length are retained for reporting.

## Applications And Evidence

`Bpp1dMilp` and `Bpp1dColumnGeneration` currently expose an explicit **same-domain
f64** solver boundary. Generic contexts do not silently send rounded values to a
backend. Solver callbacks receive the actual `LinearTriadModel`, remaining total
budget and cancellation handle, and return the existing Core `SolveReport`.

The finite-pool CG implementation keeps the full candidate/extension matrix and
temporarily fixes inactive pattern variables to zero. This deliberately trades
matrix size for safe pricing of arbitrary extension rows and helper variables;
it is not an unbounded pattern oracle. Each LP report must pass independent
optimal primal/dual validation and match the actual model mapping/fingerprint.
No raw externally supplied shadow-price map is accepted. An inventory-infeasible
seed LP can proceed to final but never establishes pricing convergence.

Final always uses the full deduplicated **generated + seed** pool, in a new model
without artificials. Raw reports certify only that actual candidate model.
`globally_optimal` additionally requires complete enumeration, integer proof and
default domain rules. Custom policies/extensions conservatively retain validated
feasibility, without claiming their cross-stage equivalence. Restricted-pool
infeasibility is not promoted to domain infeasibility. Objective constants remain
outside backend `c*x` and are retained once in the source snapshot.

`optimize_tail` preserves prefix bins and coordinates, subtracts prefix demand
and inventory, validates both local and complete plans, and invokes the full-plan
acceptance policy. Global model extensions must not be reused as local rules.

## Boundaries

`load_packing_csv` uses a caller-supplied numeric parser and explicitly declared
units. Closed schemas are `id,length,weight,amount` and
`id,length,max_weight,cost,inventory`; blank inventory means unlimited. Malformed
quotes, duplicate/unknown headers, row-width errors and count overflow fail.

`Bpp1dDiscreteLoading` adapts one explicit interval per position to shared BPP
loading, preserving explicit edges, separating spaces and excluding boundary
touches. It has no BPP2D dependency and does not infer layout selection.

Features forward `serde`, `async`, high-precision and native backend support.
The callback orchestration API is synchronous independently of the framework's
`async` feature; it does not block on an async runtime internally. Asynchronous
solver callbacks are not provided yet. `kpi` recomputes native-domain indicators
from validated placements. `PackingRendererAdapter` revalidates placements and
uses explicit quantity and scalar conversions to produce `RenderSchema1d`;
display values are never optimization evidence. Render DTOs support `serde`.
