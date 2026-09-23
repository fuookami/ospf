# 不等式指示函数

`InequalityFunction` 将线性多项式与标量的比较转换为二值结果。

## 契约

- 输入：`lhs: LinearPolynomial<V>`、标量 `rhs: V` 以及 `Comparison` 符号。
- 直接 `evaluate` 支持 `LE`、`LT`、`GE`、`GT`、`EQ` 和 `NE`。
- 输出：包含二值标志的线性多项式 `result`。
- solver 注册支持 `LE/LT/GE/GT/EQ/NE`；`NE` 使用与 `EQ` 相同的零带 side 编码，结果标志充当非零指示。
- 泛型值使用 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

## 定义与数学模型

令 $d=lhs-rhs$，且 $y\in\{0,1\}$。契约是：

$$
y=\mathbf{1}[lhs\ \mathrel{\text{sign}}\ rhs].
$$

`EQ` 与 `NE` 的 solver 编码使用带 tolerance 的零带和一个方向二值变量。其他支持的符号使用两条 Big-M 约束将标志连接到满足与违反分支。直接求值比较与 solver 的容差编码彼此独立。

## 求解器数学模型

令 $d=lhs-rhs$，先把关系规范化为真分支 $q\ge T$、假分支 $q\le F$：

| 关系 | $q$ | $T$ | $F$ |
| --- | ---: | ---: | ---: |
| `GT` | $d$ | $g$ | $0$ |
| `GE` | $d$ | $0$ | $-g$ |
| `LT` | $-d$ | $g$ | $0$ |
| `LE` | $-d$ | $0$ | $-g$ |

给定结果变量 $y\in\{0,1\}$ 与由 lhs-rhs 有限范围推导的 Big-M $M$，实现实际向求解器注册两条 Big-M 约束：

$$
\begin{aligned}
q-M_1y&\le F,\\
q-M_2y&\ge T-M_2,
\end{aligned}
$$

其中一个乘数是加上间隔的 $M+g$，使得每个指示值恰有一条约束起约束作用（`GT`/`LT` 取 $M_1=M,\ M_2=M+g$，`LE`/`GE` 取 $M_1=M+g,\ M_2=M$）。因此 $y=1\Rightarrow q\ge T$，$y=0\Rightarrow q\le F$；开区间 $(F,T)$ 被刻意留为不可行间隔。对于 `EQ` 与 `NE`，两种实现都会创建方向二值变量，并使用共享的四约束零值/非零值 Big-M 编码：`EQ` 的标志等于非零标志的补，而 `NE` 的标志就是非零标志本身。

## 当前 API

### Kotlin

源码：[`Inequality.kt`（`InequalityFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Inequality.kt#L44-L229)

```kotlin
InequalityFunction(
    lhs: LinearPolynomial<V>,
    rhs: V,
    sign: Comparison,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "ineq",
    displayName: String? = null
)
```

公开工厂暴露 `bigM`，但不暴露构造器中的可选 `tolerance` 和 `strictBoundary`；需要定制这些参数时应直接调用类构造器。

### Rust

Rust 提供直接的平展表达式对应物 [`InequalityFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs)。它接收一个 `Linear<V>`、标量右侧值、显式的 [`InequalityKind`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs) 和 Big-M：

```rust
InequalityFunction::new(
    id: u64,
    name: &str,
    left: Linear<V>,
    right: V,
    kind: InequalityKind,
    big_m: V,
) -> InequalityFunction<V>

InequalityFunction::less_equal(
    id: u64,
    name: &str,
    left: Linear<V>,
    right: V,
    big_m: V,
) -> InequalityFunction<V>
```

`InequalityKind` 包含 `LessEqual`、`GreaterEqual`、`Less`、`Greater`、`Equal` 和 `NotEqual`；`result_variable()` 返回二值指标，EQ/NE 还会创建 side 变量。Rust 的机理编码也支持 `NotEqual`，不只支持直接求值，与上面记录的 Kotlin 编码一致。Rust 构造器没有 Kotlin 的 converter/tolerance 参数；其机理使用固定的指标容差以及传入或推导的 Big-M。

## evaluate 与 solver 的差异

直接求值按关系对应的间隔分类（`LE`/`GE` 用 `tolerance`，`LT`/`GT` 用 `strictBoundary`，`EQ`/`NE` 用距离带），间隔内返回 `null`。solver 注册使用相同阈值的 Big-M 约束；间隔在该处不可行而非未定义。`NE` 完全受支持：直接求值可用，`registerConstraints` 会写入四约束零值/非零值编码。

## 边界、tolerance 与 Undefined

线性多项式符号缺失时 `evaluate` 返回 `null`，取值落在关系间隔内时同样返回 `null`。Big-M 必须为正、有限、可表示且足以覆盖 lhs-rhs 范围。`EQ` 与 `NE` 还需要有限且有效的严格边界以构造 side 编码。非法输入（非正的 Big-M、无效的等式带）通过失败的注册 Result 暴露。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.InequalityFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val inequality = InequalityFunction(
    lhs = xPoly,
    rhs = Flt64.one,
    sign = Comparison.LE,
    converter = IntoValue.Identity,
    bigM = Flt64(10.0),
    name = "ineq"
)
val value = inequality.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{InequalityFunction, InequalityKind};

let left = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let inequality = InequalityFunction::less_equal(
    1,
    "x_le_1",
    left,
    1.0_f64,
    10.0_f64,
);
assert_eq!(inequality.inequality_kind(), InequalityKind::LessEqual);
let _result = inequality.result_variable();
```

:::

- core 专用测试：[`InequalityFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/InequalityFunctionDedicatedTest.kt)
- 示例目录（当前没有专门的不等式文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust 源码与 parity 覆盖：[`inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## 相关页面

- [满足数量](./satisfied-amount)
- [满足数量不等式](./satisfied-amount-inequality)
- [蕴含](./imply)
