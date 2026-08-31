# monomial

:us: [English](README.md) | :cn: 简体中文

单项式类型，用于表示多项式的单项组件。本模块定义了三种具有不同代数结构的单项式类型。

## 核心类型

| 类型 | 描述 |
|------|------|
| `LinearMonomial<T>` | 线性单项式：`c * S`（系数乘符号） |
| `QuadraticMonomial<T>` | 二次单项式：`c * S1 * S2` 或 `c * S1^2` |
| `CanonicalMonomial<T, E>` | 标准单项式：`c * S1^n1 * S2^n2 * ...` |

## 代数形式

- **LinearMonomial**：最简单的形式，包含一个符号和一个系数
  - 数学表示：`c * x`
  - 用于线性规划和仿射表达式

- **QuadraticMonomial**：两符号乘积或平方符号
  - 数学表示：`c * x * y` 或 `c * x^2`
  - 用于二次规划和凸优化

- **CanonicalMonomial**：广义幂乘积形式
  - 数学表示：`c * x^n * y^m * ...`
  - 用于多项式优化和代数几何

## 使用示例

```rust
use ospf_rust_math::symbol::monomial::{LinearMonomial, QuadraticMonomial, CanonicalMonomial};
use ospf_rust_math::symbol::OwnedSymbol;

// 创建线性单项式：2.5 * x
let x = OwnedSymbol::new("x");
let linear = LinearMonomial::new(2.5, x);

// 创建二次单项式：3.0 * x * y
let y = OwnedSymbol::new("y");
let quadratic = QuadraticMonomial::new(3.0, x.clone(), y);

// 创建标准单项式：2.0 * x^3 * y^2
let canonical = CanonicalMonomial::with_powers(2.0, [(x, 3), (y, 2)]);
```

## 许可证

本项目采用 MIT 许可证。
