# 取模

`ModFunction` 表示线性多项式除以正常数后的非负余数：

$$
y = p\bmod d.
$$

## 契约

- 输入：`x: LinearPolynomial<V>` 与常数除数 `d: V`。
- 构造要求 `d > 0`；零和负除数会抛出参数异常。
- 商辅助变量是 `IntVar`；余数和结果辅助变量是 `URealVar`。
- `evaluate` 在输入无法求值时返回 `null`；否则计算基于 floor 的 `x` 余数。
- `bigM` 仍是构造参数以保持兼容，但当前取模约束不使用它。

## 数学定义

实现使用

$$
q=\left\lfloor\frac{p}{d}\right\rfloor,\qquad y=p-dq,
$$

并满足

$$
0\le y<d.
$$

注册的上界为 `y <= d - NONZERO_TOLERANCE`（转换为 `V`），因此 solver 用一个小数值余量编码严格的余数上边界。

## 适用域与边界

`d` 必须严格为正；此 API 没有带符号除数变体。对于负 `p`，floor 定义仍给出非负余数（例如 $-0.2\bmod 0.7=0.5$）。solver 使用 `IntVar q`、`URealVar r` 和 `URealVar result`；余数严格上界通过库容差近似实现。输入可以是任意有限且可求值的线性多项式。

## 当前 API

### Kotlin

源码：[`Mod.kt`（构造、校验、求值与约束）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Mod.kt#L39-L118)

```kotlin
ModFunction(
    x: LinearPolynomial<V>,
    d: V,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "mod",
    displayName: String? = null
)
```

### Rust

Rust 暴露 [`ModFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/mod_function.rs)：

```rust
ModFunction::new(id: u64, name: &str, input: Linear<V>, divisor: V) -> ModFunction<V>
```

Rust 符号创建连续余数（`result_variable()`）和整数商（`quotient_variable()`）。机制注入时拒绝非有限或零除数；但与 Kotlin 不同，它没有要求除数为正，负除数会使用 Rust 实现中的带符号边界。Rust 没有 `bigM` 参数。

## 辅助变量与注册模型

`helperVariables` 注册 `qVar`、`rVar` 和 `resultVar`。约束注册添加 `r = x - d*q`、`r <= d - epsilon` 与 `result = r`；非负性来自 `URealVar`。当前编码没有 Big-M 约束。

## `evaluate` 与 solver 的差异

`evaluate` 通过 `IntoValue` 转换输入和除数，直接应用 `floor` 并计算余数。solver 使用整数商和带容差调整的严格上界。在普通有限值上两者一致；靠近余数上边界的值可能受到 `NONZERO_TOLERANCE` 影响。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ModFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val mod = ModFunction(
    x = xPoly,
    d = Flt64.two,
    converter = IntoValue.Identity,
    name = "mod"
)
val value = mod.evaluate(mapOf<Symbol, Flt64>(x to Flt64(5.0)))
check(value != null && (value eq Flt64.one))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::ModFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let modulo = ModFunction::new(1, "mod", input, 2.0_f64);
assert_eq!(modulo.divisor(), &2.0);
let _remainder = modulo.result_variable();
```

:::

完整示例：[`ModTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ModTest.kt)

Core 验证：[`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)

Rust 源码与 parity 覆盖：[`mod_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/mod_function.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## 相关页面

- [`floor`](./floor)：定义中使用的商运算。
- [`ceiling`](./ceiling) 与 [`rounding`](./rounding)：其他离散变换。
- [`ulp`](./ulp)：需要连续近似时的分段线性插值。
