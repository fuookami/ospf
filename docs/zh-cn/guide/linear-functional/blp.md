# 二元线性分段函数

`BivariateLinearPiecewiseFunction` 表示定义在三角剖分上的分片平面。每个单元包含三个顶点 $(x_{tk},y_{tk},z_{tk})$。

## 求解器数学模型

令 $s_t\in\{0,1\}$ 选择三角形，$\lambda_{tk}\in[0,1]$ 为该三角形的重心权重。精确公式为

$$
\begin{aligned}
\sum_t s_t&=1,\\
\sum_{k=0}^{2}\lambda_{tk}&=s_t &&\forall t,\\
x&=\sum_{t,k}x_{tk}\lambda_{tk},\\
y&=\sum_{t,k}y_{tk}\lambda_{tk},\\
z&=\sum_{t,k}z_{tk}\lambda_{tk}.
\end{aligned}
$$

只有一个三角形可以具有非零权重。该模型不是所有顶点的无约束凸包，因此能够保留非共面单元之间的分段曲面。

## 直接求值

求值器对每个三角形计算重心坐标 $(\lambda_0,\lambda_1,\lambda_2)$。两种实现使用相同的几何容差 `1e-12`，允许权重低至 `-1e-12`。第一个满足所有权重在该容差内的三角形包含输入点，此时

$$
z=\lambda_0z_0+\lambda_1z_1+\lambda_2z_2.
$$

输入不属于任何三角形时返回 `null`/`None`。两种实现都会在构造时拒绝坐标非有限或二维行列式绝对值不大于 `1e-12` 的三角形，因此退化三角形无法进入求值。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
val surface = BivariateLinearPiecewiseFunction(
    x = xPolynomial,
    y = yPolynomial,
    triangles = triangles,
    converter = IntoValue.Identity,
    name = "surface"
)
```

```rust [Rust]
let surface = BivariateLinearPiecewiseFunction::new(
    1,
    "surface",
    x_input,
    y_input,
    vec![Triangle3::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 10.0),
        Point3::new(0.0, 1.0, 20.0),
    )],
);
assert_eq!(surface.selector_variables().len(), 1);
```

:::

## 测试与参考

- Kotlin 实现：[`BivariateLinearPiecewise.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BivariateLinearPiecewise.kt)
- Kotlin 独立测试：[`BivariateLinearPiecewiseFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/BivariateLinearPiecewiseFunctionDedicatedTest.kt)
- Rust 实现：[`bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/bivariate_linear_piecewise.rs)
- Rust 独立测试：[`function_symbol_bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_bivariate_linear_piecewise.rs)
