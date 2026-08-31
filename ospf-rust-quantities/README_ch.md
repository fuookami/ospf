# ospf-rust-quantities

物理量、量纲和单位系统 / Physical quantities, dimensions and units system

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 概述 / Overview

`ospf-rust-quantities` 是一个 Rust 库，提供物理量、量纲和单位的完整实现，支持运行时和编译时两种模式。

`ospf-rust-quantities` is a Rust library providing a complete implementation of physical quantities, dimensions, and units, supporting both runtime and compile-time modes.

### 核心特性 / Key Features

- **零成本抽象 / Zero-cost Abstraction**: 编译时单位类型在编译期完成所有计算，无运行时开销
- **编译时量纲检查 / Compile-time Dimension Checking**: 不匹配的量纲操作会导致编译错误
- **灵活的单位转换 / Flexible Unit Conversion**: 支持相同量纲单位之间的自动转换
- **预定义单位制 / Predefined Unit Systems**: 内置 SI、MKS、CGS 等单位制
- **泛型值类型 / Generic Value Types**: 支持 `BigDecimal`、`f64` 等多种数值类型
- **完整的中英双语文档 / Complete Bilingual Documentation**

## 安装 / Installation

在 `Cargo.toml` 中添加依赖 / Add dependency in `Cargo.toml`:

```toml
[dependencies]
ospf-rust-quantities = "0.1.0"
```

## 快速开始 / Quick Start

### 编译时物理量 / Compile-time Quantities

编译时物理量的单位类型在编译时确定，提供零成本抽象和编译时量纲检查。

Compile-time quantities have their unit types determined at compile time, providing zero-cost abstraction and compile-time dimension checking.

```rust
use ospf_rust_quantities::quantity::CTQuantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Second};
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

// 创建编译时物理量 / Create compile-time quantity
let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(1000));

// 编译时单位转换 / Compile-time unit conversion
let length_km: CTQuantity<BigDecimal, Kilometer> = length.to();
assert_eq!(length_km.value, BigDecimal::from(1));

// 物理量运算产生新单位类型 / Quantity operations produce new unit types
let time: CTQuantity<BigDecimal, Second> = CTQuantity::new(BigDecimal::from(10));
let velocity = length_km / time; // 类型: CTQuantity<BigDecimal, CTUnitDiv<Kilometer, Second>>
                                  // Type: CTQuantity<BigDecimal, CTUnitDiv<Kilometer, Second>>
```

### 运行时物理量 / Runtime Quantities

运行时物理量的单位在运行时确定，支持动态单位转换。

Runtime quantities have their units determined at runtime, supporting dynamic unit conversion.

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Kilogram};
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

// 创建运行时物理量 / Create runtime quantity
let length = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());

// 运行时单位转换 / Runtime unit conversion
let length_km = length.to_unit(&Kilometer::INSTANT.clone()).unwrap();
assert_eq!(length_km.value, BigDecimal::from(1));

// 物理量运算 / Quantity operations
let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());
let momentum = &length * &mass; // 产生新量纲 / Produces new dimension
```

### 单位制 / Unit Systems

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::system::SI_SYSTEM;
use ospf_rust_quantities::unit::{CTUnit, UnitSystem};
use bigdecimal::BigDecimal;

let length = Quantity::new(BigDecimal::from(100), Meter::INSTANT.clone());

// 转换为单位制的标准单位 / Convert to standard unit of the system
let standard = length.to_standard_unit(SI_SYSTEM.as_ref()).unwrap();
assert_eq!(standard.unit.symbol(), "m"); // SI 中长度的标准单位是米
                                          // Standard unit for length in SI is meter
```

## API 概览 / API Overview

### 核心类型 / Core Types

| 类型 / Type | 说明 / Description |
|------------|-------------------|
| `CTQuantity<V, U>` | 编译时物理量，单位类型 `U` 在编译时确定 / Compile-time quantity with unit type `U` determined at compile time |
| `Quantity<V>` | 运行时物理量，单位在运行时确定 / Runtime quantity with unit determined at runtime |
| `Unit` | 运行时单位 / Runtime unit |
| `CTUnit` | 编译时单位 trait / Compile-time unit trait |
| `DerivedQuantity` | 导出量纲 / Derived dimension |
| `CTDerivedQuantity` | 编译时导出量纲 trait / Compile-time derived dimension trait |
| `Scale` | 单位比例尺 / Unit scale |

### 模块结构 / Module Structure

```
ospf_rust_quantities
├── dimension          # 量纲 / Dimensions
│   ├── fundamental    # 基本量纲 / Fundamental dimensions
│   └── derived        # 导出量纲 / Derived dimensions
├── unit               # 单位 / Units
│   ├── physical_unit  # 核心单位类型 / Core unit types
│   ├── system         # 单位制（SI、MKS、CGS）/ Unit systems
│   └── derived        # 预定义导出单位 / Predefined derived units
├── quantity           # 物理量 / Quantities
│   ├── ct_quantity    # 编译时物理量 / Compile-time quantities
│   └── quantity       # 运行时物理量 / Runtime quantities
├── scale              # 比例尺 / Scales
└── error              # 错误类型 / Error types
```

### 预定义单位 / Predefined Units

| 类别 / Category | 单位 / Units |
|----------------|-------------|
| 长度 / Length | Meter, Kilometer, Centimeter, Millimeter, Inch, Foot, Yard, Mile |
| 质量 / Mass | Kilogram, Gram, Milligram, Ton |
| 时间 / Time | Second, Minute, Hour, Day |
| 速度 / Velocity | MeterPerSecond, KilometerPerHour |
| 加速度 / Acceleration | MeterPerSecondSquared |
| 力 / Force | Newton, Dyne |
| 能量 / Energy | Joule, Calorie, ElectronVolt |
| 功率 / Power | Watt, Kilowatt, Horsepower |
| 压力 / Pressure | Pascal, Bar, Atmosphere |
| 频率 / Frequency | Hertz, Kilohertz, Megahertz |
| 信息 / Information | Bit, Byte, Kilobyte, Megabyte |

## 编译时量纲检查 / Compile-time Dimension Checking

编译时物理量会在编译时检查量纲匹配。以下代码会导致编译错误：

Compile-time quantities check dimension matching at compile time. The following code will cause a compile error:

```rust,compile_fail
use ospf_rust_quantities::quantity::CTQuantity;
use ospf_rust_quantities::unit::derived::{Meter, Second};
use bigdecimal::BigDecimal;

let length: CTQuantity<BigDecimal, Meter> = CTQuantity::new(BigDecimal::from(10));
let time: CTQuantity<BigDecimal, Second> = CTQuantity::new(BigDecimal::from(5));

// 编译错误：不同量纲的物理量不能相加
// Compile error: quantities with different dimensions cannot be added
let result = length + time; // 错误！/ Error!
```

## 自定义单位 / Custom Units

### 编译时自定义单位 / Compile-time Custom Units

```rust
use ospf_rust_quantities::unit::physical_unit::CTUnit;
use ospf_rust_quantities::dimension::derived_quantity::Length;
use ospf_rust_quantities::scale::Scale;
use once_cell::sync::Lazy;

// 定义自定义长度单位 / Define custom length unit
struct MyUnit;

impl CTUnit for MyUnit {
    const NAME: &'static str = "my_unit";
    const SYMBOL: &'static str = "mu";
    const SCALE: Lazy<Scale> = Lazy::new(|| Scale::from_int(100)); // 100 个基本单位
                                                                     // 100 base units
    type Dimension = Length;
}
```

### 运行时自定义单位 / Runtime Custom Units

```rust
use ospf_rust_quantities::unit::physical_unit::UnitBuilder;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::scale::Scale;

// 使用 UnitBuilder 创建自定义单位 / Create custom unit using UnitBuilder
let custom_unit = UnitBuilder::new(
    Meter::INSTANT.dimension().clone(),
    Scale::from_int(100)
)
.name("custom_unit")
.symbol("cu")
.build();
```

## 错误处理 / Error Handling

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilogram};
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
let mass_unit = Kilogram::INSTANT.clone();

// 尝试转换到不同量纲的单位 / Try to convert to unit with different dimension
match length.to_unit(&mass_unit) {
    Ok(q) => println!("转换成功 / Conversion succeeded: {}", q.value),
    Err(e) => println!("错误 / Error: {}", e.msg()), // 输出量纲不匹配错误
                                                         // Prints dimension mismatch error
}
```

## 性能基准测试 / Performance Benchmarks

以下基准测试比较了编译时物理量（`CTQuantity`）和运行时物理量（`Quantity`）的性能。运行命令：

The following benchmarks compare the performance of compile-time (`CTQuantity`) and runtime (`Quantity`) physical quantities. Run with:

```bash
cargo bench --package ospf-rust-quantities --bench quantity_bench
```

### 基本操作 / Basic Operations

| 操作 / Operation | CTQuantity | Quantity | 比率 / Ratio |
|-----------------|------------|----------|--------------|
| 创建 / Creation | 35.6 ns | 704.8 ns | CTQuantity **快 20 倍 / 20x faster** |
| 加法（引用）/ Addition (reference) | 70.5 ns | 85.0 ns | CTQuantity **快 1.2 倍 / 1.2x faster** |
| 加法（不同单位）/ Addition (different units) | - | 242.9 ns | - |
| 减法（引用）/ Subtraction (reference) | 71.7 ns | 86.4 ns | CTQuantity **快 1.2 倍 / 1.2x faster** |
| 标量乘法 / Scalar Multiplication | 70.3 ns | 81.2 ns | CTQuantity **快 1.2 倍 / 1.2x faster** |
| 标量除法 / Scalar Division | 107.2 ns | 98.1 ns | Quantity **快 1.1 倍 / 1.1x faster** |
| 取负 / Negation | 27.1 ns | 26.5 ns | 相当 / Comparable |

### 物理量运算（产生新量纲）/ Quantity Operations (producing new dimensions)

| 操作 / Operation | CTQuantity | Quantity | 比率 / Ratio |
|-----------------|------------|----------|--------------|
| 乘法 / Multiplication | 69.8 ns | 343.3 ns | CTQuantity **快 4.9 倍 / 4.9x faster** |
| 除法 / Division | 104.7 ns | 403.5 ns | CTQuantity **快 3.9 倍 / 3.9x faster** |

### 单位转换 / Unit Conversion

| 操作 / Operation | CTQuantity | Quantity | 比率 / Ratio |
|-----------------|------------|----------|--------------|
| 米 → 千米 / Meter → Kilometer | 515.2 ns | 171.3 ns | Quantity **快 3 倍 / 3x faster** |
| 千米 → 米 / Kilometer → Meter | 500.4 ns | 116.6 ns | Quantity **快 4.3 倍 / 4.3x faster** |

### 批量操作（10,000 次迭代）/ Batch Operations (10,000 iterations)

| 操作 / Operation | CTQuantity | Quantity | 比率 / Ratio |
|-----------------|------------|----------|--------------|
| 加法 / Addition | 683.9 µs | 955.3 µs | CTQuantity **快 1.4 倍 / 1.4x faster** |
| 乘法 / Multiplication | 690.7 µs | 8.4 ms | CTQuantity **快 12 倍 / 12x faster** |
| 单位转换 / Unit Conversion | 5.57 ms | 1.91 ms | Quantity **快 2.9 倍 / 2.9x faster** |

### 复合运算 / Compound Operations

| 操作 / Operation | CTQuantity | Quantity | 比率 / Ratio |
|-----------------|------------|----------|--------------|
| (a + b) * c / d | 343.3 ns | 916.7 ns | CTQuantity **快 2.7 倍 / 2.7x faster** |

### 主要结论

1. **CTQuantity 创建更快** - 无需克隆 `Unit` 对象，创建速度约快 20 倍
2. **CTQuantity 算术运算更快** - 无运行时量纲检查开销
3. **Quantity 单位转换更快** - 运行时实现优化更好
4. **CTQuantity 批量操作优势明显** - 特别是产生新量纲的操作

## 许可证 / License

根据 Apache License, Version 2.0 许可。有关详细信息，请参阅 [LICENSE](./../LICENSE)。

Licensed under the Apache License, Version 2.0. See [LICENSE](./../LICENSE) for details.
