# inequality

:us: English | :cn: [简体中文](README_ch.md)

不等式类型，用于优化问题中的约束表示。本模块提供与多项式形式对应的不等式类型。

## 核心类型

| 类型 | 描述 |
|------|------|
| `Comparison` | 比较运算符：`<=`, `<`, `>=`, `>` |
| `LinearInequality<T>` | 线性约束：`Linear <=, <, >=, > constant` |
| `QuadraticInequality<T>` | 二次约束：`Quadratic <=, <, >=, > constant` |
| `CanonicalInequality<T, E>` | 标准约束：`Canonical <=, <, >=, > constant` |

## 比较运算符

`Comparison` 枚举定义了四种比较方向：
- `Le` - 小于等于（`<=`）
- `Lt` - 小于（`<`）
- `Ge` - 大于等于（`>=`）
- `Gt` - 大于（`>`）

## 约束形式

- **LinearInequality**：线性约束，用于 LP 和 MILP
  - 示例：`2*x + 3*y <= 10`

- **QuadraticInequality**：二次约束，用于 QP
  - 示例：`x^2 + y^2 <= 1.0`（单位圆盘约束）

- **CanonicalInequality**：广义多项式约束
  - 示例：`x^3 + 2*x*y^2 >= 5`

## 使用示例

```rust
use ospf_rust_math::symbol::inequality::{Comparison, LinearInequality, QuadraticInequality};
use ospf_rust_math::symbol::polynomial::{Linear, Quadratic};
use ospf_rust_math::symbol::OwnedSymbol;

// 创建线性不等式：2*x + 3*y <= 10
let x = OwnedSymbol::new("x");
let y = OwnedSymbol::new("y");
let linear_poly = Linear::from_terms([(2.0, x), (3.0, y)], 0.0);
let linear_ineq = LinearInequality::new(linear_poly, Comparison::Le, 10.0);

// 创建二次不等式：x^2 + y^2 <= 1.0
let quadratic_poly = Quadratic::from_quadratic_terms(
    [(1.0, x.clone(), x.clone()), (1.0, y.clone(), y.clone())],
    [],
    0.0
);
let quadratic_ineq = QuadraticInequality::new(quadratic_poly, Comparison::Le, 1.0);
```

## 许可证

本项目采用 MIT 许可证。