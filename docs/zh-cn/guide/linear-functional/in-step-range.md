# 步进区间

`InStepRangeFunction` 返回不超过上界表达式的最大步进点：

$$
y=lb+\left\lfloor\frac{ub-lb}{step}\right\rfloor step
$$

它是数值步进函数，不是布尔成员测试。`step` 必须是有限正数；求值和注册约束时会拒绝 `ub < lb`。

## 求解器数学模型

令 $d=ub-lb$、$k=\lfloor d/step\rfloor$、$y=lb+step\,k$。求解器接收委托 `FloorFunction` 的 $k$ 约束，以及显式顺序约束

$$
ub-lb\ge0
$$

商在取整前会除以 `step`；因此下界 1、上界 10、步长 3 时结果是 10，下界 1、上界 9 时结果是 7。

## Kotlin API

```kotlin
val stepped = InStepRangeFunction(
    lb = lower,
    ub = upper,
    step = Flt64(3.0),
    converter = IntoValue.Identity,
    name = "stepped"
)
```

## Rust API

Rust 现在提供相同的数值函数：

```rust
let stepped = InStepRangeFunction::new(
    1, "stepped", lower, upper, 3.0_f64,
);
```

原先的网格成员函数已经明确命名为 `InStepRangeIndicatorFunction`；只有在需要“是否为网格点”的二元结果时才使用它。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
val stepped = InStepRangeFunction(
    lb = constant(1.0),
    ub = constant(10.0),
    step = Flt64(3.0),
    converter = IntoValue.Identity,
    name = "stepped"
)
check(stepped.evaluate(emptyMap()) == Flt64(10.0))
```

```rust [Rust]
let stepped = InStepRangeFunction::new(1, "stepped", lower, upper, 3.0_f64);
assert_eq!(stepped.calculate_value(&tokens, false), Some(10.0));
```

:::

## 测试与参考

- Kotlin 数值聚焦测试：[`InStepRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/InStepRangeFunctionDedicatedTest.kt)
- Kotlin 成员聚焦测试：[`InStepRangeIndicatorFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/InStepRangeIndicatorFunctionDedicatedTest.kt)
- Kotlin 实现：[`InStepRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/InStepRange.kt)
- Rust 数值实现：[`in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/in_step_range.rs)
- Rust 数值聚焦测试：[`function_symbol_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_in_step_range.rs)
- Rust 成员聚焦测试：[`function_symbol_in_step_range_indicator.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_in_step_range_indicator.rs)

独立聚焦测试会断言数值函数的两个 Floor 辅助变量和四条注册约束，以及成员函数的七个辅助变量和十六条点/OR 约束；同时覆盖网格端点、缺失输入及非法 step/Big-M。

端点函数移除了未使用的 Big-M 参数，因为 FloorFunction 不需要 Big-M。
InStepRangeIndicatorFunction 已在 Kotlin 和 Rust 两端提供成员测试，并统一使用
1e-10 边界 epsilon。Kotlin 使用可选的 `bigM` 构造参数；Rust 在需要显式有限正值时使用
`InStepRangeIndicatorFunction::with_big_m`，否则 `new` 遵循令牌推导/默认策略。
