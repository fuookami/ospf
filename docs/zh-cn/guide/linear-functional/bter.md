# 平衡三值化

## 契约

`BalanceTernaryzationFunction` 将线性表达式映射为 $-1$、$0$、$1$ 三个值。Kotlin 与 Rust 使用相同的阈值语义：

$$
y=\operatorname{BTer}_{\varepsilon}(x)=
\begin{cases}
-1, & x<-\varepsilon,\\
0, & -\varepsilon\le x\le\varepsilon,\\
1, & x>\varepsilon.
\end{cases}
$$

端点 $x=\pm\varepsilon$ 属于零分支。为表达求解器中的严格比较，两种实现使用相同的严格边界宽度 $\delta=10^{-10}$：$x\ge\varepsilon+\delta$ 为正，$x\le-\varepsilon-\delta$ 为负。开放过渡区 $(\varepsilon,\varepsilon+\delta)$ 与 $(-\varepsilon-\delta,-\varepsilon)$ 在直接求值时返回未定义（`null`/`None`），在注册模型中不可行。无法计算输入表达式时也不返回结果。

## API

::: code-group

```kotlin [Kotlin]
BalanceTernaryzationFunction(
    x: LinearPolynomial<V>,
    epsilon: Flt64 = Flt64(1e-6),
    converter: IntoValue<V>,
    name: String = "bter",
    displayName: String? = null,
    fallbackBigM: Flt64 = Flt64(1e6)
)
```

```rust [Rust]
BalanceTernaryzationFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    epsilon: V,
    fallback_big_m: V,
) -> Self
```

:::

两端实现都公开结果变量以及互斥的正、负二值变量。实现会优先根据输入推导有限界；只有无法推导时才使用回退 Big-M。

## 实际传给求解器的数学模型

令 $p,n\in\{0,1\}$ 分别表示正、负状态，$y$ 为结果，并选择很小的严格边界宽度 $\delta>0$（当前实现取 `1e-10`）。生成的模型为

$$
y=p-n,
$$

$$
p+n\le 1,
$$

$$
x-Mp\ge \varepsilon+\delta-M,
\qquad
x-Mp\le \varepsilon,
$$

$$
x+Mn\le M-\varepsilon-\delta,
\qquad
x+Mn\ge-\varepsilon.
$$

因此，$p=1$ 强制 $x\ge\varepsilon+\delta$，$n=1$ 强制 $x\le-\varepsilon-\delta$，而 $p=n=0$ 强制 $-\varepsilon\le x\le\varepsilon$。零带外侧紧邻的两个宽度为 $\delta$ 的开区间被有意设为不可行，使线性求解器无需分数过渡值即可表达严格比较；直接求值也遵循相同的定义域。

$M$ 必须覆盖输入绝对值范围再加上 $\varepsilon+\delta$。两端实现会尽可能由有限变量界推导 $M$，否则使用配置的回退值。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.BalanceTernaryzationFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val function = BalanceTernaryzationFunction(
    x = LinearPolynomial(emptyList(), Flt64(2.0)),
    epsilon = Flt64(0.1),
    converter = IntoValue.Identity,
    name = "direction"
)

check(function.evaluate(emptyMap()) == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::BalanceTernaryzationFunction;
use ospf_rust_core::token::VecTokenList;

let function = BalanceTernaryzationFunction::new(
    1,
    "direction",
    Linear::new(vec![], 2.0),
    0.1,
    1_000_000.0,
);
let tokens = VecTokenList::<f64>::new();

assert_eq!(function.calculate_value(&tokens, false), Some(1.0));
```

:::

## 源码与测试

- [Kotlin 实现](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BalanceTernaryzation.kt)
- [Kotlin 独立测试](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/BalanceTernaryzationFunctionDedicatedTest.kt)
- [Kotlin 示例](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example)
- [Rust 实现](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/balance_ternaryzation.rs)
- [Rust 独立测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_balance_ternaryzation.rs)
- [Rust 示例](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src)

## 相关页面

- [二值化](/zh-cn/guide/linear-functional/bin)
- [不等式指示](/zh-cn/guide/linear-functional/inequality)
- [一元线性分段函数](/zh-cn/guide/linear-functional/ulp)
