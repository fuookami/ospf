# Composite Functions and Cardinality

Several useful linearizations are compositions of familiar building blocks. OSPF provides direct function symbols for common patterns and pure constraint helpers when a result indicator is unnecessary.

## Positive part, clamp, and dead zone

For a bounded linear expression $x$, the positive part is

$$
\max(x,0),
$$

`PositivePartFunction` returns this value exactly. `ClampFunction` computes $\min(\max(x,\ell),u)$ for a finite ordered interval $[\ell,u]$. `DeadZoneFunction` computes $\max(|x|-\delta,0)$ for a nonnegative width $\delta$. Their MILP formulations use finite input bounds and are exact for every objective direction.

```kotlin
val positive = PositivePartFunction(input, converter, name = "overage")
val clipped = ClampFunction(input, lower, upper, converter, name = "clipped")
val deadZone = DeadZoneFunction(input, delta, converter, name = "tolerance")
```

```rust
let positive = PositivePartFunction::new(1, "overage", input.clone())?;
let clipped = ClampFunction::new(2, "clipped", input.clone(), 0.0, 100.0)?;
let dead_zone = DeadZoneFunction::new(3, "tolerance", input, 2.0)?;
```

Kotlin constructors return `Ret<...>` for validation; Rust constructors return `Result<...>`. Both versions require finite input ranges for exact linearizations. Kotlin may infer them from variable ranges or accept explicit bounds on APIs that expose them. Rust validates declared variable token domains when it builds the mechanism.

## Complementarity

`ComplementarityFunction` enforces that two nonnegative bounded expressions cannot both be positive:

$$
x\ge 0,\quad y\ge 0,\quad xy=0.
$$

The implementation uses a binary selector and finite upper bounds to form an exact MILP disjunction. The function is constraint-only: its semantic evaluation returns 1 when both inputs are positive (a violation) and 0 when the pair satisfies the rule. The internal selector is a witness for either feasible branch; when both expressions are zero, either branch is valid.

```kotlin
val exclusive = ComplementarityFunction(
    x = x,
    y = y,
    converter = converter,
    xBounds = ConditionBounds(Flt64.zero, Flt64(10.0)),
    yBounds = ConditionBounds(Flt64.zero, Flt64(10.0)),
    name = "exclusive"
)
```

```rust
let exclusive = ComplementarityFunction::new(
    4, "exclusive", x, y, (0.0, 10.0), (0.0, 10.0)
)?;
```

Both bound intervals must be finite, ordered, nonnegative, and cover every feasible input. Kotlin explicit bounds are enforced as domain rows, so they can bound otherwise unbounded expressions; Rust requires the registered token domains to fit within the declared bounds.

## Distances and range

For equal-length vectors, `L1DistanceFunction` computes $\sum_i |x_i-y_i|$ and `LInfinityDistanceFunction` computes $\max_i |x_i-y_i|$. `RangeFunction` computes $\max_i x_i-\min_i x_i$ for a non-empty list. These functions compose exact absolute-value and extremum selectors, so their results stay correct when used in constraints or in either objective direction. All inputs need finite ranges; the L-infinity and range input collections must be non-empty.

```kotlin
val l1 = L1DistanceFunction(left, right, converter, name = "l1")
val linf = LInfinityDistanceFunction(left, right, converter, name = "linf")
val spread = RangeFunction(inputs, converter, name = "spread")
```

```rust
let l1 = L1DistanceFunction::new(5, "l1", left.clone(), right.clone())?;
let linf = LInfinityDistanceFunction::new(6, "linf", left, right)?;
let spread = RangeFunction::new(7, "spread", values)?;
```

## At-most and exactly counts

When $b_i$ are binary variables, the count itself is the linear expression $\sum_i b_i$. If only feasibility is needed, the shortest formulations are the ordinary rows

$$
\sum_i b_i\le k,\qquad \sum_i b_i=k.
$$

Kotlin exposes `atMostConstraints(...)` and `exactlyConstraints(...)` for these pure rows. `AtMostFunction` and `ExactlyFunction` instead return an exact 0/1 indicator for whether the count satisfies the requested limit or total. The Rust counterparts provide the same indicator form; a pure count condition can also be written directly as a linear constraint. These forms require binary inputs and integer counts in $[0,n]$.

```kotlin
val atMost = AtMostFunction(indicators, limit = 2, converter = converter)
val exactly = ExactlyFunction(indicators, amount = 1, converter = converter)
val rows = atMostConstraints(indicators, limit = 2, converter = converter)
```

```rust
use std::collections::HashMap;
use std::sync::Arc;

use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::function::{
    at_most_constraints, exactly_constraints, AtMostFunction, ExactlyFunction,
};
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

let mut model = BasicModel::new("cardinality");
let indicators = vec![
    BinaryVariableItem::create(VariableId::standalone(1), "b0"),
    BinaryVariableItem::create(VariableId::standalone(2), "b1"),
    BinaryVariableItem::create(VariableId::standalone(3), "b2"),
];
for indicator in &indicators {
    model.register_variable(indicator.clone())?;
}
let at_most = Arc::new(AtMostFunction::new(8, "at_most", indicators.clone(), 2)?);
let exactly = Arc::new(ExactlyFunction::new(9, "exactly", indicators.clone(), 1)?);
model.add_symbol(at_most)?;
model.add_symbol(exactly)?;
let symbol_to_index = model.tokens().iter()
    .map(|token| (token.id().unique_id() as usize, token.solver_index))
    .collect::<HashMap<_, _>>();
let pure_at_most = at_most_constraints::<f64>(
    &indicators, 2, &symbol_to_index, "pure_at_most"
)?;
let pure_exactly = exactly_constraints::<f64>(
    &indicators, 1, &symbol_to_index, "pure_exactly"
)?;
```

Rust's `at_most_constraints` and `exactly_constraints` take the registered `symbol_to_index` map and return `Result` rows; a missing indicator column is an error. Use each variable ID's `unique_id()` as the map key and its final `solver_index` as the value. For a constraint-only count condition in either language, these rows avoid introducing an auxiliary satisfaction indicator.

## Choosing a formulation

Use a pure linear row when only a condition matters, a function symbol when the numerical result is used elsewhere, and the supplied exact composite when the operation is nonlinear in its inputs. Exactness assumes the finite bounds cover the full feasible domain; a smaller or invalid bound changes the modeled problem.
