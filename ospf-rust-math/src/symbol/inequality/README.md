# inequality

🇺🇸 [English](README.md) | 🇨🇳 简体中文

Inequality types for constraint representation in optimization problems. This module provides inequality types corresponding to polynomial forms.

## Key Types

| Type | Description |
|------|-------------|
| `Comparison` | Comparison operator: `<=`, `<`, `>=`, `>` |
| `LinearInequality<T>` | Linear constraint: `Linear <=, <, >=, > constant` |
| `QuadraticInequality<T>` | Quadratic constraint: `Quadratic <=, <, >=, > constant` |
| `CanonicalInequality<T, E>` | Canonical constraint: `Canonical <=, <, >=, > constant` |

## Comparison Operators

The `Comparison` enum defines four comparison directions:
- `Le` - Less than or equal (`<=`)
- `Lt` - Less than (`<`)
- `Ge` - Greater than or equal (`>=`)
- `Gt` - Greater than (`>`)

## Constraint Forms

- **LinearInequality**: Linear constraint for LP and MILP
  - Example: `2*x + 3*y <= 10`

- **QuadraticInequality**: Quadratic constraint for QP
  - Example: `x^2 + y^2 <= 1.0` (unit disk constraint)

- **CanonicalInequality**: General polynomial constraint
  - Example: `x^3 + 2*x*y^2 >= 5`

## Usage

```rust
use ospf_rust_math::symbol::inequality::{Comparison, LinearInequality, QuadraticInequality};
use ospf_rust_math::symbol::polynomial::{Linear, Quadratic};
use ospf_rust_math::symbol::OwnedSymbol;

// Create a linear inequality: 2*x + 3*y <= 10
let x = OwnedSymbol::new("x");
let y = OwnedSymbol::new("y");
let linear_poly = Linear::from_terms([(2.0, x), (3.0, y)], 0.0);
let linear_ineq = LinearInequality::new(linear_poly, Comparison::Le, 10.0);

// Create a quadratic inequality: x^2 + y^2 <= 1.0
let quadratic_poly = Quadratic::from_quadratic_terms(
    [(1.0, x.clone(), x.clone()), (1.0, y.clone(), y.clone())],
    [],
    0.0
);
let quadratic_ineq = QuadraticInequality::new(quadratic_poly, Comparison::Le, 1.0);
```

## License

This project is licensed under the MIT License.