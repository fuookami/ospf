# ospf-rust-math

🇺🇸 [English](README.md) | 🇨🇳 简体中文

Mathematical functions library providing algebraic structures, geometric entities, chaotic systems, symbolic computation, and more.

## Module Overview

| Module | Description |
|--------|-------------|
| **algebra** | Algebraic structures (groups, rings, fields, vector spaces) and value ranges/intervals |
| **geometry** | Geometric entities (points, vectors, edges, triangles, circles) and Delaunay triangulation |
| **operator** | Mathematical operator traits (absolute value, power, trigonometry, tolerance comparison) |
| **chaotic** | Chaotic systems and iterative maps (Lorenz, Chen, various attractors) |
| **fractal** | Fractal generation algorithms (Mandelbrot set) |
| **combinatorics** | Combinatorics (combinations, permutations, Cartesian products) |
| **ordinary** | Ordinary mathematical functions (GCD, LCM, primes, factorization) |
| **symbol** | Symbolic computation (linear programming, quadratic programming, inequalities) |
| **trivalent** | Trivalent logic (true, false, unknown) |

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

## Features

| Feature | Description |
|---------|-------------|
| `serde` | Serialization/deserialization support |
| `parser` | Symbolic expression parser |

## License

MIT License
