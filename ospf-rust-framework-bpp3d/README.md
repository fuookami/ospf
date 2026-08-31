# OSPF Rust Framework BPP3D

:us: English | :cn: [简体中文](README_ch.md)

This crate is the Rust migration target for `ospf-kotlin-framework-bpp3d`.
It now provides the BPP3D infrastructure, item domain, BLA/block-loading/layer-generation contexts, layer-assignment pipelines, packing context, CSV protocol adapters, renderer DTOs, and application-level column-generation orchestration.

The modeling path is centered on `MetaModel`: RMP and final MILP assembly share `LayerAssignmentContext`, `LayerAssignmentAggregation`, variable components, demand/depth/activation limits, and objective pipelines. Application services coordinate solver backends, CSV materialization, trace/KPI diagnostics, and renderer adaptation, while placement/block trace replay remains in the packing domain.

Current migration coverage includes:

1. Solver-agnostic `MetaModel` RMP/final executors, pluggable backend wrappers, and a one-round RMP -> shadow-price generation -> column refresh -> final flow.
2. Demand coverage metadata on generated layers, stable demand shadow-price keys, selected-layer extraction, and packed-bin/render replay from placement or block traces.
3. BLA local/global layer candidates, simple-block layer candidates, multi-round axis `ComplexBlockGenerator`, bounded DFS space-splitting, MLHS branch/depth candidate ranking, circle-packing grid candidates for fixed/discrete cylinder radii, and request-aware Pattern/Pile/Historical generators.
4. Axis-aware cuboid and cylinder geometry, guarded horizontal-cylinder final validation, PWL continuous-radius metadata/fixtures, and renderer `actualVolume` output.
5. CSV schema guards, Kotlin Gurobi grouped-layer/material-width-amount adapters, manifest and recursive directory fixture loading, backend survey/no-run/fake fallback reports, and feature-matrix diagnostics.
6. PatternedItem conservative demand coverage, PackageAttribute validation/packing diagnostics, and RestAmount/TailBinLoadingRate/BinLoadingOrder MetaModel registration hooks.

The manifest now mirrors the full Kotlin Gurobi CSV sample set into 22 fixtures, including 19 Kotlin-derived dataset samples plus 3 local regression fixtures. The full manifest suite is covered by real Gurobi 10 and SCIP feature-gated tests when those solvers are available locally.

Known remaining gaps are documented in [bpp3d.md](bpp3d.md). In short, the crate has a working Rust-style framework baseline with full-manifest real solver coverage; the remaining boundaries are the documented long-term non-goals.
