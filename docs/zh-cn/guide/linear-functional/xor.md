# Xor（恰好一个）

`XorFunction` 当且仅当恰好一个输入表达式非零时返回一。对两个输入，它等同于通常的 XOR；对三个及以上输入，它是“恰好一个”谓词，不是奇数奇偶校验。

## 数学定义

令非零指示变量为 $a_i\in\{0,1\}$，结果为 $y\in\{0,1\}$：

$$
y=1\iff\sum_i a_i=1.
$$

Kotlin 与 Rust 使用相同的精确线性编码：

$$
\begin{aligned}
y&\le\sum_i a_i,\\
y&\ge a_i-\sum_{j\ne i}a_j &&\forall i,\\
y+a_i+a_j&\le2 &&\forall i<j.
\end{aligned}
$$

第一条在所有输入均为零时强制 $y=0$；第二组在只有一个指示变量激活时强制 $y=1$；成对约束在至少两个指示变量激活时强制 $y=0$。

## 非零指示模型

每个输入通过共享的非零 Big-M 公式连接到一个指示变量和一个符号侧辅助变量。两种实现的默认零带容差都是 `1e-10`，严格非零边界约为 `1.6e-9`。Kotlin 的直接构造器暴露 `bigM`、`tolerance` 和 `strictBoundary`；Rust 暴露对应的 `with_big_m`、`with_tolerance`、`with_strict_boundary` 和 `with_parameters` 构造器。`abs(input) <= tolerance` 视为零；`tolerance < abs(input) < strictBoundary` 的开放过渡区返回未定义（`null`/`None`），因为求解器会刻意拒绝这一区间；达到严格边界才视为非零。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
val exactlyOne = XorFunction(
    polynomials = listOf(a, b, c),
    converter = IntoValue.Identity,
    name = "exactly-one"
)
check(exactlyOne.evaluate(valuesWithOnlyA) == Flt64.one)
check(exactlyOne.evaluate(valuesWithAAndB) == Flt64.zero)
```

```rust [Rust]
let exactly_one = XorFunction::new(1, "exactly_one", vec![a, b, c]);
assert_eq!(exactly_one.calculate_value(&only_a_tokens, false), Some(1.0));
assert_eq!(exactly_one.calculate_value(&a_and_b_tokens, false), Some(0.0));
```

:::

Rust 也可以一次配置全部数值策略：

```rust
let configured = XorFunction::new(2, "configured_xor", vec![a, b])
    .with_parameters(Some(100.0), 1e-8, 1e-6);
```

## 测试与参考

- Kotlin：[`XorFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/XorFunctionDedicatedTest.kt)
- Kotlin 实现：[`And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- Rust：[`function_symbol_xor.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_xor.rs)
- Rust 实现：[`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)
