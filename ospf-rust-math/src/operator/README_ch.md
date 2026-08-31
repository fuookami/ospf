# operator

:us: [English](README.md) | :cn: 简体中文

## 概述

`operator` 模块提供各类数学运算的 trait 定义，包括基础运算、指数对数、幂运算、三角函数、引用运算和容差比较。

主要特性：
- 完整的数学运算符 trait
- 支持精度控制的指数和对数运算
- 高效的引用算术运算
- 基于容差的相等和排序比较

## 子模块

| 子模块 | 描述 |
|--------|------|
| `abs` | 绝对值运算（`Abs`, `AbsRef`） |
| `contains` | 包含检查运算（`Contains`） |
| `exp_log` | 指数和对数运算（`Exp`, `Log`, `ExpWithPrecision`, `LogWithPrecision`） |
| `exponent` | 指数标记 trait（`Exponent`） |
| `power` | 幂运算（`Pow`, `PowF`, `PowFWithPrecision`） |
| `reciprocal` | 倒数运算（`Reciprocal`, `ReciprocalRef`） |
| `ref_additive` | 引用加减运算（`AddRef`, `SubRef`, `NegRef`） |
| `ref_multiplicative` | 引用乘除运算（`MulRef`, `DivRef`） |
| `one_zero_ref` | 常量引用 trait（`ZeroRef`, `OneRef`, `NegOneRef`, `Two`） |
| `tolerance` | 容差比较（`Tolerance`, `TolerancedEq`, `TolerancedOrd`） |
| `trigonometry` | 三角函数和双曲函数（`Trigonometry`） |

## 主要类型

### 基础运算

| Trait | 描述 |
|-------|------|
| `Abs` | 绝对值运算 |
| `AbsRef` | 返回引用的绝对值运算 |
| `Reciprocal` | 倒数（1/x）运算 |
| `ReciprocalRef` | 返回引用的倒数运算 |
| `Contains` | 检查一个值是否包含另一个值 |

### 指数与对数

| Trait | 描述 |
|-------|------|
| `Exp` | 自然指数（e^x） |
| `ExpWithPrecision` | 支持精度控制的自然指数 |
| `Log` | 对数运算 |
| `LogWithPrecision` | 支持精度控制的对数运算 |
| `Exponent` | 指数类型的标记 trait |

### 幂运算

| Trait | 描述 |
|-------|------|
| `Pow` | 整数幂运算 |
| `PowF` | 浮点幂运算 |
| `PowFWithPrecision` | 支持精度控制的浮点幂运算 |

### 三角函数

| Trait | 描述 |
|-------|------|
| `Trigonometry` | 完整的三角函数运算，包括 `sin`、`cos`、`tan`、`asin`、`acos`、`atan`、`sinh`、`cosh`、`tanh` 及其反函数/双曲变体 |

### 引用运算

| Trait | 描述 |
|-------|------|
| `AddRef` | 返回引用的加法运算 |
| `SubRef` | 返回引用的减法运算 |
| `NegRef` | 返回引用的取负运算 |
| `MulRef` | 返回引用的乘法运算 |
| `DivRef` | 返回引用的除法运算 |
| `ZeroRef` | 零常量引用 |
| `OneRef` | 一常量引用 |
| `NegOneRef` | 负一常量引用 |
| `Two` | 二常量引用 |

### 容差比较

| Trait | 描述 |
|-------|------|
| `Tolerance` | 定义比较的容差 |
| `TolerancedEq` | 带容差的相等比较 |
| `TolerancedOrd` | 带容差的排序比较 |

## 使用示例

```rust
use ospf_rust_math::operator::{Abs, Exp, Pow, Trigonometry};

// 绝对值
let x = -5.0_f64;
assert_eq!(x.abs(), 5.0);

// 指数
let e_squared = 2.0_f64.exp();
assert!((e_squared - std::f64::consts::E.powi(2)).abs() < 1e-10);

// 幂运算
let result = 2.0_f64.pow(3);
assert_eq!(result, 8.0);

// 三角函数
let pi = std::f64::consts::PI;
assert!((pi.sin() - 0.0).abs() < 1e-10);
```

## 许可证

MIT License
