# 一元线性分段函数

`UnivariateLinearPiecewiseFunction` 表示对连续采样点 $(t_i,f_i)$ 逐段线性插值得到的函数图形。采样点至少有两个，$t_i$ 必须有限且严格递增；$[t_0,t_m]$ 之外的求值未定义。

## 求解器数学模型

Kotlin 为每条线段创建二元选择变量 $z_j$。令该段的仿射公式为 $g_j(x)=a_jx+b_j$，实现强制 $\sum_jz_j=1$，用有限 Big-M 把 $x$ 限制到所选区间，并门控 $y=g_j(x)$。

Rust 使用采样点权重 $\lambda_i\in[0,1]$ 和相同的一热段选择变量 $z_j\in\{0,1\}$，并强制

$$
\begin{aligned}
\sum_i\lambda_i&=1,&\sum_j z_j&=1,\\
x&=\sum_i t_i\lambda_i,&y&=\sum_i f_i\lambda_i.
\end{aligned}
$$

相邻性约束为

$$
\lambda_0\le z_0,
\qquad
\lambda_m\le z_{m-1},
\qquad
\lambda_i\le z_{i-1}+z_i\quad(0<i<m).
$$

因此只有所选线段的两个端点可以具有正权重。两边内部公式虽然不同，但现在都表示单一激活线段，并排除任意混合不相邻采样点的凸包行为。

## Kotlin/Rust 示例

::: code-group

```kotlin [Kotlin]
val piecewise = UnivariateLinearPiecewiseFunction.fromPoints(
    x = input,
    points = points,
    converter = IntoValue.Identity,
    name = "ulp"
)
```

```rust [Rust]
let piecewise = UnivariateLinearPiecewiseFunction::new(
    1,
    "ulp",
    input,
    vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 2.0),
        Point2::new(2.0, 0.0),
    ],
);
assert_eq!(piecewise.selector_variables().len(), 2);
```

:::

## 求值与边界

在 $[t_i,t_{i+1}]$ 上，直接求值采用

$$
y=f_i+\frac{x-t_i}{t_{i+1}-t_i}(f_{i+1}-f_i).
$$

输入缺失或超出采样点定义域时，Kotlin 返回 `null`，Rust 返回 `None`。重复、降序、非有限或数量不足的采样点会被拒绝。

## 测试与参考

- Kotlin 独立聚焦测试：[`UnivariateLinearPiecewiseFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseFunctionDedicatedTest.kt)
- Kotlin 边界/注册测试：[`UnivariateLinearPiecewiseFailureBoundaryTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseFailureBoundaryTest.kt)
- Kotlin 实现：[`UnivariateLinearPiecewise.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewise.kt)
- Rust 实现：[`univariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs)
- Rust 独立测试：[`function_symbol_univariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_univariate_linear_piecewise.rs)

独立聚焦测试会断言辅助变量数量和每一条选择/分段图形约束行，并覆盖端点、定义域外求值及非法采样点校验。

点形式和线段形式共享严格递增的断点契约。Kotlin 直接接收
breakpoints/slopes/intercepts；Rust 还提供
UnivariateLinearPiecewiseFunction::from_segments，并要求每个相邻断点对
对应一个斜率和截距。非法值会在注册辅助变量前拒绝。
