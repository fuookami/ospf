# 区间松弛

`SlackRangeFunction` 精确计算线性表达式到闭区间 $[lower,upper]$ 的距离：

$$
s=\max(lower-x,\ x-upper,\ 0)
$$

构造器要求 `lower <= upper`，并接收标量上下界：

```kotlin
SlackRangeFunction(
    input: LinearPolynomial<V>,
    lower: V,
    upper: V,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

当输入是线性中间符号时，可以使用 `fromLinearIntermediateSymbol` 创建适配器。结果多项式是内部精确 `MaxFunction` 的结果，不会因为没有放入目标函数而被任意放大。

## 求解器数学模型

令候选值 $p_0=lower-x$、$p_1=x-upper$、$p_2=0$，令 $s$ 为结果，$z_i$ 为二元选择变量，$M_i$ 为有效上界。实现注册：

$$
\begin{aligned}
s-p_i&\ge0,\\
s-p_i+M_i z_i&\le M_i\quad(i=0,1,2),\\
z_0+z_1+z_2&=1.
\end{aligned}
$$

这是精确最大值模型：区间内 $s=0$，区间外分别为 $lower-x$ 或 $x-upper$。

## Rust 对齐 API

```rust
let slack = SlackRangeFunction::new(
    1, "slack_range", input, -2.0_f64, 2.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(0.0));
```

Rust 同样使用标量上下界、最大值模型和相同的直接求值公式。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
val slack = SlackRangeFunction(
    input = input,
    lower = Flt64(-2.0),
    upper = Flt64(2.0),
    converter = IntoValue.Identity,
    name = "slack-range"
)
check(slack.evaluate(values) == Flt64(0.0))
```

```rust [Rust]
let slack = SlackRangeFunction::new(1, "slack_range", input, -2.0_f64, 2.0_f64);
assert_eq!(slack.calculate_value(&tokens, false), Some(0.0));
```

:::

## 测试与参考

- Kotlin：[`SlackRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SlackRangeFunctionDedicatedTest.kt)
- Kotlin 实现：[`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt)
- Rust 实现：[`slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs)
- Rust 独立聚焦测试：[`function_symbol_slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_slack_range.rs)

独立聚焦测试会断言四个辅助变量和显式 Big-M 下七条精确 Max 约束，并覆盖区间内外三种距离及反向边界。

两端都会校验有限且有序的上下界。Rust 还提供
SlackRangeFunction::with_big_m，要求显式 Big-M 为有限正数；默认构造器使用共享
回退或 token 推导策略。
