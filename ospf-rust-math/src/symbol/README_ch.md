# Symbol - 符号运算模块

:us: [English](README.md) | :cn: 简体中文

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
├── expression/      # 运行时表达式系统 (ScalarExpression, BooleanExpression, 求值, 解析)
├── macros/          # 构造宏
├── category/        # 符号分类
├── parser/          # 布尔表达式解析器（可选，需要 "parser" feature）
└── serde.rs         # 序列化（可选，需要 "serde" feature）
```

### expression/ 子模块结构

运行时表达式系统是本模块的核心子模块，支持动态表达式构造、求值、解析和序列化：

```
expression/
├── mod.rs           # 模块注册 + 布尔表达式解析器/序列化/测试
├── property_path.rs # PropertyPath 属性路径 + PathSymbol 路径符号
├── operators.rs     # 操作符枚举 (Unary/Binary/Comparison/PatternMatch/Boolean/NullCheck)
├── value.rs         # ExpressionValue 运行时值类型
├── scalar.rs        # ScalarExpression<T> 标量表达式 AST
├── boolean.rs       # BooleanExpression<T> 布尔表达式 AST
├── dsl.rs           # DSL trait (ScalarExpressionDsl, PathBuilder, BooleanExpressionDsl) + 便捷构造函数
├── evaluation.rs    # EvaluationContext 求值上下文 + evaluate_boolean/evaluate_scalar_expression
├── normalize.rs     # 布尔表达式规范化 (flatten, constant_fold, deduplicate, de_morgan, structural_key)
├── math_functions.rs # ScalarFunctionEvaluator trait + MathFunctionEvaluator (17 个 math.* 函数)
└── scalar_parser.rs # 标量表达式解析器（可选，需要 "parser" feature）
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

### 运行时表达式

expression 子模块提供运行时表达式系统，用于构造、求值、解析和序列化动态表达式。与 polynomial 模块的编译时符号运算不同，expression 模块在运行时动态构造和求值表达式，支持条件分支、函数调用和属性路径引用。

#### 核心类型

| 类型                          | 描述                                                          |
|-------------------------------|---------------------------------------------------------------|
| `PropertyPath`                | 属性路径（如 `user.address.city`），支持分段、父子路径、子路径判断 |
| `PathSymbol`                  | 路径符号，桥接 `PropertyPath` 与 `DynSymbol`                    |
| `ExpressionValue`             | 运行时值枚举（`Null` / `Boolean` / `Number` / `String`）        |
| `ScalarExpression<T>`         | 标量表达式 AST（常量、引用、一元/二元运算、函数、条件、布尔包装） |
| `BooleanExpression<T>`        | 布尔表达式 AST（常量、比较、In、模式匹配、空值检查、And/Or/Not） |
| `PathBuilder<T>`              | 路径构建器，链式构造引用与比较表达式                          |

#### 操作符

| 枚举                     | 值                                                     |
|--------------------------|--------------------------------------------------------|
| `UnaryOperator`          | `Negate`、`Positive`、`Abs`                            |
| `BinaryOperator`         | `Add`、`Subtract`、`Multiply`、`Divide`、`Modulo`、`Power` |
| `ComparisonOperator`     | `Eq`、`Ne`、`Lt`、`Le`、`Gt`、`Ge`                     |
| `PatternMatchMode`       | `Like`、`Exact`、`Prefix`、`Suffix`、`Contains`、`Regex` |
| `BooleanOperator`        | `And`、`Or`                                            |
| `NullCheckType`          | `IsNull`、`IsNotNull`                                  |

#### 标量表达式变体

`ScalarExpression<T>` 支持以下变体：

| 变体               | 描述                                                |
|--------------------|-----------------------------------------------------|
| `Constant`         | 常量值                                              |
| `Reference`        | 属性路径引用                                        |
| `SymbolReference`  | 动态符号引用                                        |
| `Unary`            | 一元操作（取负、正号、绝对值）                      |
| `Binary`           | 二元操作（加减乘除、取模、幂运算）                  |
| `Function`         | 函数调用（如 `abs`、`math.sqrt`）                   |
| `Conditional`      | 条件表达式（`if/then/else` 或三元 `?:`）            |
| `Boolean`          | 布尔包装表达式（将布尔表达式作为标量值）            |
| `Custom`           | 自定义表达式（带 payload 与描述）                   |

#### 布尔表达式变体

`BooleanExpression<T>` 支持以下变体：

| 变体             | 描述                                       |
|------------------|--------------------------------------------|
| `Constant`       | 三值逻辑常量（`True` / `False` / `Unknown`）|
| `Comparison`     | 比较表达式（两个标量的比较运算）           |
| `In`             | 集合成员判断                               |
| `PatternMatch`   | 模式匹配（`like`、`regex`、前缀/后缀/包含）|
| `NullCheck`      | 空值检查（`is null` / `is not null`）      |
| `And` / `Or`     | 逻辑与/或（支持多操作数）                  |
| `Not`            | 逻辑非                                     |
| `Custom`         | 自定义布尔表达式                           |

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

### 运行时表达式构造与求值

```rust
use ospf_rust_math::symbol::expression::{
    ScalarExpression, BooleanExpression, ExpressionValue, MapEvaluationContext,
    evaluate_scalar_expression, MathFunctionEvaluator,
};

// 构造标量表达式：x * 2 + 3
let x = ScalarExpression::<ExpressionValue>::reference("x");
let expr = ScalarExpression::add_expr(
    ScalarExpression::multiply_expr(x.clone(), 2.0.into()),
    3.0.into(),
);

// 构造条件表达式：if x > 0 then x else 0
let condition = BooleanExpression::gt(x.clone(), 0.0.into());
let conditional = ScalarExpression::conditional(condition, x, 0.0.into());

// 求值上下文
let ctx = MapEvaluationContext::from_string_map([
    ("x", ExpressionValue::Number(5.0)),
]);

// 求值（使用 MathFunctionEvaluator 支持 math.* 函数）
let result = evaluate_scalar_expression(&expr, &ctx, &MathFunctionEvaluator);
assert_eq!(result, Some(ExpressionValue::Number(13.0)));
```

### 表达式解析（需要 "parser" feature）

```rust
use ospf_rust_math::symbol::expression::{
    parse_scalar_expression, evaluate_scalar_expression,
    MapEvaluationContext, MathFunctionEvaluator, ExpressionValue,
};

// 解析标量表达式字符串
let expr = parse_scalar_expression("if math.sqrt(x) > 2 then x else 0 fi").unwrap();

let ctx = MapEvaluationContext::from_string_map([
    ("x", ExpressionValue::Number(16.0)),
]);

let result = evaluate_scalar_expression(&expr, &ctx, &MathFunctionEvaluator);
assert_eq!(result, Some(ExpressionValue::Number(16.0)));
```

支持的解析语法：

- 算术：`+`、`-`、`*`、`/`、`%`、`^`、`**`
- 比较：`>`、`<`、`>=`、`<=`、`==`、`!=`、`<>`
- 逻辑：`&&`、`||`、`!`、`and`、`or`、`not`
- 条件：`? :` 三元、`if/then/else/fi`
- 函数：`name(args)`、`math.sqrt`、`math.pow`、`math.PI`、`math.E` 等
- 字面量：数字、字符串、`true`、`false`、`null`

### 布尔表达式规范化

```rust
use ospf_rust_math::symbol::expression::{
    BooleanExpression, ScalarExpression, ExpressionValue,
    flatten_boolean_expression, constant_fold_boolean_expression,
};

let x = ScalarExpression::<ExpressionValue>::reference("x");
// 构造嵌套的 And/Or：(x > 0 && x < 10) || (x > 100)
let inner = BooleanExpression::and(vec![
    BooleanExpression::gt(x.clone(), 0.0.into()),
    BooleanExpression::lt(x.clone(), 10.0.into()),
]);
let nested = BooleanExpression::or(vec![
    inner,
    BooleanExpression::gt(x, 100.0.into()),
]);

// 扁平化嵌套的 And/Or
let flat = flatten_boolean_expression(&nested);

// 常量折叠：消除常量 True/False 操作数
let folded = constant_fold_boolean_expression(&flat);
```

## 已实现功能

| 功能        | 状态 | 描述                                                               |
|-----------|----|------------------------------------------------------------------|
| 符号定义      | ✅  | `SymbolId`, `DynSymbol`, `Symbol`, `OwnedSymbol`                 |
| 线性单项式/多项式 | ✅  | `LinearMonomial<T>`, `Linear<T>`                                 |
| 二次单项式/多项式 | ✅  | `QuadraticMonomial<T>`, `Quadratic<T>`                           |
| 标准单项式/多项式 | ✅  | `CanonicalMonomial<T, E>`, `Canonical<T, E>`                     |
| 不等式       | ✅  | `LinearInequality`, `QuadraticInequality`, `CanonicalInequality` |
| 运行时表达式  | ✅  | `ScalarExpression`, `BooleanExpression`, `ExpressionValue`      |
| 表达式求值    | ✅  | `evaluate_scalar_expression`, `evaluate_boolean`, 可注入函数求值器 |
| 数学函数表    | ✅  | `MathFunctionEvaluator`（17 个 math.* 函数）                      |
| 布尔规范化    | ✅  | flatten, constant_fold, deduplicate, de_morgan, structural_key   |
| 条件表达式    | ✅  | `if/then/else/fi`、三元 `?:`、`Conditional` AST 变体              |
| 标量解析器    | ✅  | `parse_scalar_expression`（可选 feature）                        |
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
