# polynomial

:us: [English](README.md) | :cn: 简体中文

多项式类型，用于表示优化问题中的代数表达式。本模块定义了三种表达能力递增的多项式类型。

## 核心类型

| 类型 | 描述 |
|------|------|
| `Linear<T>` | 线性多项式：`Sum(c_i * S_i) + b` |
| `Quadratic<T>` | 二次多项式：`Sum(c_ij * S_i * S_j) + Sum(d_i * S_i) + e` |
| `Canonical<T, E>` | 标准多项式：`Sum(c_i * Prod(S_j^n_j)) + d` |

## 代数形式

- **Linear**：仿射表达式，包含线性项和常数偏移
  - 数学表示：`c_1 * x_1 + c_2 * x_2 + ... + b`
  - 用于线性规划（LP）和混合整数线性规划（MILP）

- **Quadratic**：二次形式，包含二次项和线性项
  - 数学表示：`x^T * Q * x + c^T * x + e`
  - 用于二次规划（QP）和凸优化

- **Canonical**：广义多项式，支持任意幂次
  - 数学表示：`幂乘积项之和 + 常数`
  - 用于多项式优化和非线性规划

## Kotlin 兼容别名

本模块提供 Kotlin 风格的命名别名：
- `LinearPolynomial<T>` = `Linear<T>`
- `QuadraticPolynomial<T>` = `Quadratic<T>`
- `CanonicalPolynomial<T, E>` = `Canonical<T, E>`

## 使用示例

```rust
use ospf_rust_math::symbol::polynomial::{Linear, Quadratic, Canonical};
use ospf_rust_math::symbol::OwnedSymbol;

// 创建线性多项式：2*x + 3*y + 5
let x = OwnedSymbol::new("x");
let y = OwnedSymbol::new("y");
let linear = Linear::from_terms([(2.0, x), (3.0, y)], 5.0);

// 创建二次多项式：x^2 + 2*x*y + 3*y + 1
let quadratic = Quadratic::from_quadratic_terms(
    [(1.0, x.clone(), x.clone()), (2.0, x, y.clone())],
    [(3.0, y)],
    1.0
);

// 创建标准多项式
let canonical = Canonical::from_terms([...], 0.0);
```

## 许可证

本项目采用 MIT 许可证。
