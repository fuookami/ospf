# ospf-rust-quantities

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-quantities` maps Kotlin `ospf-kotlin-quantities` into a Rust physical quantity, dimension, and unit system. It supports compile-time and runtime unit modes through `Quantity<V, U>` and integrates with `ospf-rust-math` for quantity-aware symbolic computation.

## Scope

This crate owns physical dimensions, unit systems, unit conversion, quantity arithmetic, compile-time/runtime quantity modes, and quantity-aware symbolic helpers.

Explicit non-goals:

1. Optimization solver modeling or framework orchestration.
2. Domain-specific business units unless they become general reusable units.
3. Serialization protocol ownership beyond quantity/unit data boundaries.

Physical quantities, dimensions and units system

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`ospf-rust-quantities` is a Rust library providing a complete implementation of physical quantities, dimensions, and units, supporting both runtime and compile-time modes through a unified type system.

### Key Features

- **Unified Type System**: Single `Quantity<V, U>` type works for both compile-time and runtime modes
- **Zero-cost Abstraction**: Compile-time unit types (`Quantity<V, U: CTUnit>`) complete all calculations at compile time with no runtime overhead
- **Compile-time Dimension Checking**: Mismatched dimension operations cause compile errors for compile-time quantities
- **Flexible Unit Conversion**: Supports automatic conversion between units of the same dimension
- **Predefined Unit Systems**: Built-in SI, MKS, CGS and other unit systems
- **Generic Value Types**: Supports `BigDecimal`, `f64`, and other numeric types
- **Complete Bilingual Documentation**: Available in both Chinese and English

## Installation

Add dependency in `Cargo.toml`:

```toml
[dependencies]
ospf-rust-quantities = "0.1.0"
```

## Generic Numeric Boundaries

`Quantity<V, U>` is generic over value type `V`. Supported examples include `f64`, `BigDecimal`, and rational/symbolic values through math integration. Conversion between numeric types should remain explicit at caller or adapter boundaries.

## Physical Quantity Boundaries

Physical quantities should preserve dimensions through compile-time unit types when units are known statically, or through runtime `Unit` when unit selection is dynamic. Bare values are appropriate only for dimensionless scale factors, counts, and low-level adapter edges.

## Quick Start

### Compile-time Quantities

Compile-time quantities have their unit types determined at compile time, providing zero-cost abstraction and compile-time dimension checking.

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Second};
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

// Create compile-time quantity (using new_ct for CTUnit types)
let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(1000));

// Compile-time unit conversion
let length_km: Quantity<BigDecimal, Kilometer> = length.to();
assert_eq!(length_km.value, BigDecimal::from(1));

// Quantity operations produce new unit types
let time: Quantity<BigDecimal, Second> = Quantity::new_ct(BigDecimal::from(10));
let velocity = length_km / time; // Type: Quantity<BigDecimal, CTUnitDiv<Kilometer, Second>>
```

### Runtime Quantities

Runtime quantities have their units determined at runtime, supporting dynamic unit conversion.

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Kilogram};
use ospf_rust_quantities::unit::{CTUnit, Unit};
use bigdecimal::BigDecimal;

// Create runtime quantity (using Unit type)
let length: Quantity<BigDecimal, Unit> = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());

// Runtime unit conversion
let length_km = length.to_unit(&Kilometer::INSTANT.clone()).unwrap();
assert_eq!(length_km.value, BigDecimal::from(1));

// Quantity operations
let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());
let momentum = &length * &mass; // Produces new dimension
```

### Converting Between Modes

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::{CTUnit, Unit};
use bigdecimal::BigDecimal;

// Create compile-time quantity
let ct_length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));

// Convert to runtime quantity
let rt_length: Quantity<BigDecimal, Unit> = ct_length.to_runtime();
assert_eq!(rt_length.unit.symbol(), "m");
```

### Unit Systems

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::system::SI_SYSTEM;
use ospf_rust_quantities::unit::{CTUnit, UnitSystem};
use bigdecimal::BigDecimal;

let length = Quantity::new(BigDecimal::from(100), Meter::INSTANT.clone());

// Convert to standard unit of the system
let standard = length.to_standard_unit(SI_SYSTEM.as_ref()).unwrap();
assert_eq!(standard.unit.symbol(), "m"); // Standard unit for length in SI is meter
```

## API Overview

### Core Types

| Type | Description |
|------|-------------|
| `Quantity<V, U>` | Unified quantity type, `U` can be `Unit` (runtime) or a `CTUnit` type (compile-time) |
| `Quantity<V, Unit>` | Runtime quantity with unit determined at runtime |
| `Quantity<V, U: CTUnit>` | Compile-time quantity with unit type determined at compile time |
| `Unit` | Runtime unit |
| `CTUnit` | Compile-time unit trait |
| `DerivedQuantity` | Derived dimension |
| `CTDerivedQuantity` | Compile-time derived dimension trait |
| `Scale` | Unit scale |
| `QuantityTrait` | Unified interface for all quantity types |

### Module Structure

```
ospf_rust_quantities
├── dimension          # Dimensions
│   ├── fundamental    # Fundamental dimensions
│   └── derived        # Derived dimensions
├── unit               # Units
│   ├── physical_unit  # Core unit types
│   ├── system         # Unit systems (SI, MKS, CGS)
│   └── derived        # Predefined derived units
├── quantity           # Quantities (unified type)
├── scale              # Scales
└── error              # Error types
```

### Predefined Units

| Category | Units |
|----------|-------|
| Length | Meter, Kilometer, Centimeter, Millimeter, Inch, Foot, Yard, Mile |
| Mass | Kilogram, Gram, Milligram, Ton |
| Time | Second, Minute, Hour, Day |
| Velocity | MeterPerSecond, KilometerPerHour |
| Acceleration | MeterPerSecondSquared |
| Force | Newton, Dyne |
| Energy | Joule, Calorie, ElectronVolt |
| Power | Watt, Kilowatt, Horsepower |
| Pressure | Pascal, Bar, Atmosphere |
| Frequency | Hertz, Kilohertz, Megahertz |
| Information | Bit, Byte, Kilobyte, Megabyte |

## Physical Quantity Symbolic Computation

`ospf-rust-quantities` integrates with `ospf-rust-math` to support symbolic computation with physical quantities. This allows you to create polynomials with physical units, combining the benefits of type-safe dimensional analysis with symbolic mathematics.

### Supported Value Types

| Value Type | Crate | Use Case |
|------------|-------|----------|
| `f64` | `std` | Fast numerical computation |
| `BigDecimal` | `bigdecimal` | High-precision decimal for financial calculations |
| `BigRational` | `num-rational` | Exact rational numbers for symbolic computation |

### Usage Examples

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::CTUnit;
use ospf_rust_math::symbol::polynomial::Linear;
use ospf_rust_math::symbol::symbol::OwnedSymbol;
use bigdecimal::BigDecimal;
use num_rational::BigRational;

// Physical quantity symbol
let x: Quantity<OwnedSymbol, Meter> = Quantity::new_ct(OwnedSymbol::new("x"));
let y: Quantity<OwnedSymbol, Meter> = Quantity::new_ct(OwnedSymbol::new("y"));

// Physical quantity polynomial with f64
let distance: Quantity<Linear<f64>, Meter> = Quantity::new_ct(2.0 * x.clone() + 3.0 * y.clone());

// With BigDecimal
let coef_bd = BigDecimal::from(2);
let distance_bd: Quantity<Linear<BigDecimal>, Meter> = Quantity::new_ct(coef_bd * x.clone());

// With BigRational
let coef_br = BigRational::new(3.into(), 2.into()); // 3/2
let distance_br: Quantity<Linear<BigRational>, Meter> = Quantity::new_ct(coef_br * x);
```

### Type Definitions

| Type | Description |
|------|-------------|
| `Quantity<OwnedSymbol, U>` | Physical quantity symbol |
| `Quantity<LinearMonomial<T>, U>` | Physical quantity linear monomial |
| `Quantity<Linear<T>, U>` | Physical quantity linear polynomial |
| `Quantity<QuadraticMonomial<T>, U>` | Physical quantity quadratic monomial |
| `Quantity<Quadratic<T>, U>` | Physical quantity quadratic polynomial |
| `Quantity<Canonical<T, E>, U>` | Physical quantity canonical polynomial |

### Compile-time vs Runtime Units

Symbolic computation works with both compile-time and runtime units:

```rust
// Compile-time (zero-cost)
let ct_symbol: Quantity<OwnedSymbol, Meter> = Quantity::new_ct(OwnedSymbol::new("x"));

// Runtime (flexible)
let rt_symbol: Quantity<OwnedSymbol, Unit> = 
    Quantity::new(OwnedSymbol::new("x"), Meter::INSTANT.clone());
```

## Compile-time Dimension Checking

Compile-time quantities check dimension matching at compile time. The following code will cause a compile error:

```rust,compile_fail
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Second};
use bigdecimal::BigDecimal;

let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
let time: Quantity<BigDecimal, Second> = Quantity::new_ct(BigDecimal::from(5));

// Compile error: quantities with different dimensions cannot be added
let result = length + time; // Error!
```

## Custom Units

### Compile-time Custom Units

```rust
use ospf_rust_quantities::unit::physical_unit::CTUnit;
use ospf_rust_quantities::dimension::derived_quantity::Length;
use ospf_rust_quantities::scale::Scale;
use once_cell::sync::Lazy;

// Define custom length unit
struct MyUnit;

impl CTUnit for MyUnit {
    const NAME: &'static str = "my_unit";
    const SYMBOL: &'static str = "mu";
    const SCALE: Lazy<Scale> = Lazy::new(|| Scale::from_int(100)); // 100 base units
    type Dimension = Length;
}
```

### Runtime Custom Units

```rust
use ospf_rust_quantities::unit::physical_unit::UnitBuilder;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::scale::Scale;

// Create custom unit using UnitBuilder
let custom_unit = UnitBuilder::new(
    Meter::INSTANT.dimension().clone(),
    Scale::from_int(100)
)
.name("custom_unit")
.symbol("cu")
.build();
```

## Error Handling

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilogram};
use ospf_rust_quantities::unit::{CTUnit, Unit};
use bigdecimal::BigDecimal;

let length: Quantity<BigDecimal, Unit> = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
let mass_unit = Kilogram::INSTANT.clone();

// Try to convert to unit with different dimension
match length.to_unit(&mass_unit) {
    Ok(q) => println!("Conversion succeeded: {}", q.value),
    Err(e) => println!("Error: {}", e.msg()), // Prints dimension mismatch error
}
```

## Performance Benchmarks

The following benchmarks compare the performance of compile-time (`Quantity<V, CTUnit>`) and runtime (`Quantity<V, Unit>`) physical quantities. Run with:

```bash
cargo bench --package ospf-rust-quantities --bench quantity_bench
```

### Key Findings

1. **Compile-time quantities excel at creation** - No need to clone `Unit` objects, resulting in ~20x faster creation
2. **Compile-time quantities are faster for arithmetic** - No runtime dimension checking overhead
3. **Runtime quantities are faster for unit conversion** - Runtime implementation is better optimized
4. **Compile-time quantities show significant advantage in batch operations** - Especially for operations producing new dimensions

### When to Use Which Mode

| Scenario | Recommended Mode |
|----------|------------------|
| Performance-critical code with known units | Compile-time (`Quantity<V, U: CTUnit>`) |
| Dynamic unit selection at runtime | Runtime (`Quantity<V, Unit>`) |
| Need compile-time dimension safety | Compile-time |
| Interoperability with user-provided units | Runtime |
| Mixed scenarios | Use compile-time and convert to runtime when needed |

## Local Validation

```powershell
cargo check -p ospf-rust-quantities
cargo test -p ospf-rust-quantities
cargo bench --package ospf-rust-quantities --bench quantity_bench
```

## Related Modules

- [Root README](../README.md)
- [Math README](../ospf-rust-math/README.md)
- [Kotlin quantities README](../../ospf-kotlin/ospf-kotlin-quantities/README.md)

## License

Licensed under the MIT License. See [LICENSE](../LICENSE) for details.
