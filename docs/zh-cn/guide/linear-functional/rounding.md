# 舍入

`RoundingFunction` 使用数值实现的 `round` 操作将线性多项式映射为整数：

$$
y=\operatorname{round}(p).
$$

由于半整数行为并不相同，本页将直接求值契约与 solver 契约分开说明。

## 契约

- 输入：`x: LinearPolynomial<V>`。
- 输出：`IntVar`（`resultVar`），通过 `resultPolynomial` 暴露。
- 输入无法求值时，`evaluate` 返回 `null`；否则委托给 `converter.fromValue(x).round()`。
- `bigM` 控制小数指示变量门控，并规范为至少 1；它必须足以支持该门控，默认值为 1。

## 数学定义

solver 将输入分解为

$$
k=\lfloor p\rfloor,\qquad b=p-k,\qquad 0\le b<1,
$$

然后使用二进制 $r$：

$$
b\ge 0.5r,\qquad b\le 0.5-\varepsilon+Mr,
$$

并注册

$$
y=k+r.
$$

因此 solver 在小于 0.5 时取 `r = 0`，大于等于 0.5 时取 `r = 1`：半整数向正无穷取整。

## 适用域与边界

直接求值接受所有有限实数。solver 在整数边界附近使用 `NONZERO_TOLERANCE`，小数门控使用规范化后的 Big-M。关键语义边界是半整数：

| 路径 | 半整数规则 | 示例 |
| --- | --- | --- |
| solver 注册 | `b >= 0.5` 向上取整（`k + 1`） | `2.5 -> 3`, `-1.5 -> -1` |
| `Flt32` / `Flt64` 求值 | 委托 `kotlin.math.round`，半整数取偶 | `2.5 -> 2`, `-1.5 -> -2` |
| `FltX` 求值 | `BigDecimal` `HALF_UP` | `2.5 -> 3`, `-1.5 -> -2` |

没有处理这一差异时，不要使用半整数交叉核对 `evaluate` 与 solver 结果。

## 当前 API

### Kotlin

源码：[`Rounding.kt`（构造、求值与约束）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Rounding.kt#L37-L147)

`evaluate` 使用的数值行为实现于 [`Floating.kt`（`Flt64.round`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-math/src/main/fuookami/ospf/kotlin/math/algebra/number/Floating.kt#L1264-L1277) 与 [`FltX.round`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-math/src/main/fuookami/ospf/kotlin/math/algebra/number/Floating.kt#L1977-L1980)。

```kotlin
RoundingFunction(
    x: LinearPolynomial<V>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

### Rust

Rust 在 [`RoundingFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/rounding.rs) 中显式指定取整类型：

```rust
RoundingFunction::new(id: u64, name: &str, input: Linear<V>, kind: RoundingKind) -> RoundingFunction<V>
RoundingFunction::round(id: u64, name: &str, input: Linear<V>) -> RoundingFunction<V>
```

`RoundingKind` 包含 `Floor`、`Ceil`、`Round` 和 `Trunc`，同时提供 `named_round` 等快捷构造器。Rust 保留整数辅助变量，但通过 `result_variable()` 暴露连续结果；它没有 Kotlin 的 `bigM`/converter 参数。取整边界由 Rust 自己的 `ROUNDING_EPSILON` 规则实现，因此不要假定两种语言在半整数处的行为完全一致。

## 求解器数学模型

Kotlin 引入 $k\in\mathbb Z$、$r\in\{0,1\}$、$b\ge0$ 和整数结果 $y$，实际传入

$$
p-k\ge0,\qquad p-k\le1-\varepsilon,\qquad b-p+k=0,
$$

$$
b-0.5r\ge0,\qquad b-Mr\le0.5-\varepsilon,\qquad y-k-r=0.
$$

Rust 使用所选 `RoundingKind` 对应的约束组；`Round` 同样把连续结果连接到整数辅助变量，但使用 Rust 自己的固定边界 epsilon。

## `evaluate` 与 solver 的差异

`evaluate` 调用转换器的 `round`，不会创建辅助变量或使用 Big-M。solver 始终使用 floor 加 `b >= 0.5` 门控。两者在非半整数处一致，但在半整数处可能返回不同值，如上表所示。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.RoundingFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val round = RoundingFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "round"
)
val value = round.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.2)))
check(value != null && (value eq Flt64.one))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::RoundingFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let round = RoundingFunction::round(1, "round", input);
let _result = round.result_variable();
assert!(round.sign_variable().is_some());
```

:::

完整示例：[`RoundTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/RoundTest.kt)

Core 验证：[`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt) 与 [`FunctionSymbolRoundingGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRoundingGenericRegistrationTest.kt)

Rust 源码与 parity 覆盖：[`rounding.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/rounding.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## 相关页面

- [`floor`](./floor)：solver 的整数分解步骤。
- [`ceiling`](./ceiling)：没有除数的向上取整变换。
- [`mod`](./mod)：要求 `d > 0` 的 floor 余数。
