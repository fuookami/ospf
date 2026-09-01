# ospf-rust-quantities

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-quantities` 把 Kotlin `ospf-kotlin-quantities` 映射为 Rust 物理量、量纲和单位系统。它通过 `Quantity<V, U>` 支持编译期与运行期单位模式，并与 `ospf-rust-math` 集成以支持物理量感知的符号计算。

## 作用范围

本 crate 拥有物理量纲、单位制、单位转换、物理量算术、编译期/运行期物理量模式和物理量感知符号 helper。

明确非目标：

1. 优化 solver 建模或 framework 编排。
2. 领域专用业务单位，除非它们成为通用可复用单位。
3. 除 quantity/unit 数据边界外的序列化协议所有权。

物理量、量纲和单位系统

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 概述 / Overview

`ospf-rust-quantities` 是一个 Rust 库，提供物理量、量纲和单位的完整实现，通过统一的类型系统支持运行时和编译时两种模式。

`ospf-rust-quantities` is a Rust library providing a complete implementation of physical quantities, dimensions, and units, supporting both runtime and compile-time modes through a unified type system.

### 核心特性 / Key Features

- **统一类型系统 / Unified Type System**: 单一 `Quantity<V, U>` 类型同时支持编译时和运行时模式
- **零成本抽象 / Zero-cost Abstraction**: 编译时单位类型（`Quantity<V, U: CTUnit>`）在编译期完成所有计算，无运行时开销
- **编译时量纲检查 / Compile-time Dimension Checking**: 编译时物理量的不匹配量纲操作会导致编译错误
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

## 泛型数值边界

`Quantity<V, U>` 对值类型 `V` 泛型化。支持示例包括 `f64`、`BigDecimal`，以及通过 math 集成支持的 rational/symbolic value。数值类型转换应保留在调用方或 adapter 边界显式完成。

## 物理量边界

单位静态已知时，应通过编译期单位类型保留物理量维度；单位运行时动态选择时，应通过 runtime `Unit` 保留维度。裸值只适合无量纲 scale factor、计数和低层 adapter 边界。

## 快速开始 / Quick Start

### 编译时物理量 / Compile-time Quantities

编译时物理量的单位类型在编译时确定，提供零成本抽象和编译时量纲检查。

Compile-time quantities have their unit types determined at compile time, providing zero-cost abstraction and compile-time dimension checking.

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Second};
use ospf_rust_quantities::unit::CTUnit;
use bigdecimal::BigDecimal;

// 创建编译时物理量（使用 new_ct 方法）/ Create compile-time quantity (using new_ct method)
let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(1000));

// 编译时单位转换 / Compile-time unit conversion
let length_km: Quantity<BigDecimal, Kilometer> = length.to();
assert_eq!(length_km.value, BigDecimal::from(1));

// 物理量运算产生新单位类型 / Quantity operations produce new unit types
let time: Quantity<BigDecimal, Second> = Quantity::new_ct(BigDecimal::from(10));
let velocity = length_km / time; // 类型: Quantity<BigDecimal, CTUnitDiv<Kilometer, Second>>
                                  // Type: Quantity<BigDecimal, CTUnitDiv<Kilometer, Second>>
```

### 运行时物理量 / Runtime Quantities

运行时物理量的单位在运行时确定，支持动态单位转换。

Runtime quantities have their units determined at runtime, supporting dynamic unit conversion.

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Kilometer, Kilogram};
use ospf_rust_quantities::unit::{CTUnit, Unit};
use bigdecimal::BigDecimal;

// 创建运行时物理量（使用 Unit 类型）/ Create runtime quantity (using Unit type)
let length: Quantity<BigDecimal, Unit> = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());

// 运行时单位转换 / Runtime unit conversion
let length_km = length.to_unit(&Kilometer::INSTANT.clone()).unwrap();
assert_eq!(length_km.value, BigDecimal::from(1));

// 物理量运算 / Quantity operations
let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());
let momentum = &length * &mass; // 产生新量纲 / Produces new dimension
```

### 模式转换 / Converting Between Modes

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::{CTUnit, Unit};
use bigdecimal::BigDecimal;

// 创建编译时物理量 / Create compile-time quantity
let ct_length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));

// 转换为运行时物理量 / Convert to runtime quantity
let rt_length: Quantity<BigDecimal, Unit> = ct_length.to_runtime();
assert_eq!(rt_length.unit.symbol(), "m");
```

### `std::time::Duration` 互操作

带类型的秒物理量通过 `IntoDuration` 和 `FromDuration` 支持纳秒边界上的精确转换：

```rust
use ospf_rust_quantities::{FromDuration, IntoDuration, Quantity};
use ospf_rust_quantities::unit::derived::Second;
use bigdecimal::BigDecimal;
use std::time::Duration;

let seconds: Quantity<BigDecimal, Second> =
    Quantity::new_ct("42.123456789".parse().unwrap());
let duration = seconds.into_duration().unwrap();
assert_eq!(duration, Duration::new(42, 123_456_789));

let restored = Quantity::<BigDecimal, Second>::from_duration(duration).unwrap();
assert_eq!(restored.value, seconds.value);
```

`BigDecimal` 路径不会经过 `f64`：值必须非负、在 `Duration` 可表示范围内，并且能精确表示到纳秒（纳秒以外的尾随零可以保留）。`f64` 路径要求值有限且非负，发生精度或范围损失时返回错误。`Duration` 转 `BigDecimal` 使用秒和纳秒组成部分精确构造；`Duration` 转 `f64` 仍受 IEEE-754 精度限制。

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
| `Quantity<V, U>` | 统一物理量类型，`U` 可以是 `Unit`（运行时）或 `CTUnit` 类型（编译时）/ Unified quantity type, `U` can be `Unit` (runtime) or a `CTUnit` type (compile-time) |
| `Quantity<V, Unit>` | 运行时物理量，单位在运行时确定 / Runtime quantity with unit determined at runtime |
| `Quantity<V, U: CTUnit>` | 编译时物理量，单位类型在编译时确定 / Compile-time quantity with unit type determined at compile time |
| `Unit` | 运行时单位 / Runtime unit |
| `CTUnit` | 编译时单位 trait / Compile-time unit trait |
| `DerivedQuantity` | 导出量纲 / Derived dimension |
| `CTDerivedQuantity` | 编译时导出量纲 trait / Compile-time derived dimension trait |
| `Scale` | 单位比例尺 / Unit scale |
| `QuantityTrait` | 所有物理量类型的统一接口 / Unified interface for all quantity types |

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
├── quantity           # 物理量（统一类型）/ Quantities (unified type)
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

## 物理量符号运算 / Physical Quantity Symbolic Computation

`ospf-rust-quantities` 与 `ospf-rust-math` 集成，支持物理量的符号运算。这允许您创建带有物理单位的多项式，结合类型安全的量纲分析和符号数学的优势。

`ospf-rust-quantities` integrates with `ospf-rust-math` to support symbolic computation with physical quantities. This allows you to create polynomials with physical units, combining the benefits of type-safe dimensional analysis with symbolic mathematics.

### 支持的值类型 / Supported Value Types

| 值类型 / Value Type | Crate | 用途 / Use Case |
|--------------------|-------|----------------|
| `f64` | `std` | 快速数值计算 / Fast numerical computation |
| `BigDecimal` | `bigdecimal` | 高精度十进制，金融计算 / High-precision decimal for financial calculations |
| `BigRational` | `num-rational` | 精确有理数，符号计算 / Exact rational numbers for symbolic computation |

### 使用示例 / Usage Examples

```rust
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_quantities::unit::CTUnit;
use ospf_rust_math::symbol::polynomial::Linear;
use ospf_rust_math::symbol::symbol::OwnedSymbol;
use bigdecimal::BigDecimal;
use num_rational::BigRational;

// 物理量符号 / Physical quantity symbol
let x: Quantity<OwnedSymbol, Meter> = Quantity::new_ct(OwnedSymbol::new("x"));
let y: Quantity<OwnedSymbol, Meter> = Quantity::new_ct(OwnedSymbol::new("y"));

// 物理量多项式（f64 值类型）/ Physical quantity polynomial with f64
let distance: Quantity<Linear<f64>, Meter> = Quantity::new_ct(2.0 * x.clone() + 3.0 * y.clone());

// 使用 BigDecimal / With BigDecimal
let coef_bd = BigDecimal::from(2);
let distance_bd: Quantity<Linear<BigDecimal>, Meter> = Quantity::new_ct(coef_bd * x.clone());

// 使用 BigRational / With BigRational
let coef_br = BigRational::new(3.into(), 2.into()); // 3/2
let distance_br: Quantity<Linear<BigRational>, Meter> = Quantity::new_ct(coef_br * x);
```

### 类型定义 / Type Definitions

| 类型 / Type | 说明 / Description |
|------------|-------------------|
| `Quantity<OwnedSymbol, U>` | 物理量符号 / Physical quantity symbol |
| `Quantity<LinearMonomial<T>, U>` | 物理量线性单项式 / Physical quantity linear monomial |
| `Quantity<Linear<T>, U>` | 物理量线性多项式 / Physical quantity linear polynomial |
| `Quantity<QuadraticMonomial<T>, U>` | 物理量二次单项式 / Physical quantity quadratic monomial |
| `Quantity<Quadratic<T>, U>` | 物理量二次多项式 / Physical quantity quadratic polynomial |
| `Quantity<Canonical<T, E>, U>` | 物理量标准多项式 / Physical quantity canonical polynomial |

### 编译时与运行时单位 / Compile-time vs Runtime Units

符号运算同时支持编译时和运行时单位：

Symbolic computation works with both compile-time and runtime units:

```rust
// 编译时（零成本）/ Compile-time (zero-cost)
let ct_symbol: Quantity<OwnedSymbol, Meter> = Quantity::new_ct(OwnedSymbol::new("x"));

// 运行时（灵活）/ Runtime (flexible)
let rt_symbol: Quantity<OwnedSymbol, Unit> = 
    Quantity::new(OwnedSymbol::new("x"), Meter::INSTANT.clone());
```

## 编译时量纲检查 / Compile-time Dimension Checking

编译时物理量会在编译时检查量纲匹配。以下代码会导致编译错误：

Compile-time quantities check dimension matching at compile time. The following code will cause a compile error:

```rust,compile_fail
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, Second};
use bigdecimal::BigDecimal;

let length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
let time: Quantity<BigDecimal, Second> = Quantity::new_ct(BigDecimal::from(5));

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
use ospf_rust_quantities::unit::{CTUnit, Unit};
use bigdecimal::BigDecimal;

let length: Quantity<BigDecimal, Unit> = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
let mass_unit = Kilogram::INSTANT.clone();

// 尝试转换到不同量纲的单位 / Try to convert to unit with different dimension
match length.to_unit(&mass_unit) {
    Ok(q) => println!("转换成功 / Conversion succeeded: {}", q.value),
    Err(e) => println!("错误 / Error: {}", e.msg()), // 输出量纲不匹配错误
                                                         // Prints dimension mismatch error
}
```

## 性能基准测试 / Performance Benchmarks

以下基准测试比较了编译时物理量（`Quantity<V, CTUnit>`）和运行时物理量（`Quantity<V, Unit>`）的性能。运行命令：

The following benchmarks compare the performance of compile-time (`Quantity<V, CTUnit>`) and runtime (`Quantity<V, Unit>`) physical quantities. Run with:

```bash
cargo bench --package ospf-rust-quantities --bench quantity_bench
```

### 主要结论 / Key Findings

1. **编译时物理量创建更快** - 无需克隆 `Unit` 对象，创建速度约快 20 倍
2. **编译时物理量算术运算更快** - 无运行时量纲检查开销
3. **运行时物理量单位转换更快** - 运行时实现优化更好
4. **编译时物理量批量操作优势明显** - 特别是产生新量纲的操作

### 何时使用哪种模式 / When to Use Which Mode

| 场景 / Scenario | 推荐模式 / Recommended Mode |
|----------------|---------------------------|
| 已知单位的性能关键代码 / Performance-critical code with known units | 编译时 / Compile-time (`Quantity<V, U: CTUnit>`) |
| 运行时动态选择单位 / Dynamic unit selection at runtime | 运行时 / Runtime (`Quantity<V, Unit>`) |
| 需要编译时量纲安全 / Need compile-time dimension safety | 编译时 / Compile-time |
| 与用户提供的单位互操作 / Interoperability with user-provided units | 运行时 / Runtime |
| 混合场景 / Mixed scenarios | 使用编译时，需要时转换为运行时 / Use compile-time and convert to runtime when needed |

## 本地验证

```powershell
cargo check -p ospf-rust-quantities
cargo test -p ospf-rust-quantities
cargo bench --package ospf-rust-quantities --bench quantity_bench
```

## 相关模块

- [根 README](../README_ch.md)
- [Math README](../ospf-rust-math/README_ch.md)
- [Kotlin quantities README](../../ospf-kotlin/ospf-kotlin-quantities/README_ch.md)

## 许可证 / License

基于 MIT 许可证发布。详情请参阅 [LICENSE](../LICENSE)。

Licensed under the MIT License. See [LICENSE](../LICENSE) for details.
