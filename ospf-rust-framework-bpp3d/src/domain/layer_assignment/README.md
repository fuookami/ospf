# BPP3D Layer Assignment Context

[中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-layer-assignment-context`.

## Purpose

Layer assignment owns RMP and final MILP assignment components, demand load expressions, capacity expressions, dynamic layer-column lifecycle, constraints, objectives, and shadow-price extraction.

## File Layout

- `model.rs` is the public modeling helper shim; variable arrays, expression arrays, solution extraction, and component traits live under `model/`.
- `service/mod.rs` is the public service shim; value adapters, assignment models, load/capacity models, iterative context, aggregation, and context registration live in sibling files.
- `service/limits.rs` is the public limits shim; each Kotlin-style limit/objective lives under `service/limits/`.

## Extension Points

Add new constraints or objectives as `Pipeline<MetaModel<f64>>` implementations under `service/limits/`. Keep add/remove/hide/fix/flush column lifecycle behavior in `IterativeLayerAssignmentContext`.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib`; for solver integration, also run serde feature checks and backend `--no-run` builds.
