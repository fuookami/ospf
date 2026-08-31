# BPP3D Application

:us: English | :cn: [简体中文](README_ch.md)

This context maps Kotlin `bpp3d-application` and also hosts the Rust-side orchestration equivalent of Kotlin `bpp3d-domain-layer-selection-context`.

## Responsibilities

The application layer coordinates CSV loading, solver backend selection, column-generation flow, reporting, render DTOs, fixture baselines, and Kotlin comparison artifacts. It should not own domain variables or constraint families; those remain in `domain/*` contexts and are registered through `MetaModel`-oriented components and pipelines.

## File Layout

- `service.rs` is the public application service shim; implementation fragments live under `service/`.
- `service/config.rs`, `state.rs`, `algorithm.rs`, and `standard_executors.rs` cover column-generation configuration, lifecycle state, and layer-selection orchestration.
- `service/executor.rs` contains RMP/final executor contracts plus MetaModel-backed executor adapters.
- `service/application_service.rs` coordinates one-round and CSV materialized flows.
- `service/fixture_suite.rs`, `dataset_suite.rs`, `layer_quality.rs`, and `reporting_helpers.rs` support large fixture runs, Kotlin baselines, and quality reports.
- `csv.rs` is the public CSV boundary; parser, materializer, Kotlin adapter, schema guard, and tests live under `csv/`.
- `report.rs` is the public structured report boundary; report DTO fragments live under `report/`.

## Public API

- `ColumnGenerationApplicationService`
- `ColumnGenerationAlgorithm`
- `ColumnGenerationConfig`
- `ColumnGenerationRmpExecutor`
- `ColumnGenerationFinalExecutor`
- `MetaModelSolverBackend`
- `MetaModelRmpExecutor`
- `MetaModelFinalExecutor`
- `ColumnGenerationResult`
- `Bpp3dRunReport`
- `Bpp3dFixtureReport`
- `CsvDatasetLoader` when `serde` is enabled.

## Extension Points

Add solver behavior through `MetaModelSolverBackend` or the RMP/final executor traits. Use `ColumnGenerationApplicationService::with_geometry_guard` with a `PackingGeometryContract` implementation to replace final geometry validation without changing application flow. Add dataset protocols at the CSV materializer boundary. Add comparison output through the fixture suite and run-report DTOs instead of embedding domain modeling logic in application flow.

## Lifecycle and Data Flow

CSV or in-memory input is materialized into application requests, layer generation provides candidate columns, RMP executors solve iterative assignment models, final executors solve selected-column MILP models, and packing/report adapters produce structured outputs. Layer selection remains an application orchestration concern around domain contexts.

## Verification

Use `cargo check -p ospf-rust-framework-bpp3d --features serde` for the full application protocol surface and `cargo test -p ospf-rust-framework-bpp3d --features serde --lib` for fixture/report/CSV coverage.

## Related Directories

- [`../domain`](../domain/README.md)
- [`../infrastructure`](../infrastructure/README.md)
