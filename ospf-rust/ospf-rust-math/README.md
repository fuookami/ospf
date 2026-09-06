# ospf-rust-math

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-math` maps Kotlin `ospf-kotlin-math` into the Rust mathematical foundation for OSPF. It provides algebraic structures, operators, ordinary math utilities, geometry, chaotic systems, fractals, combinatorics, symbolic computation, expression parsing, and trivalent logic.

## Scope

This crate owns reusable mathematical abstractions and algorithms used by core modeling, physical quantities, multiarray integrations, and domain frameworks.

Explicit non-goals:

1. Optimization model assembly and solver lifecycle; those belong in `ospf-rust-core` and `ospf-rust-framework`.
2. Physical unit ownership; those belong in `ospf-rust-quantities`.
3. Domain-specific business formulas unless they are reusable mathematical primitives.

Mathematical functions library providing algebraic structures, geometric entities, chaotic systems, symbolic computation, and more.

## Module Structure

| Rust module | Kotlin boundary | Responsibility |
| --- | --- | --- |
| [`algebra`](src/algebra/README.md) | `math/algebra` | Algebraic structures and value ranges/intervals. |
| [`geometry`](src/geometry/README.md) | `math/geometry` | Geometric entities and Delaunay triangulation. |
| [`operator`](src/operator/README.md) | `math/operator` | Mathematical operator traits such as absolute value, power, trigonometry, and tolerance comparison. |
| [`chaotic`](src/chaotic/README.md) | `math/chaotic` | Chaotic systems and iterative maps. |
| [`fractal`](src/fractal/README.md) | `math/fractal` | Fractal generation algorithms. |
| [`combinatorics`](src/combinatorics/README.md) | `math/combinatorics` | Combinations, permutations, and Cartesian products. |
| [`ordinary`](src/ordinary/README.md) | `math/ordinary` | GCD, LCM, primes, factorization, and common utilities. |
| [`symbol`](src/symbol/README.md) | `math/symbol` | Symbolic computation, polynomials, inequalities, expressions, parsing, macros, and operation traits. |
| `trivalent` | `math/trivalent` | Trivalent logic. |

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `algebra::*` | Algebraic concepts, laws, and value ranges. | stable within migration |
| `geometry::*` | Points, vectors, edges, triangles, circles, shapes, and triangulation helpers. | migration |
| `operator::*` | Reusable operator traits. | stable within migration |
| `ordinary::*` | Common math utilities. | stable within migration |
| `symbol::*` | Symbolic polynomials, expressions, inequalities, parsing, and macros. | migration |
| `chaotic::*` / `fractal::*` | Dynamical-system and fractal helpers. | stable within migration |

## Algebraic Structure Hierarchy

```
Semigroup
  └── Monoid
        └── Group
              └── AbelianGroup
                    └── Ring
                          └── CommutativeRing
                                └── Field

VectorSpace
  └── NormedSpace
        └── InnerProductSpace
```

## Chaotic Systems

Supports 28 chaotic systems and iterative maps, including:

- **3D systems**: Lorenz, Chen, Chen-Celikovsky, Chen-Lee, Coullet, Burke-Shaw, Bouali, Aizawa, Arneodo, etc.
- **2D systems**: Brusselator, Arnold's Cat Map, Baker's Map, Bogdanov Map, etc.
- **Scalar maps**: Circle Map, Arnold Tongue, Chebyshev Map, Gauss Map, etc.

## Usage Examples

```rust
use ospf_rust_math::geometry::{Point2, Point3};
use ospf_rust_math::chaotic::LorenzSystem;

// Create a Lorenz system and generate trajectory
let system = LorenzSystem::new(10.0, 28.0, 8.0 / 3.0, 0.01);
let mut gen = system.generator(Point3::new(1.0, 1.0, 1.0));
let next = gen.next_point(); // Execute one step
```

```rust
use ospf_rust_math::fractal::MandelbrotSet;

// Create a Mandelbrot set and generate sequence
let mandelbrot = MandelbrotSet::from_parts(-0.5, 0.5);
let mut gen = mandelbrot.generator_from_origin();
let z0 = gen.next_point(); // Origin
let z1 = gen.next_point(); // First iteration
```

```rust
use ospf_rust_math::combinatorics::{permutations, combinations_of_size, cross};

// Permutations and combinations
let perms = permutations(&[1, 2, 3]);            // 6 permutations
let combs = combinations_of_size(&[1, 2, 3, 4], 2); // 6 combinations

// Cartesian product
let result = cross(&[vec![1, 2], vec![3, 4]]);   // 4 combinations
```

## Generic Numeric Boundaries

Most math APIs are generic over numeric types and algebraic traits. Concrete numeric conversion should remain explicit at crate boundaries, especially when data flows into solver adapters or physical-quantity wrappers.

## Features

| Feature | Description |
|---------|-------------|
| `serde` | Serialization/deserialization support |
| `parser` | Symbolic expression parser |

## Local Validation

```powershell
cargo check -p ospf-rust-math
cargo test -p ospf-rust-math
cargo check -p ospf-rust-math --features serde
cargo check -p ospf-rust-math --features parser
```

## Related Modules

- [Root README](../README.md)
- [Quantities README](../ospf-rust-quantities/README.md)
- [Kotlin math README](../../ospf-kotlin/ospf-kotlin-math/README.md)

## License

MIT License
