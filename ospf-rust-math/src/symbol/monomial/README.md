# monomial

🇺🇸 [English](README.md) | 🇨🇳 简体中文

Monomial types for representing single-term polynomial components. This module defines three monomial types with different algebraic structures.

## Key Types

| Type | Description |
|------|-------------|
| `LinearMonomial<T>` | Linear monomial: `c * S` (coefficient times symbol) |
| `QuadraticMonomial<T>` | Quadratic monomial: `c * S1 * S2` or `c * S1^2` |
| `CanonicalMonomial<T, E>` | Canonical monomial: `c * S1^n1 * S2^n2 * ...` |

## Algebraic Forms

- **LinearMonomial**: Simplest form with one symbol and a coefficient
  - Mathematical representation: `c * x`
  - Used in linear programming and affine expressions

- **QuadraticMonomial**: Two-symbol product or squared symbol
  - Mathematical representation: `c * x * y` or `c * x^2`
  - Used in quadratic programming and convex optimization

- **CanonicalMonomial**: General power-product form
  - Mathematical representation: `c * x^n * y^m * ...`
  - Used in polynomial optimization and algebraic geometry

## Usage

```rust
use ospf_rust_math::symbol::monomial::{LinearMonomial, QuadraticMonomial, CanonicalMonomial};
use ospf_rust_math::symbol::OwnedSymbol;

// Create a linear monomial: 2.5 * x
let x = OwnedSymbol::new("x");
let linear = LinearMonomial::new(2.5, x);

// Create a quadratic monomial: 3.0 * x * y
let y = OwnedSymbol::new("y");
let quadratic = QuadraticMonomial::new(3.0, x.clone(), y);

// Create a canonical monomial: 2.0 * x^3 * y^2
let canonical = CanonicalMonomial::with_powers(2.0, [(x, 3), (y, 2)]);
```

## License

This project is licensed under the MIT License.