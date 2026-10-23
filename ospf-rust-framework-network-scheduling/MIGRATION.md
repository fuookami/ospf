# Shared Network / Gantt Kernel Migration

[简体中文](MIGRATION_ch.md)

This increment targets Kotlin `907d28d` and `3dc1225`. It includes shared foundations,
extended Flow, generalized flow, design, explicit resilience, rooted structures, allocation,
finite whole-path masters, seven standalone services and the optional bridge. Shared paths,
task/bunch batches, extended-row pricing matrices and Demo4's Gantt loop are integrated.
Overall migration acceptance remains incomplete; see current proof boundaries and regression requirements below.

## Flow and Generalized Flow

- `FlowGraph` and `SupplyDemand` become `FlowProblem` and `FlowCommodity`. Problem
  fields are externally immutable; use `nodes()`, `arcs()`, `commodities()` and
  `units()`. Context and aggregation expose `problem()` instead of mutable state.
- Commodities declare independent units, allowed arcs, tariff overrides, explicit
  arc-capacity consumption and optional integer steps. Integer variables count
  physical steps, not indivisible orders. Conservation, capacity, resources and
  costs all use the same scaling.
- Registration prepares and validates all step-scaled capacity/resource/cost
  coefficients and derived variable bounds before model mutation, even when
  `minimize_cost` is false. Non-finite arithmetic and zero-underflow capacity
  scaling fail transactionally; pipelines and downstream expressions reuse the
  checked coefficients rather than recomputing unchecked products.
- `with_resources` accepts explicit shared usage; `with_node_capacities` compiles
  incoming/outgoing processing limits through the same resource pipeline.
  Cross-dimensional commodity capacity consumption requires an explicit mapping.
- Balances are hard by default. `FlowBalanceSlack` explicitly adds finite spill
  and shortage with nonnegative penalties. Extraction returns a validated
  `FlowSolution` with physical flows, resources and independent operating/spill/
  shortage/total costs. Unbalanced hard supplies remain model infeasibility.
- `GeneralizedFlowProblem` declares node measurement and hard net supply; arcs
  declare finite origin input bounds, input tariffs, dimensionless business yield
  and optional unit/process conversion. Cross-dimensional conversion is explicit.
  Transactional contexts expose input, output, net-supply and cost expressions to
  downstream `GeneralizedFlowPipeline` extensions.
- Public `GeneralizedFlowSolverTolerance` defaults to `1e-7` absolute and relative.
  Production extraction checks missing/negative/non-finite values, capacities,
  actual net-supply residuals, all derived arithmetic and return conversions.
  Large gross circulation does not inflate tolerance for a small net balance.
- `FlowApplicationService` and `GeneralizedFlowApplicationService` create fresh
  contexts and return existing `SolveReport<DomainSolution>` values. Generic
  `NetworkApplicationService` accepts a context factory and `SolveOptions`, audits
  the complete incumbent before domain extraction, and preserves ordinary
  no-incumbent limit/cancellation/infeasibility reports. Domain reports contain no
  pricing duals; numeric audits remain in `solver.*` diagnostic extensions with
  an explicit warning, and cannot be replayed as LP pricing certificates.

## Design and Explicit Resilience

- `NetworkDesignProblem` validates references, levels, costs and budget before any
  model mutation. Capacity levels are mutually exclusive total capacities bounded
  by the finite nominal envelope. `NetworkConstruction` registers construction
  only, without implicit nominal flow, for reuse by structure models.
- Design combines construction with the existing Flow kernel. `FlowRegistration`
  optionally delegates operating cost to an outer aggregation; `with_name` isolates
  scenario namespaces. Repeated registration must use identical options. Flow
  exposes capacity, resource and operating-cost expressions for downstream rules.
- Explicit `FailureScenario` instances share construction but own independent
  flows and optional balance/slack replacements. Failed-node demand is not waived.
  Built capacity, scenario degradation and facility gating remain independent
  constraints. A legal degradation below an enabled hard lower bound registers
  normally as an infeasible model, not an invalid capacity interval. Negative
  degradation and increases above nominal capacity remain input errors.
- The default objective minimizes construction plus worst operating cost. Results
  distinguish actual `worst_operating_cost` from `operating_cost_epigraph`; a
  feasible nonoptimal incumbent may have slack in its epigraph.
- Auxiliary connectivity supports reachability, edge-disjoint and internally
  node-disjoint paths, preserves parallel arcs and excludes self-loop witnesses.
  It certifies nominal construction structure, never failure-demand capacity.
- Both contexts offer transactional registration, model-binding checks, independent
  production extraction and downstream pipelines. Standalone design and resilience
  services share these registrations and return the unified solve report.

## Rooted Structures and Facility Ownership

- `NetworkStructureProblem` combines a validated design, root, mandatory terminals,
  optional Steiner nodes, eligible partition requests and facility count quotas.
  It reuses `NetworkConstruction` only: no nominal flow variables, conservation,
  flow capacity rows or operating tariffs are registered implicitly.
- `RootedTreeComponent` uses unique parent arcs and depths bounded by the finite
  candidate-node count. Self-loops are excluded, parallel arcs retain their IDs,
  and tree arcs require built services and active endpoints. Production extraction
  independently follows every parent chain back to the root.
- Partition ownership is separate from transportation and tree membership.
  Undeclared candidate facilities are fixed open; declared facilities gate their
  assignments. Required requests have exactly one eligible owner, optional requests
  at most one. Quotas count assignments, not physical flow.
- Valid disconnection, required requests with no eligible facilities and unattainable
  minimum quotas remain ordinary model infeasibility. Unknown references, duplicate
  definitions and reversed quota intervals are input errors.
- `NetworkStructurePipeline` exposes construction, parent, tree-arc and assignment
  expressions for downstream constraints and objectives. Registration is transactional,
  repeated options must match, and a context cannot cross models. The standalone
  `NetworkStructureApplicationService` uses that same registration and extraction path.

## Transportation Allocation

- `AllocationProblem::new` validates immutable location/service/request snapshots,
  nondecreasing optional layers, explicit compatible-service sets, shared quotas and
  hubs before mapping once to `NetworkDesignProblem`. Original attributes remain
  available through the allocation problem; no parallel flow kernel is introduced.
- `AllocationOptions` declares canonical capacity and total-cost units. Default
  service tariffs are per capacity unit; request overrides are per request unit and
  take precedence. Heterogeneous capacities require explicit `service_usage`.
  `integer_step` scales conservation, cost and usage together and does not prohibit
  splitting an order across parallel services.
- Each quota has explicit usage (missing terms are zero) and at most one controlling
  hub. Hub activation gates its incident services and controlled resource rows through
  existing Design rules. Capacity levels remain total capacities and the optional
  budget applies to construction only. Unknown or incompatible quota/usage keys fail.
- `AllocationContext` delegates transactional registration and production extraction
  directly to Design, exposing its aggregation and accepting `NetworkDesignPipeline`
  extensions. `AllocationApplicationService` creates a fresh context and returns the
  unified domain report; illegal incumbents cannot bypass underlying flow validation.

## Whole-Path Finite Masters

- `PathCompilationProblem` declares explicit row units and finite optional bounds,
  a complete-cost unit and initial `PathColumn` candidates. Each whole path is binary;
  its payload retains internal activities/times and its complete cost/usage may be
  non-additive. Missing coefficients mean zero; unknown row keys fail before mutation.
- Equal column IDs are idempotent only when payload, normalized cost and every
  coefficient agree. Batch conflicts roll back. Removal clears managed rows and all
  objectives' old-variable terms while retaining unrelated extensions; retired tokens
  are fixed at zero because the core has no physical variable deletion. Re-addition
  creates a new variable. Extension remove hooks clean their own affected references.
- `PathCompilationContext` and `PathCompilationPipeline` share transactional register,
  add/remove, shadow-price and extraction hooks with `PathCompilationApplicationService`.
  Row/cost expressions are public. Full-model audits and independent binary/row/cost
  checks reject illegal incumbents in production. `candidate_ids` is the final finite
  pool: solver MILP optimality does not certify a larger path universe.
- Solve `pricing_model(&model)` for the actual LP relaxation. Verified refresh accepts
  a report or `VerifiedPricingDuals`, revalidates the current model and consumes its
  actual normalized signed duals for all declared rows, including both equality sides,
  both bounds and explicit zero prices. Unmapped external constraints prohibit pricing;
  declare their complete column coefficients as problem rows instead.
- Scope binds request, LP model fingerprint (`scope.model`), dual round, branch,
  groups and executor/slot domain. Revision, variable identities, objective snapshots,
  effective bounds/rows/types and current refresh epoch bind dual validity. A changed
  model or failed refresh invalidates old snapshots. Bare external prices are always
  heuristic, even if numerically equal to a certificate. Full reduced cost consumes
  every declared row; validity alone proves neither exhausted pricing nor optimality.

## VRPTW Corrections

Both ordinary and BigDecimal pricing remove permanent exclusion based on direct
travel-window failure; only monotone capacity exclusion remains. Both numeric paths
use `PathSearchContext`, retaining stable arc IDs and public label predecessor metadata.
They recompute complete terminal policy costs before reduced-cost filtering/ranking;
Phase I ignores operating costs. Uncertified cost decomposition or dominance
policies do not enable proof pruning by default. The default column selector
preserves reduced-cost order and limits output only. The exact builder accepts
`with_cost_policy(instance, policy)`; labels, full costs, duals and reduced costs
remain BigDecimal. Admitted terminal candidates survive subsequent budgets; zero
label budgets create no root, and label exhaustion takes precedence over depth
truncation in diagnostics. Small multigraph oracles cover both phases, parallel
arcs and indirect time-window recovery; sub-f64 improvements remain detectable.
Extended-row providers and per-model factories are integrated as described in
"Route Extension Rows and Factories"; the 12-case matrix remains an explicit
current-version test gate, not a conclusion inferred from compiling either numeric path.

## Graphs and Paths

- Existing `NetworkGraph<N, A>` payload wrappers remain. `NetworkGraphAccess`
  unifies graphs and views. `filtered` now returns a graph view rather than an arc
  iterator; use `view.arcs()` to enumerate arcs.
- Reversed and filtered views preserve stable identities without cloning payloads.
  Read endpoints through `view.from(arc)` and `view.to(arc)`, not payload fields.
- `NetworkTopology` provides reachability, minimum-hop witnesses, DAG ordering,
  SCCs and target-relevant shortest paths. Only source-reachable negative cycles
  that can reach the destination produce `NegativeCycle`.
- `shortest_path(&start, &target, &unit, cost)` replaces the scalar callback API.
  The callback returns `Result<Quantity<V, Unit>>`; `ShortestPathResult<V>::Found`
  contains a quantity in the requested unit. Normalize, compare and accumulate
  in `V` without converting weights to f64. Wrap old scalar costs in `Quantity`
  and supply `NoneUnit` for dimensionless costs. Incompatible/affine units and
  non-finite weights, conversions or sums return structured errors. Decimal and
  rational tests cover distinctions beyond f64 precision and decimal values
  outside its range, for both nonnegative and negative-edge searches.
- Construct `PathProblem::new(graph, policy)`, supply sources, terminals and options,
  then call `PathSearchContext.search(&problem, &budget)`. `PathPolicy` owns start,
  transition, whole-path terminal contributions, independent validation and optional
  dominance. Resources need not be monotonic; dominance is disabled by default.
- Cyclic non-elementary walks require a business step bound. DAGs do not. An output
  limit never stops traversal. Consume paths, diagnostics and `complete()` together;
  an empty result is not an infeasibility or no-improvement certificate.

## Objectives and Evidence

- `MetaModel::objective_snapshot` returns an independent copy of the registered
  objective used by the backend. Math-symbol inputs resolve to this representation
  during registration.
- `replace_sub_objective` replaces an owned named objective in place; callers must
  own that name. `remove_objective_terms` removes selected variable terms from all
  objectives, preserving constants, weights and directions. Route Phase I/II updates
  preserve unrelated business objectives; removed route variables lose objective terms.
- `VerifiedPricingDuals::new` independently checks the actual LP model, report,
  fingerprint, primal, dual and objective. `validate_for` rejects replay after model
  changes. The handle is read-only and cannot be constructed from raw duals.
- `ColumnGenerationBatch` separates exhausted, budget, interrupted and heuristic
  results and checks exact request scope and child coverage. Matching identity strings
  do not validate dual provenance; an empty batch is neither a no-improvement nor an
  integer-optimality proof.
- Legacy Gantt policies default to heuristic batches. Implement `generate_bunch_batch`
  to return completion against `BunchPricingRequest.scope`. Finite raw row duals may
  support heuristic pricing but cannot promote a bound without a model-bound certificate.
- `Iteration` now uses optional dual objectives, lower bounds and progress values.
  Candidate reduced costs no longer construct certified bounds. Restricted local
  pricing does not certify the whole domain; finite-pool MILP optimality is not global
  integer optimality.

## Numeric Boundary and Optional Bridge

The network value adapter rejects non-finite conversions. `normalize` checks both
source and target units are linear. Before any unit multiplication/division, downstream
code must also use `validate_linear_units` to avoid low-level affine-unit assertions.

`ospf-rust-framework-gantt-network` does not own a model or invoke a solver. It maps
whole resource paths to business bunches while retaining state, arc traces and batch
diagnostics. Extraction rejection downgrades completion. The independent no-improvement
predicate must not infer proof merely from an empty output-limited list.

The compatible local toolchain is `nightly-2026-08-01`; newer allocator traits are
incompatible with the existing base container contract, which this increment does
not change. SCIP fixtures cover standalone Flow, generalized flow, design,
explicit resilience, rooted structures, allocation and whole-path masters. Demo5 Gurobi/SCIP library and integration gates run separately,
covering direct-MIP, full-route oracle, smoke, strict proof and SCIP legal outcomes.
Task/bunch shared batches and current-model LP provenance validation are integrated.
Demo4's master-model loop and nonempty native cases are integrated; full migration acceptance remains
outstanding. Finite-candidate or smoke success does not imply complete migration or
general integer optimality; historical test counts are not acceptance evidence.

## Route Extension Rows and Factories

- Replace `BranchNodeSolverConfig::extensions` with `extension_factories`. Each factory
  creates a fresh `Box<dyn RouteCompilationExtension<V>>` for one model; direct contexts
  now accept boxed owned extensions rather than shared registered objects.
- Set `pricing_rows` to an immutable `Arc<RoutePricingRows<V>>` (empty by default at
  call sites). Standalone master/builder users must inject the same catalog on both sides.
  Providers return quantity-aware whole-column and optional arc coefficients; only an
  explicitly certified and independently replayed decomposition enables dominance.
- Every declared price key, including zero, is mandatory. Lower and upper duals are
  already signed after core-row normalization and are added, not subtracted again.
  Phase I adds feasibility artificials for both row sides without charging real costs;
  Phase II fixes them to zero. Route-column upper bounds are implicit in customer coverage,
  avoiding bound duals absent from pricing. Empty/unknown/repeated customer routes are rejected.
- Both numeric pricers recompute complete terminal coefficients before filtering and
  sorting. BigDecimal row coefficients, duals and reduced costs never pass through f64.
  The 12-case matrix covers both phases with no, zero-price and nonzero-price extensions.
- Proof pricing checks managed objectives, rows, active/retired variable domains and
  exact original-row dual coverage. Unmapped external/hidden compiled rows and price-hook
  changes are rejected; these checks do not replace the current-model LP report validator.

## Demo4 Recovery Records And Path Bridge

The Gantt bunch master now distinguishes stable `BunchId` from historical column indices.
Distinct recovery candidates with equal task coverage are retained; conflicting same-ID
payloads fail atomically. Opt-in `BunchMasterConfiguration` registers task coverage,
cancellation, executor selection, and an owned objective through the existing iterative
compilation path. Additions and model-aware retirement refresh actual owned rows/objectives;
slot batches roll back together. Static pipeline snapshots are not live subscriptions for
arbitrary downstream rows. The legacy graph generator retains true cost separately from
dual-dependent reduced cost and uses trace/time-based IDs independent of LP iteration.
Demo4 now consumes this shared compilation lifecycle through its actual application loop.

- `FlightTaskBunch` replaces mutable ID lists and cached endpoints with an immutable
  `FlightTaskRecovery` sequence. Construction returns `Result`; IDs, time, airports and
  flight resource use are derived from actual records.
- `RecoveryBunchGenerationContext` reuses shared resource paths and the bridge crate without
  nested global solving. It returns full business records, terminal costs, predecessors and
  completion, but creates no no-improvement or lower-bound certificate.
- Demo4's `BunchCompilationContext` now reuses the managed Gantt master through
  `RecoveryMasterConfiguration`. Full payload registration is atomic and aircraft-namespaced;
  retirement/re-addition uses fresh variables and extraction validates the complete matrix.
  It implements the Gantt iterative trait through an immutable, replayable `RecoveryProblem`
  directory; contexts do not share model registration state. Initial incumbents do not pin aircraft.
- Hard passenger coverage, seat usage and incompatible cross-aircraft transfer variants use
  the same fixed row provider for transactional registration/retirement and signed-dual pricing.
  Production extraction independently checks the complete recovery and passenger schedule.
- `RecoveryApplicationService` uses the existing Gantt loop, not a private CG implementation.
  Normal no-incumbent reports preserve termination, while finite-pool infeasibility maps to
  domain Unknown. The outer report has no global proof/bound; finite candidate IDs, numeric
  master reports and per-round traces remain available. Feasible seeds are required for hard domains.
- Bounded passenger cancellation and cabin changes share dynamic capacity/transfer rows with
  pricing. Per-leg disposition prevents negative demand; cancellation propagates forward and
  only debits the original cabin. Production extraction independently checks actual transported
  counts, actual-aircraft capacity, connections and penalties. Optional same-OD flight rebooking
  uses bounded integer flow between compatible selected recovery variants, including partial groups
  and nonbinary counts. Original and actual target OD must match. Flight penalties are separate,
  and neither estimated original times nor another unselected variant can authorize a connection.
- Terminal fleet targets count actual origins and endpoints, including unused aircraft and
  executed predecessors. Bounded integer shortage variables retain physical and modeled shortage
  separately. Fleet/passenger objectives are component-owned and preserve downstream extensions.
- Automatic seeds use the same shared path generator with complete original-record preferences,
  actual finite-domain recoveries and lifecycle-wide task locks. They preserve search evidence
  and budget/interruption diagnostics, including no-incumbent reports, but do not guarantee global
  hard-domain seed feasibility or introduce Phase I.
- `demo4-offline` tests a nonempty fake-solver loop. `demo4-gurobi-cg` cases actually run
  native Gurobi: internal events/effective links, and seed improvement with real RMP duals,
  cross-aircraft passengers and an independent complete-activity oracle. Feature forwarding
  alone is not evidence. Further cases exercise fleet shortage, automatic seeds/locks, cabin changes,
  real-dual flight-rebooking improvement and partial/nonbinary connection flows through actual RMPs.
  The CLI uses original-based automatic seeds, fleet targets and locks.
- Demo4 pricing deliberately issues no no-improvement certificate and disables unsafe automatic
  retirement through the legacy incomplete-dual callback. Explicit retirement/re-addition is
  tested. This is not a generic exact branch-and-price tree and does not establish acceptance of
  the entire Network / Gantt plan. Legacy constant-symbol passenger, fleet/link slack and duplicate aggregation/pipeline
  APIs, ID-only recovery graph/generators and the fixed-zero solver placeholder are removed in
  favor of managed contexts, shared paths and the actual application. Snapshot compilation-pipeline
  soft link-split/executor-leisure penalties now share the managed column lifecycle, complete signed-dual
  pricing and independent extraction. Half-unit cancellation losses and empty-bunch idle behavior have
  offline grid and native coverage. Actual-assignment restrictions now have relation/general conditions,
  hard generation/replay checks, bounded integer soft usage, complete dual rows and independent extraction;
  finite flow controls now share actual internal events/full-stay containment, window capacity,
  bounded integer excess, signed-dual rows and production checks with this lifecycle. Window boundaries
  have explicit event ownership; this is not arbitrary overlapping apron occupancy or a global spacetime
  framework. Passenger/cargo aircraft-category compatibility is checked for flight activities. Demo4's
  delivery scope is recorded below; the entire Network / Gantt plan is not certified by this delivery.
  See the example's paired READMEs for entry points and commands.
- The snapshot audit additionally restores Demo4 fixed-schedule release recommendations: bad complete
  reduced cost, highest complete cost, actual aircraft changes and reproducible random sampling without
  replacement, with exclusive FIFO tabu, zero quotas and atomic failure. It neither mutates the master
  nor overrides branch decisions. `Lock` replaces the ID-only DTO with exact actual `departure_time`;
  `with_departure_locks` applies it through seeds, pricing, replay and production extraction independently
  of aircraft locks. Valid times lacking candidates retain normal no-incumbent outcomes. The example's
  snapshot responsibility map makes finite-activity and non-global-cargo/crew-solver boundaries explicit.
- Demo4 executed-predecessor task identities are unique across aircraft and disjoint from pending tasks;
  shared-path full-bunch validation independently rejects re-execution. History determines actual origins
  and stays without becoming a new chargeable column or consuming remaining allowances again. Standalone
  bridging also rejects empty aircraft/activity identities before search rather than emitting unreplayable columns.
- Demo4 `RecoveryTransitionPolicy` restores injectable transition rules and turnaround calculation from
  the snapshots. Actual predecessor/current records flow through seeds, path terminal/trace checks,
  pricing, replay and independent production extraction. Invalid negative durations return errors;
  unmet valid rules reject candidates. Ignoring turnaround never bypasses custom rules or other hard
  minima, and none of these checks change pricing-proof boundaries.
- Demo4's specified-snapshot responsibilities and finite-activity-domain application migration are
  complete. Current gates include nonempty offline and actual Gurobi cases, affected Core/framework
  features and strict Clippy, Network SCIP, both Demo5 backends and workspace regressions. This delivery
  is separate from the entire Network / Gantt plan; it adds no global bound, full-path-domain integer
  optimality or continuous-time recovery guarantee.
