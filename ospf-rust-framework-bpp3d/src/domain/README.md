# BPP3D Domain

[中文](README_ch.md)

The BPP3D domain layer maps Kotlin domain submodules into Rust contexts while keeping Rust-native `MetaModel` registration boundaries.

## Contexts

- `item`: item, package, material, pattern, bin, demand, cylinder, and continuous-radius models.
- `bla`: bottom-up-left-justified projection placement.
- `block_loading`: simple/complex block generation and DFS/MLHS placement search.
- `layer_generation`: Block, BL, Circle, Pattern, Pile, and Historical candidate generation.
- `layer_assignment`: RMP/final MILP assignment, dynamic columns, constraints, objectives, and shadow prices.
- `packing`: final packed-bin conversion, geometry guard, material summary, and render adapter.

Kotlin `bpp3d-domain-layer-selection-context` is intentionally mapped into `application/service/algorithm.rs` and executor orchestration, because Rust keeps layer selection as application flow around domain contexts instead of a separate domain context.

## Rules

Domain contexts own business models, constraints, objectives, extension policies, and result extraction. Application code may orchestrate them, but should not duplicate domain modeling details.
