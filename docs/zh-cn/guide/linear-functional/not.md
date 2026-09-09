# 逻辑非

## 契约

`NotFunction<V>` 接受一个线性多项式并暴露二值结果。多项式为零时结果恰为 `1`，非零时结果为 `0`。API 对 `V : RealNumber<V> & NumberField<V>` 泛型化。

该运算是当前非零指示量的反相，不是假定输入已有二值变量的布尔取反。

## 定义与真值表

对于线性多项式 (p)，令 (a) 为其非零指示量：

$$
a = \begin{cases}
1, & p \ne 0 \\
0, & p = 0
\end{cases},
\qquad
y = 1-a = \begin{cases}
1, & p = 0 \\
0, & p \ne 0
\end{cases}
$$

| (p) | (y=\operatorname{Not}(p)) |
| --- | --- |
| zero | 1 |
| nonzero | 0 |

## 边界、tolerance 与 Undefined

`evaluate()` 将求值结果与精确的零比较，缺少输入时返回 `null`。它没有 `Undefined` 返回分支。

求解器的非零指示量使用 tolerance (t) 表示零带，使用严格边界 (g) 表示非零分支：`indicatorVar = 0` 表示 $\lvert p\rvert\le t$，`indicatorVar = 1` 要求 $p\ge g$ 或 $p\le-g$。区间 (t<\lvert p\rvert<g) 不可分类，可能使模型不可行。结果通过 (y+a=1) 连接。

当前默认值为 `NONZERO_TOLERANCE = 1e-10` 和 `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`。省略 `bigM` 时根据多项式有限范围推导，没有范围时回退到 `BIG_M_DEFAULT = 1e6`。

## 当前 API

### Kotlin

```kotlin
NotFunction(
    polynomial: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "not",
    displayName: String? = null
)
```

伴生 `invoke` 接受 `polynomial`、`converter`、`bigM`、`name` 和 `displayName`；需要显式设置 `tolerance` 或 `strictBoundary` 时使用主构造器。

### Rust

Rust 在与 `OrFunction`、`XorFunction` 相同的模块中暴露 [`NotFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)：

```rust
NotFunction::new(id: u64, name: &str, polynomial: Linear<V>) -> NotFunction<V>
```

结果、非零指示量和 side 辅助量分别通过 `result_variable()`、`indicator_variable()` 和 `side_variable()` 取得。Rust 使用共享的非零指示默认值，没有 Kotlin 的逐实例 `tolerance` 或 `strictBoundary` 参数；`evaluate` 仍把精确零作为 NOT 的真值。

## 辅助变量与注册模型

对于 `name`，实现创建：

- `name_not_nz`：非零指示量 (a)；
- `name_not_side`：非零检测使用的符号侧辅助量；
- `name_not`：二值结果 (y)。

三个变量都在 `helperVariables` 中。`registerAuxiliaryTokens` 添加它们；`registerConstraints` 添加共享的四条非零指示不等式以及：

$$
y+a=1.
$$

公开的 `resultPolynomial` 是 `name_not` 的单位系数多项式；约束注册到 `AbstractLinearMechanismModel`。

## `evaluate()` 与求解器模型的差异

求值器把所有精确非零值视为 NOT 的假分支，即使其绝对值小于 `strictBoundary`。求解器有零带、严格非零带和不可分类间隔。不要在不调整边界或模型数值格点的情况下把间隔内的值用于求解器。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.NotFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val xPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64.zero
    )
    val function = NotFunction(
        polynomial = xPoly,
        converter = IntoValue.Identity,
        name = "not"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero)) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(3.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::NotFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let not = NotFunction::new(1, "not", input);
let _result = not.result_variable();
```

:::

源码与 core 测试：

- [Implementation: `And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `NotTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/NotTest.kt)

Rust 源码与回归覆盖：[`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## 相关页面

- [逻辑与](/guide/linear-functional/and)
- [逻辑或](/guide/linear-functional/or)
- [二值化](/guide/linear-functional/bin)
- [恰好一个结果（`XorFunction`）](/guide/linear-functional/xor)
