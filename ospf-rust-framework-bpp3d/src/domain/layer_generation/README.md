# BPP3D Layer Generation Context

[中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-layer-generation-context`.

## Purpose

Layer generation creates candidate layers for column generation. It combines block, BLA, circle packing, Pattern, Pile, and Historical strategies while preserving request-scoped package rule policies and diagnostics for Kotlin comparison.

## File Layout

- `mod.rs` is the public context shim.
- `model.rs` defines requests, demand entries, traces, generator traits, and results.
- `context.rs` composes generators and deduplicates candidates.
- `block_layer_generator.rs`, `bl_layer_generator.rs`, and `circle_packing_layer_generator.rs` contain direct generators.
- `deferred_generators.rs` defines Pattern/Pile/Historical config and block-loading fallback.
- `pattern_generator.rs` includes Pattern strategy fragments under `pattern/`.
- `pile_generator.rs`, `historical_generator.rs`, and `scoring.rs` contain the remaining deferred strategies and ranking helpers.
- `tests/` groups strategy, package-rule, cylinder, and quality coverage.

## Extension Points

Add new candidate sources by implementing `LayerGenerator`. Add request-level business rules through `LayerGenerationPackageRulePolicy`; avoid storing arbitrary closures inside serializable item models. Keep source-specific diagnostics and coverage metrics suitable for Kotlin baseline comparison.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib` and the serde fixture quality reports for large Pattern/Pile/Historical comparison baselines.
