# Logical XOR

## Contract

`XorFunction<V>` counts the nonzero input polynomials. Its evaluator returns one exactly when one input is nonzero, and zero for every other count. Therefore this is an exact-one function; for two binary inputs it agrees with the usual XOR, but for more inputs it is not parity XOR.

The solver registration currently does not fully enforce that evaluator contract. The executable inequalities are documented below so that the distinction is explicit.

## Definition and truth table

For input values `p_i`, the evaluator computes:

$$
a_i =
\begin{cases}
1, & p_i\ne0 \\
0, & p_i=0
\end{cases}
\qquad
y_{\mathrm{eval}} =
\begin{cases}
1, & \sum_i a_i=1 \\
0, & \sum_i a_i\ne1
\end{cases}
$$

For three inputs, the current evaluator and the solver result permitted by the final constraints are:

| Number `s=\sum_i a_i` | Evaluator | Parity XOR | Solver result |
| ---: | ---: | ---: | --- |
| 0 | 0 | 0 | 0 |
| 1 | 1 | 1 | 0 or 1 |
| 2 | 0 | 0 | 0 |
| 3 | 0 | 1 | infeasible |

The parity column is included only to show why the current function is not parity XOR. The source comments describe a parity encoding, but the executable evaluator and constraints do not implement that definition.

## Boundary, tolerance, and Undefined

`evaluate(values)` compares each evaluated value with `converter.zero` exactly; it does not apply `tolerance` or `strictBoundary`. A missing input or failed polynomial evaluation returns `null`.

Constraint registration creates a nonzero indicator for every input. With tolerance `t` and strict boundary `g`, the current indicator constraints model the zero band `|p_i|\le t` when the indicator is zero, and an outside value `p_i\le-g` or `p_i\ge g` when it is one. Values in the open gaps `(-g,-t)` and `(t,g)` have no valid indicator assignment when `t<g`.

The defaults are `NONZERO_TOLERANCE = 1e-10` for `tolerance` and `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52` for `strictBoundary`. Thus a tiny nonzero value can count as nonzero in `evaluate` while having no solver assignment.

## Current API

### Kotlin

```kotlin
XorFunction(
    polynomials: List<LinearPolynomial<V>>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "xor",
    displayName: String? = null
)
```

The companion `invoke` overload accepts `polynomials`, `converter`, `bigM`, `name`, and `displayName`, but does not expose `tolerance` or `strictBoundary`. Use the constructor when those boundaries must be set.

### Rust

Rust exposes [`XorFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs):

```rust
XorFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> XorFunction<V>
```

The constructor asserts at least two inputs and creates `result_variable()`, `indicator_variables()`, and `side_variables()`. There are no per-instance `bigM`, tolerance, or strict-boundary arguments; the shared indicator policy is used. Rust's current evaluator returns `1` when the inputs contain both a zero and a nonzero value (the usual two-input XOR case), so for more than two inputs it is not Kotlin's exact-one evaluator and is not parity XOR.

## Auxiliary variables and registration model

For `name`, the implementation creates:

- result binary variable `name_xor`;
- one nonzero indicator `name_xor_nz{i}` per input;
- one binary side variable `name_xor_side{i}` per input.

All of them are returned by `helperVariables`; `resultPolynomial` is the unit-coefficient polynomial of `name_xor`. Each input indicator is registered through the shared nonzero-indicator helper, using the explicit `bigM` or that polynomial's default Big-M.

After the indicator constraints, the current final XOR inequalities are, with `s=\sum_i a_i` and binary result `y`:

$$
0\le s-y,
\qquad
s-y\le n-1,
\qquad
s+(n-1)y\le n.
$$

They imply `y=1\Rightarrow s=1`, but they allow `y=0` whenever `s\le n-1`; in particular `s=1` does not force `y=1`, and `s=n` is infeasible. This is a high-risk solver/evaluator mismatch, not parity behavior.

## `evaluate()` versus the solver model

The evaluator implements exact-one counting and returns `null` only for missing or failed inputs. The solver additionally imposes tolerance bands and the final inequalities above. Consequently, an assignment can evaluate to one but permit solver result zero, and an assignment with every input outside the nonzero band can make the solver model infeasible. Do not describe the current solver encoding as parity XOR or as a complete exact-one equivalence.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.XorFunction
import fuookami.ospf.kotlin.core.variable.BinVar

class XorTest {
    @Test
    fun xorEvaluate() {
        val x = BinVar("x")
        val y = BinVar("y")
        val px = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
        val py = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
        val xor = XorFunction(listOf(px, py), converter = IntoValue.Identity, name = "xor")

        val r10 = xor.evaluate(mapOf(x to Flt64.one, y to Flt64.zero))
        val r11 = xor.evaluate(mapOf(x to Flt64.one, y to Flt64.one))
        assertTrue(r10 != null && (r10 eq Flt64.one))
        assertTrue(r11 != null && (r11 eq Flt64.zero))
    }
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::XorFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let xor = XorFunction::new(1, "xor", vec![x, y]);
assert_eq!(xor.indicator_variables().len(), 2);
let _result = xor.result_variable();
```

:::

Rust source and parity coverage: [`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Source and core tests

- [Implementation: `And.kt` (contains `XorFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `XorTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/XorTest.kt)

## Related pages

- [AND](/guide/linear-functional/and)
- [OR](/guide/linear-functional/or)
- [NOT](/guide/linear-functional/not)
- [One-of constraint](/guide/linear-functional/one-of)
- [Binarization](/guide/linear-functional/bin)
