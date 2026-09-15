# 二次模型中的绝对松弛

`QuadraticSlackFunction` 精确表示

$$
s=|L(x)-R(x)|
$$

两种语言都先把二次输入桥接成线性表达式，再注册绝对值约束；结果不依赖目标函数是否最小化松弛量。

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

实现由两个精确正部函数组成，分别处理 $L-R$ 与 $R-L$；`polynomial` 是两者之和，`evaluate` 返回绝对差，注册时会注册两个委托函数。

## Rust API

```rust
let slack = QuadraticSlackFunction::with_big_m(
    21, "quadratic_slack", left, right, 100.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

`new` 使用默认 Big-M，`with_target` 将右侧设为常量。结果变量是连续变量，机理注册两个二次桥接和上面的四条约束。

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
check(slack.evaluate(values) == Flt64(1.0))
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
