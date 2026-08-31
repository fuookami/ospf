# OSPF Rust Framework BPP3D

🇺🇸 English | 🇨🇳 [简体中文](README_ch.md)

This crate is the Rust migration target for `ospf-kotlin-framework-bpp3d`.
The current state provides the BPP3D infrastructure, item domain, layer assignment,
layer generation, packing context, and the first application-layer orchestration APIs.

Application APIs currently cover column-generation config/state/result types,
layer-generation orchestration, depth-boundary orientation validation,
known-coordinate placement adaptation, packing analysis, and renderer DTO output.
With the `serde` feature enabled, application CSV APIs also cover schema guarding,
multi-table dataset loading, typed CSV records, and request draft construction.
The application service also exposes mock RMP/final executor seams so materialized
CSV requests can run through the orchestration flow before real solver adapters land.
It now also provides solver-agnostic `MetaModel` RMP/final executor skeletons,
including registration diagnostics and no-op shadow/final extraction boundaries.
Solver-backed executor wrappers can already consume a pluggable backend result,
which keeps the next Gurobi/SCIP adapter outside the application service.
On non-`async` builds, `ColumnGenerationSolverMetaModelBackend` adapts framework
`ColumnGenerationSolver` LP/MILP outputs into the same executor backend contract.
The application service also has a one-round RMP -> shadow-price generation ->
column refresh -> final orchestration entry for validating column updates.
The current MetaModel assignment wiring registers minimal solvable demand cover,
final assignment activation, bin depth, and bin-count objective pipelines, and
`serde` CSV materialized requests can exercise the same solver-backed one-round flow
with a pluggable backend.
Layer candidates now carry explicit demand coverage metadata, and the application
fills default item coverage for materialized/CSV flows before RMP and final execution.
Layer generation results also expose block and placement traces, with
`BlockLayerGenerator` producing simple-block trace candidates for coverage-aware flows.
Selected coverage layers can now be converted into minimal packed bins and renderer
DTOs through the existing final execution analysis path.
Generated placement traces are also carried through the one-round application state,
allowing selected generated layers to replay into packed bins and CSV render fixtures.
Block traces now have the same replay path for simple-block generated layers, and
empty/mismatched final flows report structured diagnostics instead of silently dropping output.
Advanced generator skeletons, solver dataset no-run diagnostics, and conservative
continuous-radius CSV render fixtures are available as migration guardrails.
Trace replay has moved into the packing domain adapter, while serde smoke fixtures,
PatternedItem/PackageAttribute skeletons, and deferred objective/limit diagnostics
cover the next set of Kotlin migration guardrails.
The serde dataset suite now carries multiple smoke fixtures, fake one-round execution,
feature-matrix no-run diagnostics, PatternedItem demand coverage, PackageAttribute
packing diagnostics, deferred registration plans, and request-aware generator diagnostics.
It also supports JSON manifest- and directory-driven fixture loading, CSV PatternedItem /
PackageAttribute materialization, and conservative non-empty Pattern/Pile/Historical layer candidates.
Crate-level regression fixtures now exercise manifest loading, unified batch reports,
business-rule diagnostics, semantic deferred objective/constraint registration, and
configurable Pattern/Pile/Historical candidate generation.
Kotlin Gurobi grouped-layer and material-width-amount CSV samples can now be adapted
through the same serde fixture suite, including PWL radius ranges, mixed cuboid/cylinder
items, and Kotlin-style axis enum tokens.
Fixture manifests now carry group/tag/backend-smoke metadata, and batch reports summarize
load, materialization, no-run, fake execution, backend-smoke, render-plan, and diagnostic counts.
CSV business-rule validation aggregates patterned-item groups, package attributes, tags,
and default coverage diagnostics. Deferred Pattern/Pile/Historical generators also rotate
across multiple items and preserve shadow-price-aware scoring diagnostics, while the
deferred RestAmount/TailBinLoadingRate/BinLoadingOrder hooks have MetaModel registration tests.

See [bpp3d.md](bpp3d.md) for the detailed migration target, checklist, and acceptance criteria.
