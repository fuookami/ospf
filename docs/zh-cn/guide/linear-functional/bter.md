# 平衡三值化

## 契约

`BalanceTernaryzationFunction<V>` 将线性多项式映射到 $-1$、$0$ 和 $1$ 三个值。当前函数是带阈值的符号函数，不是二值对 $(y'_p,y'_n)$，也不区分离散模式和连续模式。

## 定义与分段值

对于输入值 $x$ 和 `epsilon = \varepsilon`：

$$
y = \operatorname{BTer}(x) = \begin{cases}
1, & x > \varepsilon \\
0, & -\varepsilon \le x \le \varepsilon \\
-1, & x < -\varepsilon
\end{cases}
$$

在直接求值器中，$x=\varepsilon$ 和 $x=-\varepsilon$ 都属于零分支。默认 $\varepsilon$ 为 `Flt64(1e-6)`。

## 边界、tolerance 与 Undefined

该函数没有 `tolerance` 或 `Undefined` 结果。只有在无法求得输入多项式时，`evaluate()` 才返回 `null`。阈值是公开的 `epsilon` 参数，两个非零分支使用严格比较。

求解器表示由 `UnivariateLinearPiecewiseFunction` 构建，并在 $-\varepsilon$ 和 $+\varepsilon$ 附近加入 `Flt64(1e-10)` 的过渡精度。因此求解器模型是带窄斜坡段的分段线性近似；不能仅根据直接 `evaluate()` 分支推断求解器在端点的精确行为，应检查生成的分段。

## 当前 API

### Kotlin

```kotlin
BalanceTernaryzationFunction(
    x: LinearPolynomial<V>,
    epsilon: Flt64 = Flt64(1e-6),
    extract: Boolean = true,
    converter: IntoValue<V>,
    name: String = "bter",
    displayName: String? = null,
    fallbackLower: Flt64 = Flt64(-1e6),
    fallbackUpper: Flt64 = Flt64(1e6)
)
```

`extract` 为兼容性保留，目前未使用；实现始终创建分段辅助函数。`fallbackLower` 与 `fallbackUpper` 用于补充缺失的断点端点，但嵌套分段函数注册时仍需要可证明的有限输入范围以自动推导 Big-M。

### Rust

源码：[`balance_ternaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/balance_ternaryzation.rs)

Rust 有同名辅助函数，但它并不是 Kotlin 阈值输入函数的一一对应 API：不接收输入 `Linear<V>`、`epsilon` 或回退范围。`BalanceTernaryzationFunction::new(id, name)` 创建一个连续结果变量以及正、负两个二值变量；其值为 `positive - negative`。

```rust
BalanceTernaryzationFunction::new(id: u64, name: &str) -> Self
BalanceTernaryzationFunction::result_variable(&self) -> &ContinuousVariableItem
BalanceTernaryzationFunction::positive_variable(&self) -> &BinaryVariableItem
BalanceTernaryzationFunction::negative_variable(&self) -> &BinaryVariableItem
```

当前 Rust 没有 Kotlin `Linear<V> + epsilon` 阈值签名的直接 API。要复现该契约，应围绕输入多项式组合二值化/条件函数，再把正、负指示量交给该辅助函数；或者仅在正、负指示量已经存在时使用它。

## 辅助变量与注册模型

函数创建名为 ``name`_impl` 的内部 `UnivariateLinearPiecewiseFunction`。其结果通过 `result` 暴露；`helperVariables` 委托给嵌套分段函数。嵌套函数为每个分段创建一个实数结果变量和一个二值选择变量。

`registerAuxiliaryTokens` 与 `registerConstraints` 都委托给该嵌套函数。嵌套注册要求恰好一个分段激活，为各选择变量分段施加范围，并用线性 Big-M 不等式把结果连接到分段斜率和截距。

## `evaluate()` 与求解器模型的差异

直接求值遵循上面的三分支公式。求解器使用生成的斜坡：在每个阈值附近可能产生插值结果，而不是精确的 $-1$、$0$ 或 $1$；在闭端点处，首个匹配的嵌套分段决定直接嵌套求值。该差异是当前实现警告，不应当作额外契约。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.BalanceTernaryzationFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val xPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64.zero
    )
    val function = BalanceTernaryzationFunction(
        x = xPoly,
        epsilon = Flt64(1e-6),
        converter = IntoValue.Identity,
        name = "bter"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(2.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero)) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-2.0))) == Flt64(-1.0))
}
```

```rust [Rust]
use ospf_rust_core::symbol::function::BalanceTernaryzationFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};

let function = BalanceTernaryzationFunction::new(1, "bter");
let positive = function.positive_variable().clone();
let positive_token = Token::from_generic(positive.clone(), positive.index());
positive_token.set_result(1.0);
let negative = function.negative_variable().clone();
let negative_token = Token::from_generic(negative.clone(), negative.index());
negative_token.set_result(0.0);
let mut tokens = VecTokenList::new();
tokens.add_token(positive_token);
tokens.add_token(negative_token);
let value = <BalanceTernaryzationFunction as FunctionSymbol>::calculate_value(
    &function,
    &tokens,
    false,
);
assert_eq!(value, Some(1.0));
```

:::

## Source and core tests

- [Implementation: `BalanceTernaryzation.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BalanceTernaryzation.kt)
- [Core generic evaluation test: `FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)
- [Complete example: `BTerTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BTerTest.kt)
- [Rust implementation: `balance_ternaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/balance_ternaryzation.rs)
- [Rust core coverage: `p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

## 相关页面

- [二值化](/guide/linear-functional/bin)
- [单位线性分段函数](/guide/linear-functional/ulp)
- [条件 IF](/guide/linear-functional/if)
