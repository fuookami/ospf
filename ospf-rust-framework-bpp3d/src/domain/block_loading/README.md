# BPP3D Block Loading Context

:us: English | :cn: [简体中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-block-loading-context`.

## Responsibilities

Block loading builds simple and complex blocks, then searches feasible block placements by DFS or multi-layer heuristic search. Layer-generation strategies use this context as a conservative fallback and candidate engine.

## File Layout

- `model.rs` exposes item views, simple/complex blocks, block placements, spaces, and block-loading context data.
- `service.rs` is the public service shim.
- `service/simple_block_generator.rs`, `complex_block_generator.rs`, `depth_first_search_algorithm.rs`, and `multi_layer_heuristic_search_algorithm.rs` mirror the Kotlin service files.
- `service/tests/` groups cuboid, cylinder, limit, DFS, and MLHS coverage.

## Public API

- `ItemView`
- `Block`
- `SimpleBlock`
- `ComplexBlock`
- `Space`
- `BlockPlacement`
- `SimpleBlockGenerator`
- `ComplexBlockGenerator`
- `DepthFirstSearchAlgorithm`
- `MultiLayerHeuristicSearchAlgorithm`

## Extension Points

Extend block generation with new block combinators or search heuristics here. Package rule filtering can be injected from `layer_generation`; final geometry rejection belongs to `packing`.

## Lifecycle and Data Flow

Item views are converted into simple blocks, optional complex blocks are composed, and DFS or MLHS searches produce block placements. Layer-generation strategies consume those placements as candidate layers or fallback columns.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib` and focus on block-loading plus layer-generation fallback tests.

## Related Directories

- [`../item`](../item/README.md)
- [`../layer_generation`](../layer_generation/README.md)
- [`../packing`](../packing/README.md)
