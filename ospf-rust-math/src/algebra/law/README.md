# law

:us: English | :cn: [简体中文](README_ch.md)

This module provides algebraic law sampling validators for verifying that types correctly implement their claimed algebraic structures.

## Key Types

| Type | Description | Validates |
|------|-------------|-----------|
| `GroupLaw<T>` | Group law sampling validator | Associativity, identity, inverse |
| `RingLaw<T>` | Ring law sampling validator | Additive group, multiplicative semigroup, distributivity |
| `FieldLaw<T>` | Field law sampling validator | Ring laws + multiplicative inverse |

## GroupLaw

Validates group axioms on finite samples:

- **Associativity**: `(a + b) + c == a + (b + c)`
- **Identity**: `a + 0 == a` and `0 + a == a`
- **Inverse**: `a + (-a) == 0` and `(-a) + a == 0`

```rust
use ospf_rust_math::algebra::law::GroupLaw;

let law = GroupLaw::new(
    vec![-2, -1, 0, 1, 2],
    |lhs: &i32, rhs: &i32| lhs + rhs,
    0,
    |value: &i32| -value,
    |lhs: &i32, rhs: &i32| lhs == rhs,
);

assert!(law.validate());
```

## RingLaw

Validates ring axioms on finite samples:

- **Additive group**: Associativity, identity, inverse, commutativity
- **Multiplicative associativity**: `(a * b) * c == a * (b * c)`
- **Multiplicative identity**: `a * 1 == a` and `1 * a == a`
- **Distributivity**: `a * (b + c) == a * b + a * c`

```rust
use ospf_rust_math::algebra::law::RingLaw;

let law = RingLaw::new(
    vec![-2, -1, 0, 1, 2],
    |lhs: &i32, rhs: &i32| lhs + rhs,
    |lhs: &i32, rhs: &i32| lhs * rhs,
    0,
    1,
    |value: &i32| -value,
    |lhs: &i32, rhs: &i32| lhs == rhs,
);

assert!(law.validate());
```

## FieldLaw

Validates field axioms on finite samples:

- **Ring laws**: All ring properties
- **Multiplicative commutativity**: `a * b == b * a`
- **Multiplicative inverse**: `a * (1/a) == 1` for non-zero `a`

```rust
use ospf_rust_math::algebra::law::FieldLaw;

let law = FieldLaw::new(
    vec![-2.0, -1.0, 0.0, 1.0, 2.0],
    |lhs: &f64, rhs: &f64| lhs + rhs,
    |lhs: &f64, rhs: &f64| lhs * rhs,
    0.0,
    1.0,
    |value: &f64| -value,
    |value: &f64| 1.0 / value,
    |value: &f64| value.abs() <= f64::EPSILON,
    |lhs: &f64, rhs: &f64| (lhs - rhs).abs() <= 1e-10,
);

assert!(law.validate());
```

## Individual Checks

Each validator provides methods for individual axiom checks:

- `is_associative()` - Check associativity only
- `has_identity()` - Check identity element only
- `has_inverse()` - Check inverse element only
- `validate()` - Check all axioms

## License

This project is licensed under the MIT License.
