# Symbol - 符号运算模块

[English](README.md)

## 概述

本模块提供符号运算功能，专为线性规划 (LP)、二次规划 (QP) 和一般非线性优化问题设计。

### 主要特性

- **泛型值类型**：支持 `f64`、`BigDecimal`、`BigRational` 以及任何实现 `Add + Sub + Mul + Div` 的类型
- **动态符号数量**：变量数量在运行时确定
- **稀疏表示**：仅存储非零项
- **零成本抽象**：通过 `Quantity<V, U: CTUnit>` 实现编译时单位集成

## 模块结构

```
symbol/
├── symbol/          # 符号定义 (SymbolId, DynSymbol, Symbol, OwnedSymbol)
├── monomial/        # 单项式 (LinearMonomial, QuadraticMonomial, CanonicalMonomial)
├── polynomial/      # 多项式 (Linear, Quadratic, Canonical)
├── inequality/      # 不等式 (Comparison, LinearInequality, 等)
├── operation/       # 运算操作 (Evaluate, Differentiate, ToLaTeX, 等)
├── macros/          # 构造宏
├── parser/          # 表达式解析器（可选，需要 "parser" feature）
└── serde.rs         # 序列化（可选，需要 "serde" feature）
```

## 核心类型

### 符号

| 类型            | 描述                             |
|---------------|--------------------------------|
| `SymbolId`    | 静态符号标识符 trait                  |
| `SymbolDynId` | 动态标识符，支持高维符号                   |
| `DynSymbol`   | 运行时多态符号 trait                  |
| `Symbol`      | 带关联 `Id` 类型的 trait             |
| `OwnedSymbol` | 拥有所有权的包装器 `Box<dyn DynSymbol>` |

### 单项式

| 类型                        | 形式                        | 示例         |
|---------------------------|---------------------------|------------|
| `LinearMonomial<T>`       | `c * S`                   | `2x`       |
| `QuadraticMonomial<T>`    | `c * S1 * S2` 或 `c * S`   | `xy`, `2x` |
| `CanonicalMonomial<T, E>` | `c * S1^n1 * S2^n2 * ...` | `x²y³`     |

### 多项式

| 类型                | 形式                       | 示例                 |
|-------------------|--------------------------|--------------------|
| `Linear<T>`       | `Σ cᵢSᵢ + b`             | `2x + 3y + 1`      |
| `Quadratic<T>`    | `Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e` | `x² + 2xy + y + 1` |
| `Canonical<T, E>` | `Σ cᵢ * ∏ Sⱼ^nⱼ + d`     | `x²y³ + 2x + 1`    |

### 不等式

| 类型                          | 形式                   | 示例             |
|-----------------------------|----------------------|----------------|
| `LinearInequality<T>`       | `Linear op value`    | `2x + 3y ≤ 5`  |
| `QuadraticInequality<T>`    | `Quadratic op value` | `x² + y² ≤ 10` |
| `CanonicalInequality<T, E>` | `Canonical op value` | `x²y³ ≥ 1`     |

## 使用示例

### 基本符号运算

```rust
use ospf_rust_math::symbol::{OwnedSymbol, Linear, Quadratic};
use ospf_rust_math::symbol::test_utils::SimpleSymbol;

// 创建符号
let x = OwnedSymbol::new(SimpleSymbol::new("x"));
let y = OwnedSymbol::new(SimpleSymbol::new("y"));

// 线性表达式：2x + 3y + 1
let linear = 2.0 * x.clone() + 3.0 * y.clone() + 1.0;

// 二次表达式：x² + 2xy + y²
let quad = x.clone() * x.clone() + 2.0 * x * y;
```

### 使用构造宏

```rust
use ospf_rust_math::symbol::{symbols, linear, quadratic, linear_inequality};

// 定义符号
symbols!(x, y, z);

// 构造多项式
let l = linear!(2.0 * x + 3.0 * y + 1.0);
let q = quadratic!(x * x + 2.0 * x * y + 1.0);

// 构造不等式：2x + 3y ≤ 5
let ineq = linear_inequality!(2.0 * x + 3.0 * y <= 5.0);
```

### 求值与微分

```rust
use ospf_rust_math::symbol::operation::{Evaluate, Differentiate};
use std::collections::HashMap;

// 求值多项式
let values = HashMap::from([
    (x.clone(), 2.0),
    (y.clone(), 3.0),
]);
let result = linear.evaluate(&values); // 2*2 + 3*3 + 1 = 14

// 计算偏导数
let dx = linear.partial_derivative(&x); // 2.0
let grad = linear.gradient(&[x, y]);    // [2.0, 3.0]
```

### 矩阵形式转换

```rust
use ospf_rust_math::symbol::operation::ToMatrixForm;

// 转换为矩阵形式：x^T Q x + c^T x + d
let matrix_form = quad.to_matrix_form(&[x, y]);
// Q: 2x2 矩阵, c: 2x1 向量, d: 标量
```

### LaTeX 输出

```rust
use ospf_rust_math::symbol::operation::ToLaTeX;

let latex = linear.to_latex(); // "2 x + 3 y + 1"
```

### 编译求值

```rust
use ospf_rust_math::symbol::operation::{CompileEval, CompileGradient};

// 编译为高效函数
let eval_fn = linear.compile_eval(&[x, y]);
let result = eval_fn(&[2.0, 3.0]); // 快速求值

let grad_fn = linear.compile_gradient(&[x, y]);
let gradient = grad_fn(&[2.0, 3.0]); // [2.0, 3.0]
```

## 已实现功能

| 功能        | 状态 | 描述                                                               |
|-----------|----|------------------------------------------------------------------|
| 符号定义      | ✅  | `SymbolId`, `DynSymbol`, `Symbol`, `OwnedSymbol`                 |
| 线性单项式/多项式 | ✅  | `LinearMonomial<T>`, `Linear<T>`                                 |
| 二次单项式/多项式 | ✅  | `QuadraticMonomial<T>`, `Quadratic<T>`                           |
| 标准单项式/多项式 | ✅  | `CanonicalMonomial<T, E>`, `Canonical<T, E>`                     |
| 不等式       | ✅  | `LinearInequality`, `QuadraticInequality`, `CanonicalInequality` |
| 求值        | ✅  | `Evaluate`, `EvaluateOrdered` traits                             |
| 微分        | ✅  | `Differentiate`, `SecondOrderDifferentiate` traits               |
| 矩阵形式      | ✅  | `ToMatrixForm` trait                                             |
| LaTeX 输出  | ✅  | `ToLaTeX` trait                                                  |
| 类型转换      | ✅  | `ToLinear`, `ToQuadratic`, `ToCanonical` traits                  |
| 序列化       | ✅  | serde 支持（可选 feature）                                             |
| 解析器       | ✅  | 表达式解析（可选 feature）                                                |
| 构造宏       | ✅  | `symbols!`, `linear!`, `quadratic!` 等                            |
| 编译求值      | ✅  | `CompileEval`, `CompileGradient` traits                          |
| 项合并优化     | ✅  | `CombineTerms` trait                                             |

## 扩展点

本模块设计为可扩展的。用户可以通过以下方式扩展系统：

### 1. 自定义符号类型

为自定义符号类型实现 `DynSymbol` 和 `Symbol` traits：

```rust
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

#[derive(Debug, Clone)]
pub struct MySymbol {
    id: usize,
    name: String,
    // 自定义字段...
}

impl DynSymbol for MySymbol {
    fn name(&self) -> &str { &self.name }
    fn display_name(&self) -> &str { &self.name }
    fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
    // ... 实现其他方法
}
```

### 2. 复合运算符

通过实现 `CompositeOperator` trait 创建自定义复合运算符（如 `abs`、`max`、`min`）：

```rust
/// 复合运算符 trait，用于用户定义的运算符
pub trait CompositeOperator: Clone + Debug + Eq + Hash + Any {
    fn name(&self) -> &str;
    fn id(&self) -> u64;
}

/// 复合符号，将运算符应用于内部表达式
pub struct CompositeSymbol<Op: CompositeOperator, Inner> {
    operator: Op,
    inner: Inner,
}
```

这使得可以表示 `|2x + 3y|` 或 `max(x, y)` 等表达式。

### 3. 自定义值类型

任何实现了所需 traits 的类型都可以作为值类型：

```rust
// 任意精度支持
use bigdecimal::BigDecimal;
let precise: Linear<BigDecimal> = /* ... */;

// 精确有理数运算支持
use num_rational::BigRational;
let exact: Linear<BigRational> = /* ... */;

// 区间运算支持
use ospf_rust_math::algebra::value_range::ValueRange;
let interval: Linear<ValueRange<f64>> = /* ... */;
```

### 4. 物理量集成

与 `ospf-rust-quantities` 结合，支持量纲检查的物理量：

```rust
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

// 编译时量纲检查
let distance: Quantity<Linear<f64>, Meter> = /* ... */;

// 运行时量纲检查
let runtime_distance: Quantity<Linear<f64>, Unit> = /* ... */;
```

### 5. 多维数组集成

与 `ospf-rust-multiarray` 结合，支持向量化操作：

```rust
use ospf_rust_multiarray::MultiArray;

// 线性多项式向量
let equations: MultiArray<Linear<f64>, Shape<2>> = /* ... */;

// 沿轴快速求和
let sum = equations.sum_axis(0)?;
```

### 6. 鲁棒优化扩展（规划中）

本模块为鲁棒优化提供基础设施：

| 规划功能                     | 描述       |
|--------------------------|----------|
| `UncertaintySet` trait   | 不确定集合抽象  |
| `BoxUncertainty`         | 盒式不确定集   |
| `RobustLinearConstraint` | 鲁棒线性约束   |
| 对偶转化                     | 鲁棒优化对偶问题 |

> **注**：鲁棒优化功能将在独立的后续模块中实现。

## 规划功能

| 功能     | 优先级 | 描述       |
|--------|-----|----------|
| JIT 编译 | 低   | 原生代码生成求值 |
| 鲁棒优化   | 规划中 | 不确定集合抽象  |

## 依赖关系

```
ospf-rust-math (symbol)
├── ospf-rust-base (基础类型)
├── ospf-rust-multiarray (多维数组，可选)
└── ospf-rust-quantities (物理量，可选)
```

## Feature Flags

| Flag     | 描述           |
|----------|--------------|
| `serde`  | 启用序列化/反序列化支持 |
| `parser` | 启用表达式解析      |

## 参考资料

- API 文档可在源代码注释中找到
