# 二次区间指示

`QuadraticInStepRangeFunction` 是闭区间门控函数，不是 floor 或按步长取整函数。设二次多项式为 $p(x)$，且 $L\le U$，区间外分类容差为 $\varepsilon>0$：

$$
g_\varepsilon(p)=\begin{cases}
p(x), & L\le p(x)\le U,\\
0, & p(x)\le L-\varepsilon\ \text{或}\ p(x)\ge U+\varepsilon,\\
\text{未定义}, & \text{其他情况}.
\end{cases}
$$

未定义带显式说明了连续域的边界契约：严格补集无法由有限个非严格不等式精确编码。两种实现都创建三个互斥的二值指示变量 $z_{in}$、$z_{low}$、$z_{high}$，以及有符号结果变量 $y$。取正的 Big-M 常数 $M$，求解器模型为：

$$
\begin{aligned}
p(x)-Mz_{in} &\ge L-M,\\
p(x)+Mz_{in} &\le U+M,\\
p(x)+Mz_{low} &\le L-\varepsilon+M,\\
p(x)-Mz_{high} &\ge U+\varepsilon-M,\\
z_{in}+z_{low}+z_{high} &=1,\\
y-p(x)-Mz_{in} &\ge -M,\\
y-p(x)+Mz_{in} &\le M,\\
y-Mz_{in} &\le 0,\\
y+Mz_{in} &\ge 0.
\end{aligned}
$$

三状态 one-hot 划分保证：$[L,U]$ 内只能选择 inside 分支，低于或等于 $L-\varepsilon$ 时选择 low 分支，高于或等于 $U+\varepsilon$ 时选择 high 分支。最后四条约束仅在 inside 分支令 $y=p(x)$，其他分支令 $y=0$。

## API

::: code-group

```kotlin [Kotlin]
val p = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64.zero
)
val f = QuadraticInStepRangeFunction(
    x = p,
    lower = Flt64.zero,
    upper = Flt64(4.0),
    bigM = Flt64(100.0),
    outsideTolerance = Flt64(1e-6),
    converter = IntoValue.Identity,
    name = "square_gate"
)
```

```rust [Rust]
let p = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
    0.0,
);
let f = QuadraticInStepRangeFunction::with_parameters(
    11, "square_gate", p, 0.0, 4.0, 100.0, 1e-6,
);
```

:::

旧 Rust 步长取整构造器以及 `with_quadratic_bounds` API 已删除。如果数学含义确实是 $L+|s|\lfloor(U-L)/|s|\rfloor$，应使用独立的 floor 函数。

## 求值与注册

求值在闭区间内返回输入，在区间外容差之外返回零，在两侧未定义容差带内返回 `null`/`None`。注册时加入三个二值指示变量和有符号结果辅助变量，并提交九条约束。输入只有线性项时通过线性机制模型提交；包含二次项时通过二次机制模型提交。

## 测试与源码

- Kotlin 源码：[`QuadraticInStepRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticInStepRange.kt)
- Rust 源码：[`quadratic_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_in_step_range.rs)
- Kotlin 独立测试：[`QuadraticInStepRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticInStepRangeFunctionDedicatedTest.kt) 与 [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Rust 独立测试：[`function_symbol_quadratic_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_in_step_range.rs)

## 相关页面

- [线性区间步进](../linear-functional/in-step-range)
- [二次掩码范围](./quadratic-masking-range)
