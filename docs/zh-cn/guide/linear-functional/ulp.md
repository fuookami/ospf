# 一元线性分段函数

`UnivariateLinearPiecewiseFunction` 表示一个输入的一元分段线性函数。当前最方便的工厂方法是从有序采样点构造。

## 契约

- 输入：`x: LinearPolynomial<V>`，以及至少两个二维点 `(x_i, y_i)`。
- 输出：`RealVar`（`resultVar`），通过 `resultPolynomial` 暴露。
- `.fromPoints` 为每一对相邻点计算斜率和截距。
- 当输入落在某个闭区间段内时，`evaluate` 返回首个匹配线段的仿射值；输入超出全部断点区间或无法求值时返回 `null`。
- `V` 还必须实现 `FloatingNumber<V>`，因为 `.fromPoints` 计算斜率和截距需要除法。

## 数学定义

对于严格递增的断点 $t_0<t_1<\cdots<t_m$，相邻点定义

$$
a_i=\frac{y_{i+1}-y_i}{t_{i+1}-t_i},\qquad b_i=y_i-a_i t_i,
$$

线段函数为

$$
f(x)=a_i x+b_i\quad\text{for }t_i\le x\le t_{i+1}
$$

实现按列表顺序检查线段，因此共享断点属于首个匹配线段。对于来自普通函数图像的点，相邻公式在该边界处一致。

## 适用域与边界

`.fromPoints` 要求至少两个点且 x 坐标严格递增。重复或降序 x 坐标会使 `fromPointsResult` 返回 `Failed`；便捷的 `.fromPoints` 会返回一个无效占位对象，之后在注册或结果边界处报告失败。需要显式处理构造错误时应使用 `.fromPointsResult`。求值定义在闭区间 `[t_0, t_m]` 上；低于 `t_0` 或高于 `t_m` 时返回 `null`。

直接构造器还要求 `breakpoints.size = slopes.size + 1 = intercepts.size + 1`。注册 solver 时，只有能证明输入范围有限才可省略 `m`；否则传入有效的 `m` Big-M。函数会从线段推导输出界，并校验值是否有限且可表示。

## 当前 API

### Kotlin

源码：[`UnivariateLinearPiecewise.kt`（构造与求值）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewise.kt#L60-L94) 与 [`fromPoints`/`fromPointsResult`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewise.kt#L921-L1047)

```kotlin
UnivariateLinearPiecewiseFunction.fromPoints(
    x: LinearPolynomial<V>,
    points: List<Point<Dim2, V>>,
    m: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

如果已经计算好表示，也可使用接受 `breakpoints`、`slopes` 和 `intercepts` 的直接构造器/工厂；`.fromPointsResult` 是安全的 `Ret` 返回版本。

### Rust

Rust 暴露 [`Point2`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs) 与 [`UnivariateLinearPiecewiseFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs)：

```rust
Point2::new(x: V, y: V) -> Point2<V>

UnivariateLinearPiecewiseFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    points: Vec<Point2<V>>,
) -> UnivariateLinearPiecewiseFunction<V>
```

Rust 符号按 x 排序采样点，创建凸组合 `lambda_variables()`，并暴露连续 `result_variable()`。其直接求值会把低于首点或高于末点的输入钳制到相应端点；Kotlin 的 `evaluate` 则在断点区间外返回 `null`。Rust 没有 Kotlin 的 `fromPoints`/`m` converter 参数，调用方直接构造 `Point2`。

## 辅助变量与注册模型

`helperVariables` 包含实数 `resultVar` 和每条线段一个二进制 `selectorVar`。注册要求恰好激活一条线段，用 Big-M 门控每段断点区间，并在激活段上门控 `result = slope * x + intercept`。提交 token 与约束前，还会校验或推导输入/输出范围。

## `evaluate` 与 solver 的差异

`evaluate` 是简单的闭区间线段扫描，超出断点区间返回 `null`。solver 注册更严格：省略 `m` 时需要有限输入范围，必须计算有限输出界，并添加 Big-M 线段门控。因此直接求值可用，并不意味着 solver 注册一定能证明有限范围或生成有效约束。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.UnivariateLinearPiecewiseFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.geometry.*
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val ulp = UnivariateLinearPiecewiseFunction.fromPoints(
    x = xPoly,
    points = listOf(
        point2(),
        point2(x = Flt64.one, y = Flt64.two),
        point2(x = Flt64.two, y = Flt64.one)
    ),
    converter = IntoValue.Identity,
    name = "y"
)
val value = ulp.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one))
check(value != null && (value eq Flt64.two))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{Point2, UnivariateLinearPiecewiseFunction};

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let ulp = UnivariateLinearPiecewiseFunction::new(
    1,
    "y",
    input,
    vec![
        Point2::new(0.0_f64, 0.0_f64),
        Point2::new(1.0_f64, 2.0_f64),
        Point2::new(2.0_f64, 1.0_f64),
    ],
);
assert_eq!(ulp.points().len(), 3);
let _result = ulp.result_variable();
```

:::

完整示例：[`ULPTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ULPTest.kt)

Core 验证：[`UnivariateLinearPiecewiseGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseGenericEvaluateTest.kt) 与 [`UnivariateLinearPiecewiseFailureBoundaryTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseFailureBoundaryTest.kt)

Rust 源码：[`univariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs)。

## 相关页面

- [`blp`](./blp)：基于三角剖分的双输入分段插值。
- [`max`](./max) 与 [`min`](./min)：在仿射候选之间离散选择。
- [`rounding`](./rounding)：整数输出而非连续插值。
