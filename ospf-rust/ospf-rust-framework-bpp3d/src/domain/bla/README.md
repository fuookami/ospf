# BPP3D BLA Context

:us: English | :cn: [简体中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-bla-context`.

## Responsibilities

BLA provides bottom-up-left-justified projection placement for layer generation. It is a two-dimensional placement primitive used by BL local/global generators and must remain independent from final packing validation.

## File Layout

- `service.rs` contains the BLA algorithm, projections, placements, configuration, and tests.
- `mod.rs` re-exports the public algorithm entry.

## Public API

- `BottomUpLeftJustifiedAlgorithm`

## Extension Points

Add projection-level filtering or scoring here only when it is part of BLA placement itself. Package rules and layer candidate ranking belong to `layer_generation`.

## Lifecycle and Data Flow

Layer generation passes item projections and BLA configuration into the algorithm, receives ordered placement candidates, and then applies package-rule filtering and candidate scoring outside this context.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib` and layer-generation tests that exercise BL local/global generators.

## Related Directories

- [`../layer_generation`](../layer_generation/README.md)
- [`../block_loading`](../block_loading/README.md)
- [`../packing`](../packing/README.md)
