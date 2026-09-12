# 逻辑或

## 契约

`OrFunction<V>` 接受一个或多个线性多项式并暴露二值结果。至少一个输入多项式非零时结果为 `1`，所有输入均为零时结果才为 `0`。API 对 `V : RealNumber<V> & NumberField<V>` 泛型化。

该运算检测非零值，不要求输入多项式本身是二值变量。

## 定义与真值表

对于输入多项式 $p_1,\ldots,p_n$，令 $a_i$ 表示非零指示量：

$$
a_i = \begin{cases}
1, & p_i \ne 0 \\
0, & p_i = 0
\end{cases},
\qquad
y = \begin{cases}
1, & \sum_{i=1}^{n} a_i \ge 1 \\
0, & \sum_{i=1}^{n} a_i = 0
\end{cases}
$$

两个输入时：

| $p_1$ 非零 | $p_2$ 非零 | $y$ |
| --- | --- | --- |
| no | no | 0 |
| no | yes | 1 |
| yes | no | 1 |
| yes | yes | 1 |

构造器要求至少有一个输入多项式。

## 边界、tolerance 与 Undefined

`evaluate()` 对第一个非零输入使用精确的 `v != 0`，无法求得某个输入时返回 `null`，不会暴露 `Undefined` 值。

求解器的共享非零指示量使用两个数值带。指示量 `0` 表示 $\lvert p_i\rvert\le t$，其中 `tolerance` 为 (t)；指示量 `1` 要求 $p_i\ge g$ 或 $p_i\le-g$，其中 `strictBoundary` 为 (g)。开区间 $t<\lvert p_i\rvert<g$ 没有指示量赋值，可能使注册或求解不可行。

源码常量为 `NONZERO_TOLERANCE = 1e-10` 和 `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`。默认 `bigM` 按每个多项式的有限范围推导，否则回退到 `BIG_M_DEFAULT = 1e6`。

## 当前 API

### Kotlin

```kotlin
OrFunction(
    polynomials: List<LinearPolynomial<V>>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "or",
    displayName: String? = null
)
```

伴生 `invoke` 接受 `polynomials`、`converter`、`bigM`、`name` 和 `displayName`。与主构造器不同，该便捷重载不暴露 `tolerance` 或 `strictBoundary`。

### Rust

Rust 暴露 [`OrFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)：

```rust
OrFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> OrFunction<V>
```

同时提供 `named` 和 `auto`。生成的二值变量可通过 `result_variable()`、`indicator_variables()` 和 `side_variables()` 取得。Rust 构造器没有 Kotlin 风格的 converter、`bigM`、tolerance 或 strict-boundary 参数；它使用共享的非零指示策略，并在可能时从已注册边界推导 Big-M。

## 求解器数学模型

对于 `name`，实现创建 `name_or` 作为结果、每个输入一个 `name_or_nz{i}` 非零指示量，以及每个输入一个 `name_or_side{i}` 符号侧辅助量。它们全部由 `helperVariables` 返回。

对每个输入，四约束 Big-M 模型表示

$$
a_i=0\Rightarrow -t\le p_i\le t,
$$

$$
(a_i,s_i)=(1,1)\Rightarrow p_i\ge g,
\qquad
(a_i,s_i)=(1,0)\Rightarrow p_i\le-g.
$$

实现把这些蕴含展开成线性不等式后，再添加 OR 约束：

$$
\sum_i a_i \ge y,
\qquad
y \ge a_i\quad(1\le i\le n).
$$

公开的 `resultPolynomial` 是 `name_or` 的单位系数多项式。该符号向 `AbstractLinearMechanismModel` 注册。

Rust 同样先注册非零指标块，再注册相同的 OR 约束，但使用 Rust 自己的固定阈值与范围推断。

## `evaluate()` 与求解器模型的差异

求值器把所有精确非零值都视为真，包括小于 `strictBoundary` 的值。求解器编码有意不表示 tolerance 到 boundary 之间的间隔。稳健建模时，应选择与变量实际可取值相分离的 strict boundary。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.OrFunction
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
    val function = OrFunction(
        polynomials = listOf(xPoly, yPoly),
        converter = IntoValue.Identity,
        name = "or"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.zero)) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64(3.0))) == Flt64.one)
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::OrFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let or = OrFunction::new(1, "or", vec![x, y]);
assert_eq!(or.indicator_variables().len(), 2);
let _result = or.result_variable();
```

:::

源码与 core 测试：

- [Implementation: `And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `OrTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/OrTest.kt)

Rust 源码与 parity 覆盖：[`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## 相关页面

- [逻辑与](/guide/linear-functional/and)
- [逻辑非](/guide/linear-functional/not)
- [恰好一个结果（`XorFunction`）](/guide/linear-functional/xor)
- [选一约束](/guide/linear-functional/one-of)
