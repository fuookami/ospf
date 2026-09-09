# 二值化

## 契约

`BinaryzationFunction<V>` 将一个线性多项式 $p$ 映射为二值结果。当前契约是正值二值化：

$$
y = \operatorname{Bin}(p) = \begin{cases}
1, & p > 0 \\
0, & p \le 0
\end{cases}
$$

输入不必是二值变量，也可以是任意带有 `V : RealNumber<V> & NumberField<V>` 的 `LinearPolynomial<V>`。

## 定义与真值表

| $p$ | $y$ |
| --- | --- |
| $p>0$ | 1 |
| $p=0$ | 0 |
| $p<0$ | 0 |

结果是二值变量，不是 $p$ 的数值副本。

## 边界、tolerance 与 Undefined

`evaluate()` 使用严格比较 `p > 0`；缺少多项式求值结果时返回 `null`。

求解器注册固定使用 `NONZERO_TOLERANCE = 1e-10` 作为 $\varepsilon$。如果 $a$ 是结果变量，$M$ 是选定的 Big-M 值，核心指示约束为：

$$
p - M a \le 0,
\qquad
p - M' a \ge \varepsilon-M'.
$$

因此 $a=0$ 要求 $p\le0$，而 $a=1$ 要求 $p\ge\varepsilon$。开区间 $0<p<\varepsilon$ 是求解器间隔：`evaluate()` 返回 `1`，但线性化模型没有有效分支。`BinaryzationFunction` 没有公开的 `tolerance` 参数。

省略 `bigM` 时，实现根据多项式有限范围推导；必要时回退到 `BIG_M_DEFAULT = 1e6`。

## 当前 API

### Kotlin

```kotlin
BinaryzationFunction(
    polynomial: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String = "bin",
    displayName: String? = null
)
```

伴生 `invoke` 使用相同参数。`converter` 是必需参数；旧的标量构造器不属于当前 API。

### Rust

源码：[`binaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/binaryzation.rs)

Rust 提供 `BinaryzationFunction::new(id, name, input, threshold, big_m, method)` 及命名/自动工厂。便捷构造器包括 `with_big_m`/`named_big_m`/`auto_big_m`（严格使用 `input > threshold`，阈值为零时与 Kotlin 的正值测试一致）和 `with_threshold`/`named_threshold`/`auto_threshold`（使用 `input >= threshold`）。`BinaryzationMethod` 可取 `BigM`、`Threshold`、`Indicator` 或 `SOS1`；后两者在当前机理层使用 Big-M 等价编码。

```rust
BinaryzationFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    threshold: V,
    big_m: V,
    method: BinaryzationMethod,
) -> Self
BinaryzationFunction::named_big_m(name: impl AsRef<str>, input: Linear<V>, big_m: V) -> Self
BinaryzationFunction::named_threshold(name: impl AsRef<str>, input: Linear<V>, threshold: V) -> Self
```

## 辅助变量与注册模型

对于 `name`，唯一辅助变量是同时作为 `resultVar` 的 `name_bin`。`helperVariables` 包含这个二值变量，`resultPolynomial` 是它的单位系数多项式。

`registerAuxiliaryTokens` 添加结果变量。`registerConstraints` 使用选定 Big-M 和固定 tolerance 调用共享的正值指示构造器，然后把线性不等式添加到 `AbstractLinearMechanismModel`。

## `evaluate()` 与求解器模型的差异

求值器只区分 $p > 0$ 和 $p \le 0$，不表示求解器使用的数值间隔。如果模型可能产生 $(0, \text{NONZERO\_TOLERANCE})$ 内的值，应先决定是否改变输入格点，或增加单独记录的策略，再依赖求值器与求解器一致。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.BinaryzationFunction
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
    val function = BinaryzationFunction(
        polynomial = xPoly,
        converter = IntoValue.Identity,
        name = "bin"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(2.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero)) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-1.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::BinaryzationFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let input = Linear::new(vec![], 2.0);
let function = BinaryzationFunction::named_big_m("bin", input, 10.0);
let value = <BinaryzationFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(1.0));
```

:::

## 源码与 core 测试

- [Implementation: `Binaryzation.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Binaryzation.kt)
- [Core regression test: `FunctionSymbolRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRegressionTest.kt)
- [Core result-polynomial contract test: `LegacyResultPolynomialContractTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/LegacyResultPolynomialContractTest.kt)
- [Complete example: `BinTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BinTest.kt)
- [Rust implementation: `binaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/binaryzation.rs)
- [Rust core coverage: `p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

## 相关页面

- [逻辑与](/guide/linear-functional/and)
- [逻辑或](/guide/linear-functional/or)
- [逻辑非](/guide/linear-functional/not)
- [平衡三值化](/guide/linear-functional/bter)
