# 半连续变量

`SemiFunction<V>` 表示一个变量只能为零，或落在激活区间 `[lb, ub]` 内。

## 求解器数学模型

符号创建连续结果变量 $y$ 和二元激活变量 $b$，实际传给求解器的约束为

$$
y-ub\,b\le0,
\qquad
y-lb\,b\ge0.
$$

当 $b=0$ 时两条约束共同强制 $y=0$；当 $b=1$ 时得到
$lb\le y\le ub$。构造器检查 `lb <= ub`，并将结果变量和指示变量注册为辅助变量。

## Kotlin API

```kotlin
val semi = SemiFunction(
    lb = Flt64(2.0),
    ub = Flt64(5.0),
    converter = IntoValue.Identity,
    name = "semi"
)
```

`SemiFunction.from(variable, ...)` 可从连续变量推导缺失的有限边界；`resultVar`、`indicatorVar` 和 `resultPolynomial` 暴露模型表示。

## Rust API

```rust
let semi = SemiFunction::new(1, "semi", 2.0_f64, 5.0_f64);
assert_eq!(semi.lower_bound(), &2.0);
assert_eq!(semi.upper_bound(), &5.0);
```

Rust 还提供 `try_from_variable`/`from_variable` 推导有限边界。两种实现注册相同的两条域约束。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
val semi = SemiFunction(
    lb = Flt64(2.0),
    ub = Flt64(5.0),
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.helperVariables.size == 2)
```

```rust [Rust]
let semi = SemiFunction::new(1, "semi", 2.0_f64, 5.0_f64);
assert_eq!(semi.lower_bound(), &2.0);
assert_eq!(semi.upper_bound(), &5.0);
```

:::

## 测试与参考

- Kotlin 独立聚焦测试：[`SemiFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SemiFunctionDedicatedTest.kt)
- Kotlin 回归测试：[`SemiFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SemiFunctionTest.kt)
- Kotlin 实现：[`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt)
- Rust 实现：[`semi.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs)
- Rust 独立聚焦测试：[`function_symbol_semi.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_semi.rs)

独立聚焦测试会断言辅助 token 和两条实际域约束行，并覆盖共享默认区间以及反向/非有限边界的提前校验。

上下界会提前校验，必须有限且下界不大于上界。Kotlin 省略边界时使用共享
区间 [0, 1e6]；Rust 通过 SemiFunction::with_default_bounds 提供相同默认值。
缺失 token 的语义保持显式：Kotlin 返回 null，Rust 遵循 zero_if_none。
