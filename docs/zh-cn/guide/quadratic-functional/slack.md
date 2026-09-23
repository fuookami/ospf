# 二次模型中的绝对松弛

`QuadraticSlackFunction` 精确表示

$$
s=|L(x)-R(x)|
$$

两种语言实现都对原始二次表达式求值，并注册精确的绝对值约束，因此正确性不依赖是否在目标中最小化该结果。Rust 仅为真正含二次项的输入创建线性桥接；纯线性输入保持为直接表达式。

## 求解器数学模型

令 $l=L(x)$、$r=R(x)$、$d=l-r$，$s\ge0$ 为结果，$z\in\{0,1\}$ 为方向选择变量，$M$ 为 $|d|$ 的有效上界。实际传给求解器的约束为

$$
\begin{aligned}
s-d&\ge0,\\
s+d&\ge0,\\
s-d+Mz&\le M,\\
s+d-Mz&\le0.
\end{aligned}
$$

这些约束共同保证 $s=|d|$。显式的 `bigM` 必须覆盖整个差值。

## Kotlin API

Kotlin 已提供原生二次符号，不需要线性适配器：

```kotlin
val slack = QuadraticSlackFunction(
    left = leftQuadratic,
    right = rightQuadratic,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack"
)
```

实现由两个精确正部函数组成，分别处理 $L-R$ 与 $R-L$；`evaluate` 返回绝对差，注册时会注册两个委托函数。

## Rust API

```rust
let slack = QuadraticSlackFunction::with_big_m(
    21, "quadratic_slack", left, right, 100.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

`new` 使用默认 Big-M 策略，`with_target` 将右侧设为常量，`with_big_m` 覆盖默认策略。结果变量是连续变量；机理注册四条绝对值约束，并且只为每个真正含二次项的输入添加二次桥接行。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticSlackFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64

val slack = QuadraticSlackFunction(
    left = leftQuadratic,
    right = rightQuadratic,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack"
)
check(slack.evaluate(values, null, IntoValue.Identity) == Flt64(1.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::function::QuadraticSlackFunction;

let slack = QuadraticSlackFunction::with_big_m(21, "quadratic_slack", left, right, 100.0_f64);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

:::

## 测试与参考

- Kotlin：[`QuadraticSlackFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlackFunctionTest.kt)
- Kotlin 实现：[`QuadraticSlack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlack.kt)
- Rust 实现：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust 测试：[`function_symbol_quadratic_slack.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_slack.rs)
