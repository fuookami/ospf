# algebra

:us: English | :cn: [简体中文](README_ch.md)

## Overview

This module provides a complete hierarchy of algebraic structure definitions, from semigroups to fields, as well as vector spaces, normed spaces, and inner product spaces. It also provides value ranges and interval definitions for numerical computations.

## Sub-modules

| Module | Description |
|--------|-------------|
| [`concept`] | Algebraic concept traits (groups, rings, fields, vector spaces, etc.) |
| [`law`] | Algebraic law sampling validators (associativity, commutativity, distributivity, etc.) |
| [`value_range`] | Value ranges and interval definitions |

## Algebraic Structure Hierarchy

```
Additive Structures:              Multiplicative Structures:       Combined Structures:
------------------               -------------------------       --------------------
Semigroup                        MultiplicativeSemigroup
    │                                │
    ▼                                ▼
Monoid                           MultiplicativeMonoid
    │                                │
    ▼                                ▼
Group                            MultiplicativeGroup
    │                                │
    ▼                                │
AbelianGroup                        │
    │                                │
    └────────────────┬───────────────┘
                       │
                       ▼
                     Ring
                       │
                       ▼
                CommutativeRing
                       │
                       ▼
                     Field

Linear Algebra Structures:
-------------------------
VectorSpace
    │
    ▼
NormedSpace
    │
    ▼
InnerProductSpace
```

## Key Types

### Algebraic Concepts (`concept` module)

| Trait | Description |
|-------|-------------|
| `Semigroup` | Semigroup (associative binary operation) |
| `Monoid` | Monoid (semigroup with identity element) |
| `Group` | Group (monoid with inverse elements) |
| `AbelianGroup` | Abelian/commutative group |
| `MultiplicativeSemigroup` | Multiplicative semigroup |
| `MultiplicativeMonoid` | Multiplicative monoid |
| `MultiplicativeGroup` | Multiplicative group |
| `Ring` | Ring (additive group + multiplicative semigroup) |
| `CommutativeRing` | Commutative ring (ring with multiplicative commutativity) |
| `Field` | Field (commutative ring with multiplicative inverses) |
| `VectorSpace` | Vector space (using GAT for scalar field) |
| `NormedSpace` | Normed space (vector space with norm) |
| `InnerProductSpace` | Inner product space (normed space with inner product) |
| `TotallyOrdered` | Totally ordered elements |
| `Bounded` | Bounded elements (with min/max values) |
| `Epsilon` | Default precision tolerance |
| `Fixed` | Fixed point/constant elements |
| `Infinite` | Infinity support |
| `Scalar` | Scalar type marker |

### Law Validators (`law` module)

| Struct | Description |
|--------|-------------|
| `GroupLaw<T, Add, Neg, Eq>` | Validates associativity, identity, and inverse properties |
| `RingLaw<T, Add, Mul, Neg, Eq>` | Validates additive group, additive commutativity, multiplicative associativity, multiplicative identity, and distributivity |
| `FieldLaw<T, Add, Mul, Neg, Recip, IsZero, Eq>` | Validates all ring laws plus multiplicative commutativity and multiplicative inverse for non-zero elements |

### Value Range (`value_range` module)

| Type | Description |
|------|-------------|
| `ValueWrapper<T>` | Value wrapper adding infinity support to any numeric type |
| `Bound<T, I>` | Interval bound with openness/closedness marker |
| `ValueRange<T, IL, IU>` | Value range/interval with compile-time or runtime openness |
| `IntervalTrait` | Trait for interval openness abstraction |
| `Closed` | Closed interval marker type (compile-time) |
| `Open` | Open interval marker type (compile-time) |
| `Interval` | Runtime openness enumeration |

## Usage Example

```rust
use ospf_rust_math::algebra::{Semigroup, Monoid, Group, Field};
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};

// Using algebraic traits
fn compute<T: Field>(a: T, b: T) -> T {
    a + b
}

// Using value ranges
let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Closed),
);
```

## License

MIT License
