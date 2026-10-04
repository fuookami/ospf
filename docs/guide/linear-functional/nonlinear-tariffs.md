# Nonlinear Approximations and Tariffs

Smooth nonlinear functions and tiered costs can be represented in a linear model with piecewise-linear (PWL) functions. The model uses interpolation between the supplied breakpoints; it does not automatically calculate or certify an approximation error bound.

## Nonlinear functions

The factories provide PWL approximations for

| Function | Kotlin symbol | Rust symbol | Domain requirement |
|---|---|---|---|
| Exponential $e^x$ | `ExpFunction` | `ExpFunction` | finite interval |
| Natural logarithm $\ln x$ | `LogFunction` | `LogFunction` | strictly positive interval |
| Reciprocal $1/x$ | `ReciprocalFunction` | `ReciprocalFunction` | interval must not contain zero |
| Power $x^p$ | `PowerFunction` | `PowerFunction` | domain must make the configured exponent real and finite |
| Smooth logistic $1/(1+e^{-s(x-m)})$ | `LogisticApproximationFunction` | `LogisticApproximationFunction` | finite interval; $s>0$ |

Choose either an evenly divided interval or explicit, finite, strictly increasing breakpoints. More segments can improve approximation quality, but no universal error guarantee follows from segment count alone. The function's `evaluate` uses the same interpolation as the model; `originalValue`/`original_value` evaluates the mathematical source function separately for comparison.

`SigmoidFunction` has a different meaning: it is a discrete relation indicator with 0/1 values and a boundary gap. Use `LogisticApproximationFunction` when a smooth logistic curve is intended.

```kotlin
val exp = ExpFunction.create(x, Flt64(-2.0), Flt64(2.0), segments = 16, converter = converter)
val log = LogFunction.create(x, Flt64(0.1), Flt64(4.0), segments = 16, converter = converter)
val reciprocal = ReciprocalFunction.create(x, Flt64(1.0), Flt64(5.0), segments = 16, converter = converter)
val power = PowerFunction.create(x, Flt64(0.5), Flt64(0.0), Flt64(4.0), segments = 16, converter = converter)
val logistic = LogisticApproximationFunction.create(x, Flt64(-6.0), Flt64(6.0), segments = 24, converter = converter)
```

```rust
let exp = ExpFunction::with_segments(1, "exp", x.clone(), -2.0, 2.0, 16)?;
let log = LogFunction::with_segments(2, "log", x.clone(), 0.1, 4.0, 16)?;
let reciprocal = ReciprocalFunction::with_segments(3, "reciprocal", x.clone(), 1.0, 5.0, 16)?;
let power = PowerFunction::with_exponent_segments(4, "sqrt", x.clone(), 0.5, 0.0, 4.0, 16)?;
let logistic = LogisticApproximationFunction::with_segments(5, "logistic", x, -6.0, 6.0, 24)?;
```

## Incremental marginal-rate tariff

An incremental tariff charges each tranche at that tranche's marginal rate. With breakpoints $0=b_0<b_1<\cdots<b_n$, the total cost is continuous and equals

$$
C(q)=\sum_{i=0}^{n-1} r_i\,\max(0,\min(q,b_{i+1})-b_i),
$$

within the configured quantity domain. Rates must be finite and nonnegative. Kotlin also accepts a `baseCost` at the first breakpoint; Rust's factory starts at quantity and cost zero.

```kotlin
val incremental = IncrementalTariff.create(
    x = quantity,
    breakpoints = listOf(Flt64.zero, Flt64(10.0), Flt64(25.0)),
    marginalRates = listOf(Flt64(2.0), Flt64(1.5)),
    converter = converter
)
```

```rust
let incremental = TariffFunctions::incremental(
    6, "incremental", quantity,
    vec![0.0, 10.0, 25.0], vec![2.0, 1.5]
)?;
```

## All-units discount

An all-units discount selects one tier's rate and applies that rate to the entire quantity. Rates must be nonnegative and non-increasing. Each internal breakpoint belongs to the tier on its right. A positive `boundaryGap` leaves a small infeasible interval immediately before each internal breakpoint; the gap must be smaller than every tier width. The configured breakpoints define the finite quantity domain.

```kotlin
val discount = AllUnitsDiscountFunction.create(
    x = quantity,
    breakpoints = listOf(Flt64.zero, Flt64(10.0), Flt64(25.0)),
    rates = listOf(Flt64(2.0), Flt64(1.5)),
    boundaryGap = Flt64(0.01),
    converter = converter
)
```

```rust
let discount = TariffFunctions::all_units_discount(
    7, "discount", quantity,
    vec![0.0, 10.0, 25.0], vec![2.0, 1.5], 0.01
)?;
```

The gap is a modeling choice that keeps tier membership unambiguous for a continuous input. It changes the feasible quantity domain and should be chosen to match the application's units and precision.

## Fixed charge

A fixed-charge composition represents a setup cost plus a variable cost:

$$
C=fz+cq,\qquad z\in\{0,1\}.
$$

With activity linking, it also imposes $0\le q\le U z$ and optionally $q\ge Lz$. If `minimumActive` is zero, an active choice may still have zero activity. Choose a positive minimum when activation must imply positive activity. Without an activity expression, the caller controls the binary activation and may pay the fixed charge at zero output.

```kotlin
val fixed = FixedChargeFunction.create(
    activation = open,
    fixedCost = Flt64(25.0),
    activity = quantity,
    unitCost = Flt64(3.0),
    minimumActive = Flt64(1.0),
    maximumActivity = Flt64(100.0),
    converter = converter
)
```

```rust
use std::collections::HashMap;
use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::FixedChargeFunction;
use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange};

let mut model = BasicModel::new("fixed_charge");
let open = BinaryVariableItem::create(VariableId::standalone(1), "open");
model.register_variable(open.clone())?;
let quantity_var = ContinuousVariableItem::with_range(
    VariableId::standalone(2), "quantity", VariableRange::bounded(0.0, 100.0)
);
let quantity_index = model.register_variable(quantity_var)?;
let quantity = Linear::new(vec![LinearMonomial::new(1.0, quantity_index)], 0.0);
let fixed = FixedChargeFunction::new(open, quantity, 3.0, 25.0, 100.0)?
    .with_minimum_active_quantity(1.0)?;
let symbol_to_index = model.tokens().iter()
    .map(|token| (token.id().unique_id() as usize, token.solver_index))
    .collect::<HashMap<_, _>>();
let cost = fixed.cost_expression(&symbol_to_index)?;
let linking_rows = fixed.linking_constraints(&symbol_to_index)?;
```

The Rust API exposes the cost expression and linking rows separately so they can be added to the surrounding model. Pass the map from registered token IDs to final solver indexes; Kotlin returns a function symbol whose result variable and constraints are registered with the model.
