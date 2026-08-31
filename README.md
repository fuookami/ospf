# ospf-rust

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust` is the Rust implementation and migration workspace for OSPF. It provides base utilities, mathematical foundations, physical quantities, optimization core modeling, framework-level solver orchestration, and domain frameworks for BPP3D, CSP1D, and Gantt scheduling.

For the broader OSPF project and published documentation, see:

- ospf: <https://github.com/fuookami/ospf>
- documentation: <https://fuookami.github.io/ospf/>

## Scope

This workspace owns reusable Rust crates and framework migrations. It keeps domain-independent infrastructure, optimization modeling primitives, and reusable domain frameworks in the repository.

Explicit non-goals:

1. Business-specific request protocols, tenant context, formula languages, and runtime deployment adapters.
2. External renderer implementations.
3. Solver installation and license management beyond feature-gated adapter documentation.

## Module Structure

| Rust crate | Kotlin boundary | Responsibility |
| --- | --- | --- |
| [`ospf-rust-base`](ospf-rust-base/README.md) | `ospf-kotlin-utils` foundation | Error handling, indexed types, collections, containers, iterators, and cloneable function helpers. |
| [`ospf-rust-multiarray`](ospf-rust-multiarray/README.md) | `ospf-kotlin-multiarray` | Generic multi-dimensional arrays, shapes, views, storage order, and block arrays. |
| [`ospf-rust-math`](ospf-rust-math/README.md) | `ospf-kotlin-math` | Algebra, geometry, ordinary math, operators, chaotic systems, fractals, combinatorics, and symbolic computation. |
| [`ospf-rust-quantities`](ospf-rust-quantities/README.md) | `ospf-kotlin-quantities` | Physical dimensions, units, compile-time/runtime quantities, and quantity-aware arithmetic. |
| [`ospf-rust-core`](ospf-rust-core/README.md) | `ospf-kotlin-core` | Variables, tokens, symbols, `MetaModel`, flattening, solver traits, solver output, IIS, and backend adapters. |
| [`ospf-rust-framework`](ospf-rust-framework/README.md) | `ospf-kotlin-framework` | Pipeline modeling, shadow prices, column generation, Benders, combinatorial solvers, persistence contracts, remote solver client, and heartbeat utilities. |
| [`ospf-rust-framework-bpp3d`](ospf-rust-framework-bpp3d/README.md) | `ospf-kotlin-framework-bpp3d` | Reusable 3D bin-packing framework with BPP3D contexts, layer generation/assignment, packing, CSV fixtures, and renderer DTOs. |
| [`ospf-rust-framework-csp1d`](ospf-rust-framework-csp1d/README.md) | `ospf-kotlin-framework-csp1d` | Reusable one-dimensional cutting-stock framework with material, generation, produce, yield, waste, length, and application flows. |
| [`ospf-rust-framework-gantt-scheduling`](ospf-rust-framework-gantt-scheduling/README.md) | `ospf-kotlin-framework-gantt-scheduling` | Reusable Gantt scheduling framework with task, bunch, capacity, resource, produce, and branch-and-price flows. |
| [`ospf-rust-example`](ospf-rust-example/README.md) | `ospf-kotlin-example` | Runnable examples and migration compatibility demos. |

## Architecture Overview

The workspace follows a layered shape:

1. `base`, `multiarray`, `math`, and `quantities` provide reusable foundations.
2. `core` owns optimization modeling primitives and solver-facing model conversion.
3. `framework` adds solver orchestration, pipeline abstractions, shadow prices, persistence contracts, and remote solving.
4. domain framework crates assemble reusable business-domain modeling contexts around `MetaModel`.
5. `example` demonstrates current public flows and migration compatibility paths.

Framework domain crates should keep optimization semantics in context / aggregation / model component / pipeline layers. Application services coordinate solver selection, lifecycle, trace/KPI/render assembly, and recovery boundaries.

## Documentation Templates

New or refreshed README files should follow:

- [English template](docs/README_TEMPLATE.md)
- [中文模板](docs/README_TEMPLATE_ch.md)

Crate-level README files use the full template. Internal `src/domain`, `src/application`, and `src/infrastructure` README files may use the shorter skeleton: responsibilities, file layout, public API, extension points, lifecycle/data flow, validation, and related modules.

## Usage

Add workspace crates as path dependencies while developing inside this repository:

```toml
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core" }
ospf-rust-framework = { path = "../ospf-rust-framework" }
```

Domain frameworks can be enabled directly:

```toml
[dependencies]
ospf-rust-framework-csp1d = { path = "../ospf-rust-framework-csp1d" }
ospf-rust-framework-gantt-scheduling = { path = "../ospf-rust-framework-gantt-scheduling" }
```

## Local Validation

```powershell
cargo check --workspace
cargo test --workspace --no-run
```

For focused development, prefer package-level checks such as:

```powershell
cargo check -p ospf-rust-core
cargo test -p ospf-rust-framework-csp1d
```

Solver-backed tests require the corresponding Cargo feature and local solver installation or bundled support. See the core solver notes for [Gurobi](ospf-rust-core/src/solver/solvers/gurobi/README.md) and [SCIP](ospf-rust-core/src/solver/solvers/scip/README.md).

## Current Boundaries

This repository is actively migrating Kotlin framework capabilities into Rust. Some domain crates expose Kotlin-aligned public surfaces while still using Rust-side deterministic, fake, or feature-gated solver paths for parts of the lifecycle. Each domain crate README records its own current coverage and known gaps.

## Related Modules

- [OSPF Kotlin workspace](../ospf-kotlin/README.md)
- [OSPF Rust examples](ospf-rust-example/README.md)

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
