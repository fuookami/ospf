# macros

🇺🇸 [English](README.md) | 🇨🇳 简体中文

Construction macros module for simplifying monomial, polynomial, and inequality construction. Provides both mathematical-style macros and legacy construction macros.

## Mathematical Expression Macros (Recommended)

### Polynomials

| Macro | Description | Example |
|-------|-------------|---------|
| `lin!` | Linear polynomial | `lin!(2 * x + 3 * y + 1)` |
| `quad!` | Quadratic polynomial | `quad!(x ^ 2 + 2 * x * y)` |

### Inequalities

| Macro | Description | Example |
|-------|-------------|---------|
| `ineq!` | Linear inequality | `ineq!(lin!(2 * x) <= 5.0)` |
| `qineq!` | Quadratic inequality | `qineq!(quad!(x ^ 2) <= 1.0)` |
| `cineq!` | Canonical inequality | `cineq!(poly >= 0.0)` |

### Constraint Sets

| Macro | Description | Example |
|-------|-------------|---------|
| `constraints!` | Constraint collection | `constraints![ineq!(...), ineq!(...)]` |

## Legacy Construction Macros

| Macro | Description |
|-------|-------------|
| `symbols!` | Create multiple symbols |
| `linear_monomial!` | Construct linear monomial |
| `quadratic_monomial!` | Construct quadratic monomial |
| `linear!` | Construct linear polynomial |
| `quadratic!` | Construct quadratic polynomial |

## Usage

### Mathematical Style (Recommended)

```rust
use ospf_rust_math::{lin, quad, ineq, qineq, constraints};
use ospf_rust_math::symbol::OwnedSymbol;

let x = OwnedSymbol::new("x");
let y = OwnedSymbol::new("y");

// Linear polynomial: 2*x + 3*y + 1
let linear = lin!(2 * x + 3 * y + 1);

// Quadratic polynomial: x^2 + 2*x*y + 3*y
let quadratic = quad!(x ^ 2 + 2 * x * y + 3 * y);

// Linear inequality: 2*x + y <= 10
let constraint1 = ineq!(lin!(2 * x + y) <= 10.0);

// Quadratic inequality: x^2 + y^2 <= 1
let constraint2 = qineq!(quad!(x ^ 2 + y ^ 2) <= 1.0);

// Constraint set
let constraints = constraints![constraint1, constraint2];
```

### Legacy Style

```rust
use ospf_rust_math::{symbols, linear, quadratic};

// Create symbols
let (x, y, z) = symbols!("x", "y", "z");

// Linear polynomial
let linear_poly = linear!(2.0 * x + 3.0 * y + 1.0);

// Quadratic polynomial
let quad_poly = quadratic!(x ^ 2 + 2.0 * x * y);
```

## Note

Macros are automatically exported to crate root via `#[macro_export]`, so they can be used directly without importing from this module.

## License

This project is licensed under the MIT License.