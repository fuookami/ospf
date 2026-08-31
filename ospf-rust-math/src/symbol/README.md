# Symbol - Symbolic Computation Module

:us: English | :cn: [简体中文](README_ch.md)

## Overview

This module provides symbolic computation capabilities designed for optimization problems such as linear programming (LP), quadratic programming (QP), and general nonlinear
optimization.

### Key Features

- **Generic Value Types**: Support for `f64`, `BigDecimal`, `BigRational`, and any type implementing `Add + Sub + Mul + Div`
- **Dynamic Symbol Count**: Number of variables determined at runtime
- **Sparse Representation**: Only non-zero terms are stored
- **Zero-Cost Abstractions**: Compile-time unit integration with `Quantity<V, U: CTUnit>`

## Module Structure

```
symbol/
├── symbol/          # Symbol definitions (SymbolId, DynSymbol, Symbol, OwnedSymbol)
├── monomial/        # Monomials (LinearMonomial, QuadraticMonomial, CanonicalMonomial)
├── polynomial/      # Polynomials (Linear, Quadratic, Canonical)
├── inequality/      # Inequalities (Comparison, LinearInequality, etc.)
├── operation/       # Operations (Evaluate, Differentiate, ToLaTeX, etc.)
├── macros/          # Construction macros
├── parser/          # Expression parser (optional, requires "parser" feature)
└── serde.rs         # Serialization (optional, requires "serde" feature)
```

## Core Types

### Symbols

| Type          | Description                                            |
|---------------|--------------------------------------------------------|
| `SymbolId`    | Trait for static symbol identifiers                    |
| `SymbolDynId` | Dynamic identifier supporting high-dimensional symbols |
| `DynSymbol`   | Trait for runtime polymorphic symbols                  |
| `Symbol`      | Trait with associated `Id` type                        |
| `OwnedSymbol` | Owned wrapper `Box<dyn DynSymbol>`                     |

### Monomials

| Type                      | Form                      | Example    |
|---------------------------|---------------------------|------------|
| `LinearMonomial<T>`       | `c * S`                   | `2x`       |
| `QuadraticMonomial<T>`    | `c * S1 * S2` or `c * S`  | `xy`, `2x` |
| `CanonicalMonomial<T, E>` | `c * S1^n1 * S2^n2 * ...` | `x²y³`     |

### Polynomials

| Type              | Form                     | Example            |
|-------------------|--------------------------|--------------------|
| `Linear<T>`       | `Σ cᵢSᵢ + b`             | `2x + 3y + 1`      |
| `Quadratic<T>`    | `Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e` | `x² + 2xy + y + 1` |
| `Canonical<T, E>` | `Σ cᵢ * ∏ Sⱼ^nⱼ + d`     | `x²y³ + 2x + 1`    |

### Inequalities

| Type                        | Form                 | Example        |
|-----------------------------|----------------------|----------------|
| `LinearInequality<T>`       | `Linear op value`    | `2x + 3y ≤ 5`  |
| `QuadraticInequality<T>`    | `Quadratic op value` | `x² + y² ≤ 10` |
| `CanonicalInequality<T, E>` | `Canonical op value` | `x²y³ ≥ 1`     |

## Usage Examples

### Basic Symbol Operations

```rust
use ospf_rust_math::symbol::{OwnedSymbol, Linear, Quadratic};
use ospf_rust_math::symbol::test_utils::SimpleSymbol;

// Create symbols
let x = OwnedSymbol::new(SimpleSymbol::new("x"));
let y = OwnedSymbol::new(SimpleSymbol::new("y"));

// Linear expression: 2x + 3y + 1
let linear = 2.0 * x.clone() + 3.0 * y.clone() + 1.0;

// Quadratic expression: x² + 2xy + y²
let quad = x.clone() * x.clone() + 2.0 * x * y;
```

### Using Construction Macros

```rust
use ospf_rust_math::symbol::{symbols, linear, quadratic, linear_inequality};

// Define symbols
symbols!(x, y, z);

// Construct polynomials
let l = linear!(2.0 * x + 3.0 * y + 1.0);
let q = quadratic!(x * x + 2.0 * x * y + 1.0);

// Construct inequality: 2x + 3y ≤ 5
let ineq = linear_inequality!(2.0 * x + 3.0 * y <= 5.0);
```

### Evaluation and Differentiation

```rust
use ospf_rust_math::symbol::operation::{Evaluate, Differentiate};
use std::collections::HashMap;

// Evaluate polynomial
let values = HashMap::from([
    (x.clone(), 2.0),
    (y.clone(), 3.0),
]);
let result = linear.evaluate(&values); // 2*2 + 3*3 + 1 = 14

// Compute partial derivative
let dx = linear.partial_derivative(&x); // 2.0
let grad = linear.gradient(&[x, y]);    // [2.0, 3.0]
```

### Matrix Form Conversion

```rust
use ospf_rust_math::symbol::operation::ToMatrixForm;

// Convert to matrix form: x^T Q x + c^T x + d
let matrix_form = quad.to_matrix_form(&[x, y]);
// Q: 2x2 matrix, c: 2x1 vector, d: scalar
```

### LaTeX Output

```rust
use ospf_rust_math::symbol::operation::ToLaTeX;

let latex = linear.to_latex(); // "2 x + 3 y + 1"
```

### Compile-time Evaluation

```rust
use ospf_rust_math::symbol::operation::{CompileEval, CompileGradient};

// Compile to efficient function
let eval_fn = linear.compile_eval(&[x, y]);
let result = eval_fn(&[2.0, 3.0]); // Fast evaluation

let grad_fn = linear.compile_gradient(&[x, y]);
let gradient = grad_fn(&[2.0, 3.0]); // [2.0, 3.0]
```

## Implemented Features

| Feature                       | Status | Description                                                      |
|-------------------------------|--------|------------------------------------------------------------------|
| Symbol definitions            | ✅      | `SymbolId`, `DynSymbol`, `Symbol`, `OwnedSymbol`                 |
| Linear monomial/polynomial    | ✅      | `LinearMonomial<T>`, `Linear<T>`                                 |
| Quadratic monomial/polynomial | ✅      | `QuadraticMonomial<T>`, `Quadratic<T>`                           |
| Canonical monomial/polynomial | ✅      | `CanonicalMonomial<T, E>`, `Canonical<T, E>`                     |
| Inequalities                  | ✅      | `LinearInequality`, `QuadraticInequality`, `CanonicalInequality` |
| Evaluation                    | ✅      | `Evaluate`, `EvaluateOrdered` traits                             |
| Differentiation               | ✅      | `Differentiate`, `SecondOrderDifferentiate` traits               |
| Matrix form                   | ✅      | `ToMatrixForm` trait                                             |
| LaTeX output                  | ✅      | `ToLaTeX` trait                                                  |
| Type conversion               | ✅      | `ToLinear`, `ToQuadratic`, `ToCanonical` traits                  |
| Serialization                 | ✅      | serde support (optional feature)                                 |
| Parser                        | ✅      | Expression parsing (optional feature)                            |
| Construction macros           | ✅      | `symbols!`, `linear!`, `quadratic!`, etc.                        |
| Compile evaluation            | ✅      | `CompileEval`, `CompileGradient` traits                          |
| Term combining                | ✅      | `CombineTerms` trait for optimization                            |

## Extension Points

The module is designed for extensibility. Users can extend the system in the following ways:

### 1. Custom Symbol Types

Implement `DynSymbol` and `Symbol` traits for custom symbol types:

```rust
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

#[derive(Debug, Clone)]
pub struct MySymbol {
    id: usize,
    name: String,
    // custom fields...
}

impl DynSymbol for MySymbol {
    fn name(&self) -> &str { &self.name }
    fn display_name(&self) -> &str { &self.name }
    fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
    // ... implement other methods
}
```

### 2. Composite Operators

Create custom composite operators (e.g., `abs`, `max`, `min`) by implementing the `CompositeOperator` trait:

```rust
/// Composite operator trait for user-defined operators
pub trait CompositeOperator: Clone + Debug + Eq + Hash + Any {
    fn name(&self) -> &str;
    fn id(&self) -> u64;
}

/// Composite symbol with operator applied to inner expression
pub struct CompositeSymbol<Op: CompositeOperator, Inner> {
    operator: Op,
    inner: Inner,
}
```

This enables expressions like `|2x + 3y|` or `max(x, y)`.

### 3. Custom Value Types

Any type implementing the required traits can be used as the value type:

```rust
// Support for arbitrary precision
use bigdecimal::BigDecimal;
let precise: Linear<BigDecimal> = /* ... */;

// Support for exact rational arithmetic
use num_rational::BigRational;
let exact: Linear<BigRational> = /* ... */;

// Support for interval arithmetic
use ospf_rust_math::algebra::value_range::ValueRange;
let interval: Linear<ValueRange<f64>> = /* ... */;
```

### 4. Physical Quantity Integration

Combine with `ospf-rust-quantities` for dimension-checked physical quantities:

```rust
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

// Compile-time dimension checking
let distance: Quantity<Linear<f64>, Meter> = /* ... */;

// Runtime dimension checking
let runtime_distance: Quantity<Linear<f64>, Unit> = /* ... */;
```

### 5. Multi-dimensional Array Integration

Combine with `ospf-rust-multiarray` for vectorized operations:

```rust
use ospf_rust_multiarray::MultiArray;

// Vector of linear polynomials
let equations: MultiArray<Linear<f64>, Shape<2>> = /* ... */;

// Fast summation along axes
let sum = equations.sum_axis(0)?;
```

## Planned Features

| Feature             | Priority | Description                           |
|---------------------|----------|---------------------------------------|
| JIT compilation     | Low      | Native code generation for evaluation |
| Robust optimization | Planned  | Uncertainty set abstractions          |

## Dependencies

```
ospf-rust-math (symbol)
├── ospf-rust-base (basic types)
├── ospf-rust-multiarray (multi-dimensional arrays, optional)
└── ospf-rust-quantities (physical quantities, optional)
```

## Feature Flags

| Flag     | Description                                  |
|----------|----------------------------------------------|
| `serde`  | Enable serialization/deserialization support |
| `parser` | Enable expression parsing                    |

## References

- API documentation is available in the source code comments
