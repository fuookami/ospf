# 二次最小值

`QuadraticMinFunction` 计算一组二次多项式的最小值，并提供精确选择模式或下包络松弛。

## 契约

- 输入：`polynomials: List<QuadraticPolynomial<V>>`。
- 输出/辅助变量：名称在 `name` 后追加 `_min` 的实数 `resultVar`。
- 直接求值返回最小值；符号缺失或候选列表为空时返回 `null`。
- `exact = true` 为每个候选创建二值选择变量并尝试强制精确最小值；`exact = false` 只注册 $y\le p_i$ 约束。
- 泛型值要求 `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

> [!WARNING]
> `exact = false` 是下包络松弛。没有目标或其他约束推动 y 增大时，它不一定等于数学最小值。

## 定义与数学模型

对候选 $p_i$ 和结果 y，两种模式都添加

$$
y\le p_i\qquad(i=0,\ldots,n-1).
$$

精确模式另外创建 $u_i\in\{0,1\}$：

$$
y\ge p_i-M_i(1-u_i),\qquad \sum_i u_i=1.
$$

直接求值无论 `exact` 如何设置都计算 $\min_i p_i$。

## 求解器数学模型

### Kotlin

令候选二次表达式为 $p_i$，结果为有符号连续变量 $y\in\mathbb R$。所有模式都提交：

$$
y-p_i\le0,
\qquad \forall i.
$$

`exact = false` 时只有这些上界约束，必须由目标函数或其他约束把 $y$ 推到真正的最小值。`exact = true` 时还创建 $u_i\in\{0,1\}$ 并提交：

$$
y-p_i-M_i u_i\ge-M_i,
\qquad \forall i,
$$

$$
\sum_i u_i=1.
$$

也就是 $y\ge p_i-M_i(1-u_i)$。$M_i$ 优先使用显式值，否则从候选范围推导，再退回候选的默认 Big-M。

### Rust

Rust 先为每个二次候选创建有符号连续桥接变量 $b_i$ 并提交 $b_i=p_i(x)$，随后对 $b_i$ 使用相同的 Min 模型：

$$
y\le b_i,
$$

精确模式再提交：

$$
y\ge b_i-M_i(1-u_i),
\qquad
u_i\in\{0,1\},
\qquad
\sum_i u_i=1.
$$

因此两种语言的最小值约束一致，主要差别是 Rust 显式桥接每个二次候选。

## 当前 API

### Kotlin

源码：[`QuadraticMin.kt`（`QuadraticMinFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMin.kt#L40-L390)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMinFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val first = QuadraticPolynomial(
    listOf(QuadraticMonomial.quadratic(Flt64.one, x, y)), Flt64.one
)
val second = QuadraticPolynomial(
    listOf(QuadraticMonomial.linear(Flt64.one, x)), Flt64.two
)
val minimum = QuadraticMinFunction(
    polynomials = listOf(first, second),
    exact = true,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_min"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = minimum.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

### Rust

Rust 提供 [`QuadraticMinFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_min.rs)：

```rust
QuadraticMinFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
    exact: bool,
) -> QuadraticMinFunction<V>
```

`result_variable` 返回内部的 `name + "_min"` 变量，`with_declared_dependencies` 保留显式依赖 ID。每个输入都会由 `QuadraticLinearFunction` 桥接；`exact = true` 创建内部二值选择变量，`exact = false` 只保留下包络上界不等式。`calculate_value` 始终计算数学最小值。有 token 边界时，`mechanism_constraints_with_tokens` 从原始二次候选推导 Big-M；否则使用通用回退策略。Rust 该类型的构造器没有 `bigM` 参数。

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMinFunction;

let first = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)],
    1.0,
);
let second = Quadratic::new(
    vec![QuadraticMonomial::new_linear(1.0, 0)],
    2.0,
);
let minimum = QuadraticMinFunction::new(15, "quadratic_min", vec![first, second], true);
assert!(minimum.result_variable().name().contains("quadratic_min_min"));
```

## evaluate 与 solver 的差异

直接求值始终得到精确最小值。精确 solver 模式需要所有候选有效的 Big-M 范围；松弛模式只保证 $y$ 对每个候选是下包络结果。结果桥接变量是有符号变量，可以表示负的候选最小值。

## 边界、tolerance 与 Undefined

输入列表应非空；空列表会使直接最小值为 null，也没有有意义的 solver 模型。缺少值时返回 `null`。该函数没有 tolerance 或三值 Undefined 状态。Big-M 无效或不足时，精确注册可能失败或被削弱。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMinFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val first = QuadraticPolynomial(
    listOf(QuadraticMonomial.quadratic(Flt64.one, x, y)), Flt64.one
)
val second = QuadraticPolynomial(
    listOf(QuadraticMonomial.linear(Flt64.one, x)), Flt64.two
)
val minimum = QuadraticMinFunction(
    polynomials = listOf(first, second),
    exact = true,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_min"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = minimum.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMinFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let ty = Token::from_generic(y, 1);
ty.set_result(5.0);
tokens.add_token(ty);
let minimum = QuadraticMinFunction::new(
    16,
    "quadratic_min",
    vec![
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 1.0),
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 2.0),
    ],
    true,
);
assert_eq!(minimum.calculate_value(&tokens, false), Some(4.0));
```

:::

- Core 求值：[`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core 注册：[`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- 示例目录（当前没有专门的二次最小值文件）：[quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- Rust 实现与针对性测试：[`quadratic_min.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_min.rs) 与 [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## 相关页面

- [最大值](../linear-functional/max)
- [最小值](../linear-functional/min)
- [二次线性](./quadratic-linear)
