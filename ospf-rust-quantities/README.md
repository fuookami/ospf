# ospf-rust-quantities

Physical quantities, dimensions and units system

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`ospf-rust-quantities` is a Rust library providing a complete implementation of physical quantities, dimensions, and units, supporting both runtime and compile-time modes.

### Key Features

- **Zero-cost Abstraction**: Compile-time unit types complete all calculations at compile time with no runtime overhead
- **Compile-time Dimension Checking**: Mismatched dimension operations cause compile errors
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

## Quick Start

### Compile-time Quantities

Compile-time quantities have their unit types determined at compile time, providing zero-cost abstraction and compile-time dimension checking.

```rust
use ospf_rust_quantities::quantity::CTQuantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Second};
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

// Create compile-time quantity
let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(1000));

// Compile-time unit conversion
let length_km: CTQuantity<BigDecimal, Kilometer> = length.to();
assert_eq!(length_km.value, BigDecimal::from(1));

// Quantity operations produce new unit types
let time: CTQuantity<BigDecimal, Second> = CTQuantity::new(BigDecimal::from(10));
let velocity = length_km / time; // Type: CTQuantity<BigDecimal, CTUnitDiv<Kilometer, Second>>
```

### Runtime Quantities

Runtime quantities have their units determined at runtime, supporting dynamic unit conversion.

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Kilogram};
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

// Create runtime quantity
let length = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());

// Runtime unit conversion
let length_km = length.to_unit(&Kilometer::INSTANT.clone()).unwrap();
assert_eq!(length_km.value, BigDecimal::from(1));

// Quantity operations
let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());
let momentum = &length * &mass; // Produces new dimension
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
| `CTQuantity<V, U>` | Compile-time quantity with unit type `U` determined at compile time |
| `Quantity<V>` | Runtime quantity with unit determined at runtime |
| `Unit` | Runtime unit |
| `CTUnit` | Compile-time unit trait |
| `DerivedQuantity` | Derived dimension |
| `CTDerivedQuantity` | Compile-time derived dimension trait |
| `Scale` | Unit scale |

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
├── quantity           # Quantities
│   ├── ct_quantity    # Compile-time quantities
│   └── quantity       # Runtime quantities
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

## Compile-time Dimension Checking

Compile-time quantities check dimension matching at compile time. The following code will cause a compile error:

```rust,compile_fail
use ospf_rust_quantities::quantity::CTQuantity;
use ospf_rust_quantities::unit::derived::{Meter, Second};
use bigdecimal::BigDecimal;

let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
let time: CTQuantity<BigDecimal, Second> = CTQuantity::new(BigDecimal::from(5));

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
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
let mass_unit = Kilogram::INSTANT.clone();

// Try to convert to unit with different dimension
match length.to_unit(&mass_unit) {
    Ok(q) => println!("Conversion succeeded: {}", q.value),
    Err(e) => println!("Error: {}", e.msg()), // Prints dimension mismatch error
}
```

## Performance Benchmarks

The following benchmarks compare the performance of compile-time (`CTQuantity`) and runtime (`Quantity`) physical quantities. Run with:

```bash
cargo bench --package ospf-rust-quantities --bench quantity_bench
```

### Basic Operations

| Operation | CTQuantity | Quantity | Ratio |
|-----------|------------|----------|-------|
| Creation | 35.6 ns | 704.8 ns | CTQuantity **20x faster** |
| Addition (reference) | 70.5 ns | 85.0 ns | CTQuantity **1.2x faster** |
| Addition (different units) | - | 242.9 ns | - |
| Subtraction (reference) | 71.7 ns | 86.4 ns | CTQuantity **1.2x faster** |
| Scalar Multiplication | 70.3 ns | 81.2 ns | CTQuantity **1.2x faster** |
| Scalar Division | 107.2 ns | 98.1 ns | Quantity **1.1x faster** |
| Negation | 27.1 ns | 26.5 ns | Comparable |

### Quantity Operations (producing new dimensions)

| Operation | CTQuantity | Quantity | Ratio |
|-----------|------------|----------|-------|
| Multiplication | 69.8 ns | 343.3 ns | CTQuantity **4.9x faster** |
| Division | 104.7 ns | 403.5 ns | CTQuantity **3.9x faster** |

### Unit Conversion

| Operation | CTQuantity | Quantity | Ratio |
|-----------|------------|----------|-------|
| Meter → Kilometer | 515.2 ns | 171.3 ns | Quantity **3x faster** |
| Kilometer → Meter | 500.4 ns | 116.6 ns | Quantity **4.3x faster** |

### Batch Operations (10,000 iterations)

| Operation | CTQuantity | Quantity | Ratio |
|-----------|------------|----------|-------|
| Addition | 683.9 µs | 955.3 µs | CTQuantity **1.4x faster** |
| Multiplication | 690.7 µs | 8.4 ms | CTQuantity **12x faster** |
| Unit Conversion | 5.57 ms | 1.91 ms | Quantity **2.9x faster** |

### Compound Operations

| Operation | CTQuantity | Quantity | Ratio |
|-----------|------------|----------|-------|
| (a + b) * c / d | 343.3 ns | 916.7 ns | CTQuantity **2.7x faster** |

### Key Findings

1. **CTQuantity excels at creation** - No need to clone `Unit` objects, resulting in ~20x faster creation
2. **CTQuantity is faster for arithmetic** - No runtime dimension checking overhead
3. **Quantity is faster for unit conversion** - Runtime implementation is better optimized
4. **CTQuantity shows significant advantage in batch operations** - Especially for operations producing new dimensions

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](./../LICENSE) for details.
