# 正弦

`SinFunction` 是正弦函数的采样分段线性近似。它是线性建模原语，不是精确的三角函数求解器函数。

> [!WARNING]
> 默认模型只在 $[-\pi,\pi]$ 上使用五个采样点定义。断点域外的值求值为 `null`，采样点之间使用直线插值。

## 契约

- 输入：`x: LinearPolynomial<V>`。
- 输出：由委托的一元分段函数提供的线性多项式 `result`。
- 采样点：包含 $(x,\sin x)$ 点的 `List<Point<Dim2, Flt64>>`；默认列表由实现生成。
- 泛型值使用 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。
- 注册时添加普通一元分段线性实现的辅助变量和约束。

## 定义与数学模型

对于有序采样点 $(a_i,b_i)$，每段为

$$
s_i=\frac{b_{i+1}-b_i}{a_{i+1}-a_i},\qquad c_i=b_i-s_i a_i,\qquad
y=s_i x+c_i\quad(a_i\le x\le a_{i+1}).
$$

默认采样点严格为：

$$
(-\pi,0),\;(-\frac{\pi}{2},-1),\;(0,0),\;(\frac{\pi}{2},1),\;(\pi,0).
$$

因此默认实现是五点正弦线性插值，没有周期扩展，也不会精确计算 $\sin(x)$。

## 实现、辅助变量与约束

`SinFunction` 延迟构造一个 `UnivariateLinearPiecewiseFunction`，其内部名称是传入的 `name` 后接 `_impl`。辅助变量、辅助 token 注册和分段约束全部委托给该实现；分段选择使用普通分段建模，公开结果仍是委托实现的线性结果。

## 当前 API

### Kotlin

源码：[`Sin.kt`（`SinFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Sin.kt#L36-L118)

工厂也接受显式的 `samplingPoints` 列表。交给分段实现注册或求值时，点必须至少有两个、值为有限数，且 x 坐标严格递增。

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SinFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val sine = SinFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "sine"
)
val value = sine.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value != null && value == Flt64.zero)
```

### Rust

Rust 暴露 [`SinFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/sin.rs)：

```rust
SinFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
) -> SinFunction<V>
```

该构造器把建模网格固定为 [-π,π] 上的 32 个分段，创建有界连续 `result_variable()` 并注册分段线性约束。Rust 的 token 求值器直接计算 `sin(input)`，没有 Kotlin 风格可配置的 `samplingPoints` 参数；因此直接求值可能与分段模型不同，尤其是在建模网格之外。

## evaluate 与 solver 的差异

`evaluate` 使用与注册相同的分段线性插值；输入缺失或 x 落在首尾断点之外时返回 `null`。solver 注册不会添加精确三角函数关系，而是注册分段线性近似及其 Big-M/分段约束。

## 边界、tolerance 与 Undefined

该函数没有自己的三值条件判定器或 tolerance 参数，相关边界是数值转换和分段校验。采样点少于两个、存在非有限值、x 坐标重复或逆序时，委托的分段实现校验失败。可用分段的端点按闭区间处理。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SinFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val sine = SinFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "sine"
)
val value = sine.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value != null && value == Flt64.zero)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SinFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let sine = SinFunction::new(1, "sin", input);
let _result = sine.result_variable();
```

:::

- Core 测试：[`TrigonometricAndBivariateGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/TrigonometricAndBivariateGenericEvaluateTest.kt)
- 示例目录（当前没有专门的正弦文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust 源码：[`sin.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/sin.rs)。

## 相关页面

- [余弦](./cos)
- [一元分段线性](./ulp)
- [二元分段线性](./blp)
