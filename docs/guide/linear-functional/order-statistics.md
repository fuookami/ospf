# Order Statistics

`ArgMinFunction`, `ArgMaxFunction`, `KthLargestFunction`, and `TopKSumFunction` select or summarize a finite list of linear expressions. They compose exact pairwise `MinFunction`/`MaxFunction` selectors, so their result does not rely on minimizing or maximizing that result in the objective.

Every candidate expression must have a finite range for exact selector constraints. Kotlin validates finite bounds when the function is created. Rust derives candidate ranges from registered token domains when it builds the mechanism; each variable used by a candidate therefore needs finite declared bounds.

## ArgMin and ArgMax

For candidates $x_0,\ldots,x_{n-1}$, the result is a zero-based index:

$$
\operatorname{argmin}(x)=i\quad\text{for an }i\in\arg\min_j x_j,
\qquad
\operatorname{argmax}(x)=i\quad\text{for an }i\in\arg\max_j x_j.
$$

The candidate list must be non-empty. If multiple candidates tie, the model may choose any optimal index; direct semantic evaluation deterministically returns the smallest tied index. Use the returned selector polynomial in another expression or constraint when the selected index matters.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ArgMinFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.functional.*

fun main() {
    val candidates = listOf(6.0, 2.0, 2.0).map {
        LinearPolynomial(emptyList(), Flt64(it))
    }
    val argmin = when (val created = ArgMinFunction(
        polynomials = candidates,
        converter = IntoValue.Identity,
        name = "cheapest"
    )) {
        is Ok -> created.value
        is Failed, is Fatal -> return
    }
    check(argmin.evaluate(emptyMap()) == Flt64.one)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::{FunctionSymbol, function::ArgMinFunction};
use ospf_rust_core::token::VecTokenList;

fn main() {
    let candidates = vec![Linear::constant(6.0), Linear::constant(2.0), Linear::constant(2.0)];
    let argmin = ArgMinFunction::new(10, "cheapest", candidates).unwrap();
    let tokens = VecTokenList::<f64>::new();
    assert_eq!(FunctionSymbol::calculate_value(&argmin, &tokens, false), Some(1.0));
}
```

:::

Use `ArgMaxFunction` with the same input shape to return the index of a maximum. The model's tie choice remains arbitrary; direct evaluation picks the first tied maximum.

## K-th largest value

`KthLargestFunction` returns the value at a zero-based descending rank:

$$
x_{(0)}\ge x_{(1)}\ge\cdots\ge x_{(n-1)},
\qquad y=x_{(k)},\quad 0\le k<n.
$$

The list must be non-empty and $k$ must be less than its length. Equal candidate values do not make the result ambiguous: tied ranks return that same value. The comparison network is exact for any objective direction.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.KthLargestFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.functional.*

fun main() {
    val candidates = listOf(7.0, 4.0, 9.0, 2.0).map {
        LinearPolynomial(emptyList(), Flt64(it))
    }
    val thirdLargest = when (val created = KthLargestFunction(
        polynomials = candidates,
        k = 2,
        converter = IntoValue.Identity,
        name = "third_largest"
    )) {
        is Ok -> created.value
        is Failed, is Fatal -> return
    }
    check(thirdLargest.evaluate(emptyMap()) == Flt64(4.0))
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::{FunctionSymbol, function::KthLargestFunction};
use ospf_rust_core::token::VecTokenList;

fn main() {
    let candidates = vec![Linear::constant(7.0), Linear::constant(4.0), Linear::constant(9.0), Linear::constant(2.0)];
    let third_largest = KthLargestFunction::new(11, "third_largest", candidates, 2).unwrap();
    let tokens = VecTokenList::<f64>::new();
    assert_eq!(FunctionSymbol::calculate_value(&third_largest, &tokens, false), Some(4.0));
}
```

:::

## Sum of the largest k values

`TopKSumFunction` returns

$$
y=\sum_{j=0}^{k-1}x_{(j)},\qquad 0\le k\le n.
$$

The empty sum at $k=0$ is zero; at $k=n$ the result is the sum of every candidate. Both endpoints are supported, including an empty candidate list only when $k=0$. The formulation is an exact comparison network, independent of objective direction.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.TopKSumFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.functional.*

fun main() {
    val candidates = listOf(7.0, 4.0, 9.0, 2.0).map {
        LinearPolynomial(emptyList(), Flt64(it))
    }
    val topTwo = when (val created = TopKSumFunction(
        polynomials = candidates,
        k = 2,
        converter = IntoValue.Identity,
        name = "top_two_sum"
    )) {
        is Ok -> created.value
        is Failed, is Fatal -> return
    }
    check(topTwo.evaluate(emptyMap()) == Flt64(16.0))
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::{FunctionSymbol, function::TopKSumFunction};
use ospf_rust_core::token::VecTokenList;

fn main() {
    let candidates = vec![Linear::constant(7.0), Linear::constant(4.0), Linear::constant(9.0), Linear::constant(2.0)];
    let top_two = TopKSumFunction::new(12, "top_two_sum", candidates, 2).unwrap();
    let tokens = VecTokenList::<f64>::new();
    assert_eq!(FunctionSymbol::calculate_value(&top_two, &tokens, false), Some(16.0));
}
```

:::

## Related functions

- [`ElementFunction`](./products-selection) selects one table value by an integer key.
- [`MinFunction`](./min) and [`MaxFunction`](./max) select the minimum or maximum value without returning its index.
