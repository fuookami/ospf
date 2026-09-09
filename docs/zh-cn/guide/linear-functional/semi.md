# 半连续标记

## 当前 API

### Kotlin

`SemiFunction<V>` 是携带半连续变量激活区间的标记：

$$
y = 0 \quad\text{or}\quad lb \le y \le ub.
$$

它**不是**正部函数 `max(0,x)`，也没有输入表达式。实现位于 [`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt#L37-L107)。构造函数为：

```kotlin
SemiFunction(
    lb: V? = null,
    ub: V? = null,
    converter: IntoValue<V>,
    name: String = "semi",
    displayName: String? = null
)

SemiFunction.from(
    variable: AbstractVariableItem<*, *>,
    lb: V? = null,
    ub: V? = null,
    converter: IntoValue<V>,
    name: String = "semi",
    displayName: String? = null
)
```

默认值是 `lb = 0`、`ub = 1e6`（`Semi.kt:37-50`），并要求 `lb <= ub`。`from` 会从 `variable.range.valueRange` 推断未显式提供的边界（`Semi.kt:89-106`）。

### Rust

Rust 提供的是可执行的 [`SemiFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs)，而不是空操作标记：

```rust
SemiFunction::new(
    id: u64,
    name: &str,
    lower: V,
    upper: V,
) -> SemiFunction<V>

SemiFunction::try_from_variable(
    id: u64,
    name: &str,
    variable: &ContinuousVariableItem,
    lower: Option<V>,
    upper: Option<V>,
) -> Result<SemiFunction<V>>
```

该符号创建连续 `result_variable()` 和二值 `indicator_variable()`，并注册 `result <= upper * indicator` 与 `result >= lower * indicator`。`try_from_variable`（别名 `from_variable`）可从 `ContinuousVariableItem` 推导缺失的有限边界。这与 Kotlin 不同：Kotlin 的 `SemiFunction` 没有辅助变量，也不会注册域约束。

## 运行时与注册语义

该标记不创建辅助变量（`helperVariables` 为空），`evaluate` 始终返回 `null`，`registerAuxiliaryTokens` 与 `registerConstraints` 都只返回成功而不添加任何内容（`Semi.kt:53-65`）。因此它不计算 `max(0,x)`，不绑定线性表达式，也不会自行强制半连续域。只有理解该标记的求解器/后端集成才会消费它；仅构造或保留 `SemiFunction` 不会改变模型。

## 参考

- 实现：[`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt)
- 完整样例：[`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SemiTest.kt)

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SemiFunction

val semi = SemiFunction(
    lb = Flt64.two,
    ub = Flt64.five,
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.lb == Flt64.two)
check(semi.ub == Flt64.five)
check(semi.helperVariables.isEmpty())
check(semi.evaluate(emptyMap()) == null)
```

```rust [Rust]
use ospf_rust_core::symbol::function::SemiFunction;

let semi = SemiFunction::new(1, "semi", 2.0_f64, 5.0_f64);
assert_eq!(semi.lower_bound(), &2.0);
assert_eq!(semi.upper_bound(), &5.0);
let _result = semi.result_variable();
let _indicator = semi.indicator_variable();
```

:::

当前 smoke test 检查边界、空辅助变量列表和未解析时的求值结果（[`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SemiTest.kt#L16-L24)）：

若要建模 `max(0,x)`，应使用明确的正部公式；不要向 `SemiFunction` 传入表达式，因为当前 API 没有该参数。

Rust 源码：[`semi.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs)。
