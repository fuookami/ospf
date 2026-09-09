# 逻辑与

## 契约

`AndFunction<V>` 接受一个或多个线性多项式并暴露二值结果。当且仅当每个输入多项式都非零时结果为 `1`；至少一个输入为零时结果为 `0`。当前 API 对 `V : RealNumber<V> & NumberField<V>` 泛型化，并且是线性函数符号。

这不是只面向布尔变量的运算。输入可以是连续变量，也可以包含中间符号。

## 定义与真值表

对于输入多项式 $p_1,\ldots,p_n$，令 $a_i$ 表示非零指示量：

$$
a_i = \begin{cases}
1, & p_i \ne 0 \\
0, & p_i = 0
\end{cases},
\qquad
y = \begin{cases}
1, & \sum_{i=1}^{n} a_i = n \\
0, & \text{otherwise}
\end{cases}
$$

两个输入时的真值表如下：

| $p_1$ nonzero | $p_2$ nonzero | $y$ |
| --- | --- | --- |
| no | no | 0 |
| no | yes | 0 |
| yes | no | 0 |
| yes | yes | 1 |

构造器要求至少有一个输入多项式。

## 边界、tolerance 与 Undefined

`evaluate()` 将每个求值结果与精确的零比较。缺少某个多项式输入时返回 `null`，不会返回独立的 `Undefined` 值。

求解器注册使用共享的非零指示构造。给定 tolerance $t$ 时，指示量 `0` 表示零带 $\lvert p_i\rvert\le t$；给定严格边界 $g$ 时，指示量 `1` 表示 $p_i\ge g$ 或 $p_i\le-g$。位于间隔 $t<\lvert p_i\rvert<g$ 内的值没有可用分支，可能使模型不可行。

当前源码中的常量是 `NONZERO_TOLERANCE = 1e-10` 和 `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`，不是 `1e-6` 或 `0.5`。省略 `bigM` 时，先根据每个多项式的有限范围推导；没有可用范围时，范围工具回退到 `BIG_M_DEFAULT = 1e6`。

## 当前 API

### Kotlin

主构造器为：

```kotlin
AndFunction(
    polynomials: List<LinearPolynomial<V>>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "and",
    displayName: String? = null
)
```

伴生 `invoke` 接受 `polynomials`、`converter`、`bigM`、`name` 和 `displayName`。另外的 `fromLinearPolynomials` 工厂接受 `List<ToLinearPolynomial<V>>`，返回 `LinearFunctionSymbolAdapter<V>`。

### Rust

源码：[`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)

Rust 接收平展后的输入，并提供 `AndFunction::new(id, name, polynomials)`、`AndFunction::named(name, polynomials)` 和 `AndFunction::auto(polynomials)`。可通过 `result_variable()`、`indicator_variables()` 和 `side_variables()` 获取公开变量。Rust 没有公开的 tolerance 参数；求值器使用 epsilon 级别的非零判定，机理 Big-M 则从 token 范围推导，或回退到核心默认值。

```rust
AndFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self
AndFunction::named(name: impl AsRef<str>, polynomials: Vec<Linear<V>>) -> Self
AndFunction::auto(polynomials: Vec<Linear<V>>) -> Self
```

## 辅助变量与注册模型

函数名称为 `name` 时，当前实现创建：

- `name_and`：二值结果；
- `name_and_nz{i}`：每个输入对应一个非零指示量；
- `name_and_side{i}`：每个非零指示量对应一个符号侧辅助量。

`helperVariables` 包含结果、全部非零指示量和全部侧辅助量。注册先通过 `registerAuxiliaryTokens` 添加这些变量；`registerConstraints` 为每个输入添加共享的四条非零检测不等式，然后添加：

$$
\sum_i a_i \ge n y,
\qquad
y \le a_i\quad(1\le i\le n).
$$

公开的 `resultPolynomial` 是 `name_and` 的单位系数多项式。实现位于 `And.kt`，并向 `AbstractLinearMechanismModel` 注册。

## `evaluate()` 与求解器模型的差异

直接求值使用精确的 `v == 0`/`v != 0` 语义。求解器模型有意把零带与严格非零分支分开，因此数值上非零但位于 tolerance 与 strict boundary 之间的值会被 `evaluate()` 接受，却没有求解器分支。应根据模型的数值格点一致设置 `tolerance` 和 `strictBoundary`。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.AndFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val y = RealVar("y")
    val xPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64.zero
    )
    val yPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, y)),
        constant = Flt64.zero
    )
    val function = AndFunction(
        polynomials = listOf(xPoly, yPoly),
        converter = IntoValue.Identity,
        name = "and"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64(2.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.zero)) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::AndFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = AndFunction::named(
    "and",
    vec![Linear::new(vec![], 1.0), Linear::new(vec![], 2.0)],
);
let value = <AndFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(1.0));
```

:::

## 源码与 core 测试

- [Implementation: `And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `AndTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AndTest.kt)
- [Rust implementation: `and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)
- [Rust core coverage: `gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)

## 相关页面

- [逻辑或](/guide/linear-functional/or)
- [逻辑非](/guide/linear-functional/not)
- [恰好一个结果（`XorFunction`）](/guide/linear-functional/xor)
- [二值化](/guide/linear-functional/bin)
