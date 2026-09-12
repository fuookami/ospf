# 二次模型中的区间松弛

`QuadraticSlackRangeFunction` 精确表示二次表达式到闭区间 $[lower,upper]$ 的距离：

$$
s=\max(lower-p(x),\ p(x)-upper,\ 0)
$$

构造器要求 `lower <= upper`。

## 求解器数学模型

令 $a=lower-p(x)$、$b=p(x)-upper$。实现分别对 $a$、$b$ 注册精确正部函数，并将两者相加。等价地，令 $s\ge0$，$z_0,z_1,z_2$ 为二元选择变量，$M_i$ 为有效上界，则实际约束为

$$
\begin{aligned}
s&\ge a,&s&\ge b,&s&\ge0,\\
s&\le a+M_0(1-z_0),&s&\le b+M_1(1-z_1),&s&\le M_2(1-z_2),\\
z_0+z_1+z_2&=1.
\end{aligned}
$$

因此区间内部结果为零，区间外结果为精确的单侧违反量。

## Kotlin API

Kotlin 提供原生二次符号：

```kotlin
val slack = QuadraticSlackRangeFunction(
    input = inputQuadratic,
    lower = Flt64(1.0),
    upper = Flt64(2.0),
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack-range"
)
```

对于线性表达式，使用 `SlackRangeFunction(input, lower, upper, bigM,
converter, name)`；它同样使用精确最大值模型，并公开内部 `MaxFunction` 的结果多项式。

## Rust API

```rust
let slack = QuadraticSlackRangeFunction::new(
    23, "quadratic_slack_range", input, 1.0_f64, 2.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

二次输入先桥接为线性变量，再交给精确的线性区间距离模型；`result_variable()` 是连续变量。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticSlackRangeFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64

val slack = QuadraticSlackRangeFunction(
    input = inputQuadratic,
    lower = Flt64(1.0),
    upper = Flt64(2.0),
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack-range"
)
check(slack.evaluate(values) == Flt64(1.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::function::QuadraticSlackRangeFunction;

let slack = QuadraticSlackRangeFunction::new(
    23, "quadratic_slack_range", input, 1.0_f64, 2.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

:::

## 测试与参考

- Kotlin：[`QuadraticSlackRangeFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlackRangeFunctionTest.kt)
- Kotlin 实现：[`QuadraticSlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlackRange.kt)
- Rust 实现：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust 辅助/行聚焦测试：[`function_symbol_quadratic_slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_slack_range.rs)
- Rust 线性区间实现：[`slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs)
