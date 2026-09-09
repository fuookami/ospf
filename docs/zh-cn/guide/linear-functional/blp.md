# 二元线性分段函数

`BivariateLinearPiecewiseFunction` 表示覆盖一组三角形的分段线性曲面。当前模型是三角剖分加重心插值，而不是独立点的矩形列表。

## 契约

- 输入：`x` 与 `y`，二者都是 `LinearPolynomial<V>`。
- 几何数据：非空的 `List<Triangle<Point<Dim3, Flt64>, Dim3, Flt64>>`；每个顶点存储 `(x, y, z)`。
- 输出：由重心权重和顶点 z 坐标构成的线性多项式。
- `evaluate` 返回首个包含输入点且非退化三角形的插值 z 值；任一输入缺失、点在所有三角形外，或所有候选三角形退化时返回 `null`。
- `V` 必须实现 `RealNumber<V>` 与 `NumberField<V>`；几何坐标是 `Flt64`，并通过 `IntoValue<V>` 转换。

## 数学定义

对于顶点为 $P_1=(x_1,y_1,z_1)$、$P_2=(x_2,y_2,z_2)$、$P_3=(x_3,y_3,z_3)$ 的三角形，三角形内的点用重心权重表示：

$$
\lambda_1=1-u-v,\qquad \lambda_2=u,\qquad \lambda_3=v,
$$

其中 $u\ge0$、$v\ge0$、$u+v\le1$。插值结果为

$$
z=\lambda_1z_1+\lambda_2z_2+\lambda_3z_3
 =z_1+(z_2-z_1)u+(z_3-z_1)v.
$$

实现从 x/y 坐标计算 `u`、`v`，并以包含边界的方式接受三角形边界。

## 适用域与边界

构造至少要求一个三角形。二维行列式绝对值不大于 $10^{-12}$ 的三角形视为退化，不能产生求值结果。点在所有三角形外时返回 `null`。三角形重叠时，求值使用列表中首个包含该点的三角形；如果相邻三角形的 z 值不一致，共享边界的结果会依赖列表顺序。solver 注册恰好选择一个三角形，并假定所列几何数据描述了预期域。

## 当前 API

### Kotlin

源码：[`BivariateLinearPiecewise.kt`（构造、重心求值与约束）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BivariateLinearPiecewise.kt#L58-L264)

```kotlin
BivariateLinearPiecewiseFunction(
    x: LinearPolynomial<V>,
    y: LinearPolynomial<V>,
    triangles: List<Triangle<Point<Dim3, Flt64>, Dim3, Flt64>>,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

### Rust

源码：[`bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/bivariate_linear_piecewise.rs)

Rust 使用非空的 `Point3<V>` 列表和凸组合模型，不同于 Kotlin 的三角形列表。主要构造器和访问器为：

```rust
Point3::new(x: V, y: V, z: V) -> Point3<V>
BivariateLinearPiecewiseFunction::new(
    id: u64,
    name: &str,
    x_input: Linear<V>,
    y_input: Linear<V>,
    points: Vec<Point3<V>>,
) -> Self
```

`result_variable()`、`lambda_variables()`、`x_input_polynomial()`、`y_input_polynomial()` 和 `points()` 暴露已注册的模型部分。Rust 求值器从 token 读取 lambda 变量并返回 z 的加权和；x/y 几何关系由机理约束保证。

## 辅助变量与注册模型

每个三角形的 `lambdaVars` 是形状为 3 的 `PctVariable1`，`zVars` 是三角形选择用的 `BinVariable1`。注册会约束 x、y 等于按 lambda 加权的顶点坐标，约束结果等于加权 z 坐标，将所有 lambda 之和设为 1，用选择变量门控每个三角形的 lambda 和，并将选择变量之和设为 1。百分比变量提供 `[0, 1]` 范围。

## `evaluate` 与 solver 的差异

`evaluate` 按顺序搜索三角形并返回重心插值或 `null`。solver 注册引入 one-hot 三角形选择和 lambda 变量，但不会修复重叠、不一致或退化的几何数据。因此 solver 模型应使用覆盖域与预期输入范围一致的连贯三角剖分。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.BivariateLinearPiecewiseFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.geometry.*
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val blp = BivariateLinearPiecewiseFunction(
    x = xPoly,
    y = yPoly,
    triangles = listOf(
        Triangle(
            point3(Flt64.zero, Flt64.zero, Flt64.zero),
            point3(Flt64.one, Flt64.zero, Flt64.one),
            point3(Flt64.zero, Flt64.one, Flt64.one)
        )
    ),
    converter = IntoValue.Identity,
    name = "blp"
)
val value = blp.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64(0.25), y to Flt64(0.25))
)
check(value != null && (value eq Flt64(0.75)))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{BivariateLinearPiecewiseFunction, Point3};
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};

let blp = BivariateLinearPiecewiseFunction::new(
    1,
    "blp",
    Linear::new(vec![], 0.25),
    Linear::new(vec![], 0.25),
    vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    ],
);
let mut tokens = VecTokenList::new();
for (lambda, value) in blp.lambda_variables().iter().zip([0.5, 0.25, 0.25]) {
    let token = Token::from_generic(lambda.clone(), lambda.index());
    token.set_result(value);
    tokens.add_token(token);
}
let value = <BivariateLinearPiecewiseFunction as FunctionSymbol>::calculate_value(
    &blp,
    &tokens,
    false,
);
assert_eq!(value, Some(0.5));
```

:::

完整示例：[`BLPTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BLPTest.kt)

Core 验证：[`TrigonometricAndBivariateGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/TrigonometricAndBivariateGenericEvaluateTest.kt)

Rust 实现与覆盖：[`bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/bivariate_linear_piecewise.rs)、[`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)

## 相关页面

- [`ulp`](./ulp)：从有序点进行一元分段插值。
- [`max`](./max) 与 [`min`](./min)：基于选择变量的线性函数。
- [`masking`](./masking)：线性输入的二进制门控。
