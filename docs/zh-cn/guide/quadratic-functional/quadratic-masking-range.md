# 二次掩码范围

`QuadraticMaskingRangeFunction` 在 Kotlin 与 Rust 中统一表示“二值掩码二次表达式”。设二次多项式为 $p(x)$，二值掩码为 $z$，有符号结果变量为 $y$：

$$
y = \begin{cases}
p(x), & z=1,\\
0, & z=0。
\end{cases}
$$

两种实现向求解器提交完全相同的四条 Big-M 约束，其中 $M$ 必须是正的有限常数：

$$
\begin{aligned}
y-p(x)+Mz &\le M,\\
y-p(x)-Mz &\ge -M,\\
y &\le Mz,\\
y &\ge -Mz。
\end{aligned}
$$

掩码必须是二值变量，$y$ 必须是有符号实变量。当 $p(x)$ 可能为负时，最后一条约束不能省略；无下界辅助变量不能表达同一模型。

## API

::: code-group

```kotlin [Kotlin]
val z = BinVar("z")
val p = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64.zero
)
val f = QuadraticMaskingRangeFunction(
    polynomial = p,
    z = z,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "masked_square"
)
```

```rust [Rust]
let z = BinaryVariableItem::create(VariableId::standalone(2), "z");
let p = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
    0.0,
);
let f = QuadraticMaskingRangeFunction::with_big_m(
    2, "masked_square", p, z.clone(), 100.0,
);
```

:::

`with_quadratic_bounds` 以及旧的“上下界范围变量”契约已经删除。它描述的是另一种函数，不再属于该 API。

## 求值与注册

求值时，`z=0` 直接返回零，不读取结果 token；`z=1` 返回 $p(x)$。注册时创建有符号结果辅助变量，并提交上述四条约束。输入只有线性项时提交线性约束；输入包含二次项时提交二次约束。

## 测试与源码

- Kotlin 源码：[`QuadraticMaskingRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaskingRange.kt)
- Rust 源码：[`quadratic_masking_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_masking_range.rs)
- Kotlin 独立测试：[`QuadraticMaskingRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaskingRangeFunctionDedicatedTest.kt) 与 [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Rust 独立测试：[`function_symbol_quadratic_masking_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_masking_range.rs)

## 相关页面

- [线性掩码](../linear-functional/masking)
- [二次区间指示](./quadratic-in-step-range)
