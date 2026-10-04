# Products, Selection, and Lookup

These functions cover products and indexed choices while preserving a linear solver model. Their guarantees differ: bounded-integer products and binary selection are exact, while a general continuous product uses a McCormick relaxation.

## Bounded integer product

For a bounded integer variable $n$ and a bounded linear expression $x$, `IntegerProductFunction` represents

$$
y=nx.
$$

The integer is shifted to zero, encoded with binary variables, and capped at its declared width. Each bit-expression product is gated exactly, so the result is an exact MILP formulation. The integer bounds must be finite, integral, and ordered; the expression $x$ must also have finite bounds. Kotlin infers those bounds from registered variable ranges when possible. Explicit Kotlin `ConditionBounds` are enforced as input-domain rows and must therefore be valid. Rust uses flattened expressions, so the constructor requires the expression bounds explicitly; provide bounds that hold for every feasible input.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IntegerProductFunction
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val units = IntVar("units")
    units.range.geq(Int64.zero)
    units.range.leq(Int64(10))
    val price = RealVar("price")
    price.range.geq(Flt64.zero)
    price.range.leq(Flt64(8.0))
    val priceExpression = LinearPolynomial(
        listOf(LinearMonomial(Flt64.one, price)), Flt64.zero
    )
    val total = IntegerProductFunction(
        integer = units,
        input = priceExpression,
        converter = IntoValue.Identity,
        inputBounds = ConditionBounds(Flt64.zero, Flt64(8.0)),
        name = "total"
    )
}
```

```rust [Rust]
use std::sync::Arc;

use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IntegerProductFunction;
use ospf_rust_core::variable::{ContinuousVariableItem, IntegerVariableItem, VariableId, VariableRange};

fn main() {
    let mut model = BasicModel::new("integer_product");
    let units = IntegerVariableItem::with_range(
        VariableId::standalone(1), "units", VariableRange::bounded(0.0, 10.0)
    );
    let _units_index = model.register_variable(units.clone()).unwrap();
    let price = ContinuousVariableItem::with_range(
        VariableId::standalone(2), "price", VariableRange::bounded(0.0, 8.0)
    );
    let price_index = model.register_variable(price.clone()).unwrap();
    let price_expression = Linear::new(vec![LinearMonomial::new(1.0, price_index)], 0.0);
    let total = IntegerProductFunction::new(
        1, "total", units, price_expression, (0.0, 8.0)
    ).unwrap();
    model.add_symbol(Arc::new(total)).unwrap();
}
```

:::

## Binary selection

`SelectFunction` (also named `IfThenElseFunction`) selects one of two linear expressions with a binary variable $b$:

$$
y=\begin{cases}a,&b=1,\\c,&b=0.\end{cases}
\qquad y=c+b(a-c).
$$

The formulation is exact in MILP. Both branch expressions need finite ranges for the masking constraints. Kotlin can infer ranges or accept explicit `thenBounds` and `otherwiseBounds`; explicit bounds are enforced as domain rows. Rust requires both bounds in the constructor. A non-binary selector is rejected or has no defined value.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SelectFunction
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val selected = SelectFunction(
        mask = BinVar("use_premium"),
        then = LinearPolynomial(emptyList(), Flt64(12.0)),
        otherwise = LinearPolynomial(emptyList(), Flt64(8.0)),
        converter = IntoValue.Identity,
        name = "selected_cost"
    )
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::SelectFunction;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

fn main() {
    let selector = BinaryVariableItem::create(VariableId::standalone(3), "use_premium");
    let selected = SelectFunction::new(
        3, "selected_cost", selector,
        Linear::constant(12.0), (12.0, 12.0),
        Linear::constant(8.0), (8.0, 8.0),
    ).unwrap();
}
```

:::

## McCormick product envelope

For $x\in[L_x,U_x]$, $z\in[L_z,U_z]$, and a result variable $w$, `McCormickEnvelopeFunction` adds the four inequalities

$$
\begin{aligned}
w&\ge L_xz+L_zx-L_xL_z, & w&\ge U_xz+U_zx-U_xU_z,\\
w&\le U_xz+L_zx-U_xL_z, & w&\le L_xz+U_zx-L_xU_z.
\end{aligned}
$$

For general continuous factors, these inequalities form the convex hull relaxation over the bounded box; they do not enforce $w=xz$ at every interior point. The supplied finite bounds are essential and must cover the factors' full domains. In Kotlin they are required `ConditionBounds`; in Rust they are explicit bound pairs. The ordinary result evaluation reads the model's $w$ value; the separate product evaluator computes the true arithmetic product.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.McCormickEnvelopeFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x").also { it.range.geq(Flt64.zero); it.range.leq(Flt64(2.0)) }
    val z = RealVar("z").also { it.range.geq(Flt64.zero); it.range.leq(Flt64(3.0)) }
    val xExpression = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
    val zExpression = LinearPolynomial(listOf(LinearMonomial(Flt64.one, z)), Flt64.zero)
    val envelope = McCormickEnvelopeFunction(
        left = xExpression,
        right = zExpression,
        leftBounds = ConditionBounds(Flt64.zero, Flt64(2.0)),
        rightBounds = ConditionBounds(Flt64.zero, Flt64(3.0)),
        converter = IntoValue.Identity,
        name = "xz_envelope"
    )
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::McCormickEnvelopeFunction;

fn main() {
    let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
    let z = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
    let envelope = McCormickEnvelopeFunction::new(
        4, "xz_envelope", x, z, (0.0, 2.0), (0.0, 3.0)
    ).unwrap();
}
```

:::

## Integer-indexed lookup

`ElementFunction` (alias `LookupFunction`) selects the table expression indexed by $i$:

$$
y=v_{i-\ell},\qquad \ell\le i<\ell+n,
$$

where $\ell$ is `lowerIndex` and the table has $n$ entries. The index must be a bounded integer variable, and every table expression needs finite bounds for the one-hot and masking formulation. Kotlin may infer entry bounds or receive them with `valueBounds`; Rust uses `ElementValue(expression, lower_bound, upper_bound)` for each entry. An index outside the table is infeasible in the model and evaluates to `null`/`None`. Negative first indices are supported. The index always determines the table position, even when multiple entries have equal values.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ElementFunction
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64

fun main() {
    val grade = IntVar("grade")
    grade.range.geq(Int64(-1))
    grade.range.leq(Int64(1))
    val lookup = ElementFunction.fromConstants(
        index = grade,
        values = listOf(Flt64(10.0), Flt64(14.0), Flt64(19.0)),
        lowerIndex = -1,
        converter = IntoValue.Identity,
        name = "grade_cost"
    )
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{ElementFunction, ElementValue};
use ospf_rust_core::variable::{IntegerVariableItem, VariableId, VariableRange};

fn main() {
    let grade = IntegerVariableItem::with_range(
        VariableId::standalone(5), "grade", VariableRange::bounded(-1.0, 1.0)
    );
    let table = [10.0, 14.0, 19.0].into_iter()
        .map(|value| ElementValue::new(Linear::constant(value), value, value))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let lookup = ElementFunction::new(5, "grade_cost", grade, table, -1).unwrap();
}
```

:::

## Related functions

- [`ProductFunction`](../quadratic-functional/product) expands two linear expressions into a quadratic polynomial. It does not create a linearized product variable.
- [`ArgMin`, `ArgMax`, `KthLargest`, and `TopKSum`](./order-statistics) provide exact selection and ranking operations.
- [`McCormickEnvelopeFunction`](../quadratic-functional/linear-compositions) can be used in a quadratic model, but remains a relaxation of a continuous product.
