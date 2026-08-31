# concept

:us: English | :cn: [简体中文](README_ch.md)

This module defines a complete hierarchy of algebraic structure traits, providing the foundation for abstract algebra operations.

## Key Traits

### Basic Algebraic Structures

| Trait | Description | Requirements |
|-------|-------------|--------------|
| `Semigroup` | Semigroup (associativity) | `add` operation |
| `Monoid` | Monoid (semigroup + identity) | `Semigroup` + `zero` |
| `Group` | Group (monoid + inverse) | `Monoid` + `neg` |
| `AbelianGroup` | Abelian group (group + commutativity) | `Group` + commutativity |

### Multiplicative Structures

| Trait | Description | Requirements |
|-------|-------------|--------------|
| `MultiplicativeSemigroup` | Multiplicative semigroup | `mul` operation |
| `MultiplicativeMonoid` | Multiplicative monoid | `MultiplicativeSemigroup` + `one` |
| `MultiplicativeGroup` | Multiplicative group | `MultiplicativeMonoid` + `recip` |

### Rings and Fields

| Trait | Description | Requirements |
|-------|-------------|--------------|
| `Ring` | Ring (additive group + multiplicative semigroup) | `AbelianGroup` + `MultiplicativeMonoid` |
| `CommutativeRing` | Commutative ring (ring + multiplicative commutativity) | `Ring` + commutativity |
| `Field` | Field (commutative ring + multiplicative inverse) | `CommutativeRing` + `MultiplicativeGroup` |

### Linear Algebra Structures

| Trait | Description | Requirements |
|-------|-------------|--------------|
| `VectorSpace` | Vector space (using GAT for scalar field) | `AbelianGroup` + scalar multiplication |
| `NormedSpace` | Normed space (vector space + norm) | `VectorSpace` + `norm` |
| `InnerProductSpace` | Inner product space (normed space + inner product) | `NormedSpace` + `dot` |

### Ordered Structures

| Trait | Description |
|-------|-------------|
| `TotallyOrdered` | Total order comparison |

### Other Properties

| Trait | Description |
|-------|-------------|
| `Bounded` | Boundedness (min/max values) |
| `Epsilon` | Default precision tolerance |
| `Fixed` | Fixed point property |
| `Infinite` | Infinity support |
| `Scalar` | Scalar type marker |

## Reference Variants

Each algebraic trait has a corresponding `*Ref` variant (e.g., `SemigroupRef`, `MonoidRef`) that provides operations taking references, enabling more flexible borrowing patterns.

## Usage

```rust
use ospf_rust_math::algebra::concept::{Semigroup, Monoid, Group};

// Implement Semigroup for a custom type
impl Semigroup for MyType {
    fn add(self, rhs: Self) -> Self {
        // Implementation
    }
}
```

## License

This project is licensed under the MIT License.
