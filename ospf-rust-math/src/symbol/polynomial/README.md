# polynomial

🇺🇸 [English](README.md) | 🇨🇳 简体中文

Polynomial types for representing algebraic expressions in optimization problems. This module defines three polynomial types with increasing expressiveness.

## Key Types

| Type | Description |
|------|-------------|
| `Linear<T>` | Linear polynomial: `Sum(c_i * S_i) + b` |
| `Quadratic<T>` | Quadratic polynomial: `Sum(c_ij * S_i * S_j) + Sum(d_i * S_i) + e` |
| `Canonical<T, E>` | Canonical polynomial: `Sum(c_i * Prod(S_j^n_j)) + d` |

## Algebraic Forms

- **Linear**: Affine expression with linear terms and constant offset
  - Mathematical representation: `c_1 * x_1 + c_2 * x_2 + ... + b`
  - Used in linear programming (LP) and mixed-integer linear programming (MILP)

- **Quadratic**: Quadratic form with quadratic and linear terms
  - Mathematical representation: `x^T * Q * x + c^T * x + e`
  - Used in quadratic programming (QP) and convex optimization

- **Canonical**: General polynomial with arbitrary powers
  - Mathematical representation: `Sum of power-product terms + constant`
  - Used in polynomial optimization and nonlinear programming

## Kotlin Compatibility Aliases

The module provides Kotlin-style naming aliases:
- `LinearPolynomial<T>` = `Linear<T>`
- `QuadraticPolynomial<T>` = `Quadratic<T>`
- `CanonicalPolynomial<T, E>` = `Canonical<T, E>`

## Usage

```rust
use ospf_rust_math::symbol::polynomial::{Linear, Quadratic, Canonical};
use ospf_rust_math::symbol::OwnedSymbol;

// Create a linear polynomial: 2*x + 3*y + 5
let x = OwnedSymbol::new("x");
let y = OwnedSymbol::new("y");
let linear = Linear::from_terms([(2.0, x), (3.0, y)], 5.0);

// Create a quadratic polynomial: x^2 + 2*x*y + 3*y + 1
let quadratic = Quadratic::from_quadratic_terms(
    [(1.0, x.clone(), x.clone()), (2.0, x, y.clone())],
    [(3.0, y)],
    1.0
);

// Create a canonical polynomial
let canonical = Canonical::from_terms([...], 0.0);
```

## License

This project is licensed under the MIT License.