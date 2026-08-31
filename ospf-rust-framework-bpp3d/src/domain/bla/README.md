# BPP3D BLA Context

[中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-bla-context`.

## Purpose

BLA provides bottom-up-left-justified projection placement for layer generation. It is a two-dimensional placement primitive used by BL local/global generators and must remain independent from final packing validation.

## File Layout

- `service.rs` contains the BLA algorithm, projections, placements, configuration, and tests.
- `mod.rs` re-exports the public algorithm entry.

## Extension Points

Add projection-level filtering or scoring here only when it is part of BLA placement itself. Package rules and layer candidate ranking belong to `layer_generation`.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib` and layer-generation tests that exercise BL local/global generators.
