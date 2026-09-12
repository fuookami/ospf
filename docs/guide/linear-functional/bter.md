# Balance ternaryzation

## Contract

`BalanceTernaryzationFunction` maps a linear expression to the three values $-1$, $0$, and $1$. Kotlin and Rust use the same threshold semantics:

$$
y=\operatorname{BTer}_{\varepsilon}(x)=
\begin{cases}
-1, & x<-\varepsilon,\\
0, & -\varepsilon\le x\le\varepsilon,\\
1, & x>\varepsilon.
\end{cases}
$$

The endpoints $x=\pm\varepsilon$ belong to the zero branch. To represent the
strict solver comparisons, both implementations use the same strict-boundary
width $\delta=10^{-10}$: $x\ge\varepsilon+\delta$ is positive and
$x\le-\varepsilon-\delta$ is negative. The open transition intervals
$(\varepsilon,\varepsilon+\delta)$ and
$(-\varepsilon-\delta,-\varepsilon)$ are undefined (`null`/`None`) in direct
evaluation and infeasible in the registered model. Inputs outside every
defined branch, including a missing variable value, also return no value.

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

Both implementations expose the result variable and the mutually exclusive positive and negative binary variables. A finite bound inferred from the input is preferred; the fallback Big-M is used only when such a bound is unavailable.

## Mathematical model passed to the solver

Let $p,n\in\{0,1\}$ denote the positive and negative states, let $y$ be the result, and choose a small strict-boundary width $\delta>0$ (`1e-10` in the current implementations). The generated model is

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

Thus $p=1$ forces $x\ge\varepsilon+\delta$, $n=1$ forces $x\le-\varepsilon-\delta$, and $p=n=0$ forces $-\varepsilon\le x\le\varepsilon$. The open intervals immediately outside the zero band, whose width is $\delta$, are deliberately infeasible so that a linear solver can represent the strict comparisons without fractional transition values. Direct evaluation follows these same defined regions.

$M$ must cover the absolute input range plus $\varepsilon+\delta$. Both implementations infer it from finite variable bounds when possible and otherwise use the configured fallback.

## Minimal example

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

## Source and tests

- [Kotlin implementation](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BalanceTernaryzation.kt)
- [Kotlin dedicated test](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/BalanceTernaryzationFunctionDedicatedTest.kt)
- [Kotlin example](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example)
- [Rust implementation](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/balance_ternaryzation.rs)
- [Rust dedicated test](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_balance_ternaryzation.rs)
- [Rust examples](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src)

## Related pages

- [Binaryzation](/guide/linear-functional/bin)
- [Inequality indicator](/guide/linear-functional/inequality)
- [Univariate linear piecewise function](/guide/linear-functional/ulp)
