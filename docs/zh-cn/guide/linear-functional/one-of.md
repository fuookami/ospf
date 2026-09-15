# 选一约束

## 契约

`OneOfFunction<V>` 接受一个或多个线性多项式。直接求值器在恰好一个输入非零时返回 `1`，否则返回 `0`。注册到求解器后，它比自由二值指示量更强：它强制恰好一个输入非零，并把结果固定为 `1`。

该函数不选择任何分支值，也不实现旧版的 branch/payload API；它只统计非零输入多项式。

## 定义与真值表

对于输入 $p_1,\ldots,p_n$，令：

$$
a_i = \begin{cases}
1, & p_i \ne 0 \\
0, & p_i = 0
\end{cases},
\qquad
y = \begin{cases}
1, & \sum_{i=1}^{n} a_i = 1 \\
0, & \sum_{i=1}^{n} a_i \ne 1
\end{cases}
$$

注册后的模型还要求：

$$
\sum_{i=1}^{n} a_i = 1,
\qquad
y=1.
$$

两个输入时，求值器真值表为：

| $p_1$ nonzero | $p_2$ nonzero | $y$ |
| --- | --- | --- |
| no | no | 0 |
| no | yes | 1 |
| yes | no | 1 |
| yes | yes | 0 |

构造器要求至少有一个输入多项式。

## 边界、tolerance 与 Undefined

`evaluate()` 使用精确的 `v != 0`；缺少输入值时返回 `null`，不会返回 `Undefined`。

每个求解器非零指示量使用 tolerance $t$ 作为零带 $\lvert p_i\rvert\le t$，使用严格边界 $g$ 作为非零带 $p_i\ge g$ 或 $p_i\le-g$。间隔 $t<\lvert p_i\rvert<g$ 没有有效指示量赋值，可能使模型不可行。当前默认值为 `NONZERO_TOLERANCE = 1e-10` 和 `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`；省略 `bigM` 时从有限输入边界推导，否则回退到 `BIG_M_DEFAULT = 1e6`。

## 当前 API

### Kotlin

```kotlin
OneOfFunction(
    polynomials: List<LinearPolynomial<V>>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    converter: IntoValue<V>,
    name: String = "oneof",
    displayName: String? = null
)
```

伴生 `invoke` 接受 `polynomials`、`bigM`、`converter`、`name` 和 `displayName`；需要设置 `tolerance` 或 `strictBoundary` 时使用主构造器。

### Rust

Rust 的 [`OneOfFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/one_of.rs) 与 Kotlin 的“恰好一个输入非零”测试契约不同：

```rust
OneOfFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> OneOfFunction<V>
```

它创建 `selection_variables()`，并在连续的 `result_variable()` 中返回被选择候选的加权和。选择器是模型变量；Rust 不检查每个候选多项式是否为零。因此它不是 Kotlin 真值表的一一对应 API；如果语义是统计非零/二值指示量，应使用 `XorFunction` 或 `SatisfiedAmountFunction`。

## 求解器数学模型

对于 `name`，实现创建 `name_oneof` 作为结果、每个输入一个 `name_oneof_nz{i}` 非零指示量，以及每个输入一个 `name_oneof_side{i}` 符号侧辅助量。它们都在 `helperVariables` 中；`resultPolynomial` 是 `name_oneof` 的单位系数多项式。

对每个输入，共享的四约束 Big-M 模型表示

$$
a_i=0\Rightarrow -t\le p_i\le t,
$$

$$
(a_i,s_i)=(1,1)\Rightarrow p_i\ge g,
\qquad
(a_i,s_i)=(1,0)\Rightarrow p_i\le-g.
$$

实现展开这些蕴含后，再实际传入

$$
\sum_i a_i=1,\qquad y=1.
$$

Rust 的同名选择函数则传入 $\sum_i z_i=1$ 和 $y=\sum_i z_i p_i$，不会创建非零标志。

## `evaluate()` 与求解器模型的差异

注册前，`evaluate()` 是一个完整的恰好一个指示器（输入缺失时为 `null`）。注册后，求解器非零指示量为零个或多个的赋值不是单纯产生结果 `0`，而是不可行。这是当前实现有意保留的区别，凡把该函数作为模型约束使用都应说明这一点。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.OneOfFunction
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
    val function = OneOfFunction(
        polynomials = listOf(xPoly, yPoly),
        converter = IntoValue.Identity,
        name = "oneof"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.zero)) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64(2.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::OneOfFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let one_of = OneOfFunction::new(1, "one_of", vec![x, y]);
assert_eq!(one_of.selection_variables().len(), 2);
let _result = one_of.result_variable();
```

:::

Source and core tests:

- [Implementation: `OneOf.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/OneOf.kt)
- [Core conditional registration test: `FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- [Core conditional regression test: `ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [Complete example: `OneOfTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/OneOfTest.kt)

Rust 源码：[`one_of.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/one_of.rs)。

## 相关页面

- [逻辑与](/guide/linear-functional/and)
- [逻辑或](/guide/linear-functional/or)
- [恰好一个结果 (`XorFunction`)](/guide/linear-functional/xor)
- [条件 IF](/guide/linear-functional/if)
