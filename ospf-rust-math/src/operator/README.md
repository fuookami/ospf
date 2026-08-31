# operator

🇺🇸 [English](README.md) | 🇨🇳 简体中文

## Overview

The `operator` module provides trait definitions for various mathematical operations, including basic operations, exponentials/logarithms, power operations, trigonometric functions, reference operations, and tolerance comparisons.

Key features:
- Comprehensive mathematical operator traits
- Precision-controlled operations for exponential and logarithmic functions
- Reference arithmetic for efficient computations
- Tolerance-based equality and ordering comparisons

## Sub-modules

| Sub-module | Description |
|------------|-------------|
| `abs` | Absolute value operations (`Abs`, `AbsRef`) |
| `contains` | Contains check operations (`Contains`) |
| `exp_log` | Exponential and logarithm operations (`Exp`, `Log`, `ExpWithPrecision`, `LogWithPrecision`) |
| `exponent` | Exponent marker trait (`Exponent`) |
| `power` | Power operations (`Pow`, `PowF`, `PowFWithPrecision`) |
| `reciprocal` | Reciprocal operations (`Reciprocal`, `ReciprocalRef`) |
| `ref_additive` | Reference additive operations (`AddRef`, `SubRef`, `NegRef`) |
| `ref_multiplicative` | Reference multiplicative operations (`MulRef`, `DivRef`) |
| `one_zero_ref` | Constant reference traits (`ZeroRef`, `OneRef`, `NegOneRef`, `Two`) |
| `tolerance` | Tolerance-based comparisons (`Tolerance`, `TolerancedEq`, `TolerancedOrd`) |
| `trigonometry` | Trigonometric and hyperbolic functions (`Trigonometry`) |

## Key Types

### Basic Operations

| Trait | Description |
|-------|-------------|
| `Abs` | Absolute value operation |
| `AbsRef` | Absolute value returning a reference |
| `Reciprocal` | Reciprocal (1/x) operation |
| `ReciprocalRef` | Reciprocal returning a reference |
| `Contains` | Check if a value contains another |

### Exponential and Logarithm

| Trait | Description |
|-------|-------------|
| `Exp` | Natural exponential (e^x) |
| `ExpWithPrecision` | Natural exponential with precision control |
| `Log` | Logarithm operation |
| `LogWithPrecision` | Logarithm with precision control |
| `Exponent` | Marker trait for exponent types |

### Power Operations

| Trait | Description |
|-------|-------------|
| `Pow` | Integer power operation |
| `PowF` | Floating-point power operation |
| `PowFWithPrecision` | Floating-point power with precision control |

### Trigonometric Functions

| Trait | Description |
|-------|-------------|
| `Trigonometry` | Comprehensive trigonometric operations including `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, and their inverse/hyperbolic variants |

### Reference Arithmetic

| Trait | Description |
|-------|-------------|
| `AddRef` | Addition returning a reference |
| `SubRef` | Subtraction returning a reference |
| `NegRef` | Negation returning a reference |
| `MulRef` | Multiplication returning a reference |
| `DivRef` | Division returning a reference |
| `ZeroRef` | Zero constant reference |
| `OneRef` | One constant reference |
| `NegOneRef` | Negative one constant reference |
| `Two` | Two constant reference |

### Tolerance Comparison

| Trait | Description |
|-------|-------------|
| `Tolerance` | Define tolerance for comparisons |
| `TolerancedEq` | Equality comparison with tolerance |
| `TolerancedOrd` | Ordering comparison with tolerance |

## Usage Example

```rust
use ospf_rust_math::operator::{Abs, Exp, Pow, Trigonometry};

// Absolute value
let x = -5.0_f64;
assert_eq!(x.abs(), 5.0);

// Exponential
let e_squared = 2.0_f64.exp();
assert!((e_squared - std::f64::consts::E.powi(2)).abs() < 1e-10);

// Power
let result = 2.0_f64.pow(3);
assert_eq!(result, 8.0);

// Trigonometry
let pi = std::f64::consts::PI;
assert!((pi.sin() - 0.0).abs() < 1e-10);
```

## License

MIT License
