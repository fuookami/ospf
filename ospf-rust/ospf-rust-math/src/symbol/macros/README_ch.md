# macros

:us: [English](README.md) | :cn: 简体中文

构造宏模块，用于简化单项式、多项式和不等式的构造。提供数学风格宏和旧版构造宏。

## 数学表达式宏（推荐）

### 多项式

| 宏 | 描述 | 示例 |
|------|------|------|
| `lin!` | 线性多项式 | `lin!(2 * x + 3 * y + 1)` |
| `quad!` | 二次多项式 | `quad!(x ^ 2 + 2 * x * y)` |

### 不等式

| 宏 | 描述 | 示例 |
|------|------|------|
| `ineq!` | 线性不等式 | `ineq!(lin!(2 * x) <= 5.0)` |
| `qineq!` | 二次不等式 | `qineq!(quad!(x ^ 2) <= 1.0)` |
| `cineq!` | 标准不等式 | `cineq!(poly >= 0.0)` |

### 约束集合

| 宏 | 描述 | 示例 |
|------|------|------|
| `constraints!` | 约束集合 | `constraints![ineq!(...), ineq!(...)]` |

## 旧版构造宏

| 宏 | 描述 |
|------|------|
| `symbols!` | 创建多个符号 |
| `linear_monomial!` | 构造线性单项式 |
| `quadratic_monomial!` | 构造二次单项式 |
| `linear!` | 构造线性多项式 |
| `quadratic!` | 构造二次多项式 |

## 使用示例

### 数学风格（推荐）

```rust
use ospf_rust_math::{lin, quad, ineq, qineq, constraints};
use ospf_rust_math::symbol::OwnedSymbol;

let x = OwnedSymbol::new("x");
let y = OwnedSymbol::new("y");

// 线性多项式：2*x + 3*y + 1
let linear = lin!(2 * x + 3 * y + 1);

// 二次多项式：x^2 + 2*x*y + 3*y
let quadratic = quad!(x ^ 2 + 2 * x * y + 3 * y);

// 线性不等式：2*x + y <= 10
let constraint1 = ineq!(lin!(2 * x + y) <= 10.0);

// 二次不等式：x^2 + y^2 <= 1
let constraint2 = qineq!(quad!(x ^ 2 + y ^ 2) <= 1.0);

// 约束集合
let constraints = constraints![constraint1, constraint2];
```

### 旧版风格

```rust
use ospf_rust_math::{symbols, linear, quadratic};

// 创建符号
let (x, y, z) = symbols!("x", "y", "z");

// 线性多项式
let linear_poly = linear!(2.0 * x + 3.0 * y + 1.0);

// 二次多项式
let quad_poly = quadratic!(x ^ 2 + 2.0 * x * y);
```

## 注意

宏通过 `#[macro_export]` 自动导出到 crate 根，无需从本模块导入即可直接使用。

## 许可证

本项目采用 MIT 许可证。
