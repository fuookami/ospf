# BPP3D Domain

:us: English | :cn: [简体中文](README_ch.md)

The BPP3D domain layer maps Kotlin domain submodules into Rust contexts while keeping Rust-native `MetaModel` registration boundaries.

## Responsibilities

- Keep BPP3D business vocabulary, geometry-aware rules, assignment components, and final packing semantics in domain contexts.
- Register variables, expressions, constraints, objectives, and shadow-price extraction through `MetaModel`-oriented components.
- Provide reusable contexts for layer generation, layer assignment, and final packing without duplicating orchestration code in application services.

## Modules

- `item`: item, package, material, pattern, bin, demand, cylinder, and continuous-radius models.
- `bla`: bottom-up-left-justified projection placement.
- `block_loading`: simple/complex block generation and DFS/MLHS placement search.
- `layer_generation`: Block, BL, Circle, Pattern, Pile, and Historical candidate generation.
- `layer_assignment`: RMP/final MILP assignment, dynamic columns, constraints, objectives, and shadow prices.
- `packing`: final packed-bin conversion, geometry guard, material summary, and render adapter.

Kotlin `bpp3d-domain-layer-selection-context` is intentionally mapped into `application/service/algorithm.rs` and executor orchestration, because Rust keeps layer selection as application flow around domain contexts instead of a separate domain context.

## Public API

- `item`
- `bla`
- `block_loading`
- `layer_generation`
- `layer_assignment`
- `packing`

## Extension Points

Domain contexts own business models, constraints, objectives, extension policies, and result extraction. Application code may orchestrate them, but should not duplicate domain modeling details.

## Lifecycle and Data Flow

Item and infrastructure models define typed inputs, layer generation creates candidate columns, layer assignment registers RMP/final MILP components and shadow prices, and packing converts selected layers into validated packed bins and render-ready outputs.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib` for domain coverage. Add `--features serde` when the touched domain data is loaded from or compared through CSV fixtures.

## Related Directories

- [`../application`](../application/README.md)
- [`../infrastructure`](../infrastructure/README.md)
