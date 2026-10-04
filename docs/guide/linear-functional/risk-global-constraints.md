# Risk Measures and Global Constraints

## Conditional value at risk

For scenario losses $L_i$ with probabilities $p_i$ and confidence level $\alpha$, discrete CVaR is

$$
\operatorname{CVaR}_{\alpha}(L)=\min_{\eta}\left(\eta+\frac{1}{1-\alpha}\sum_i p_i\max(L_i-\eta,0)\right).
$$

`CvarFunction` is an exact discrete formulation. It enumerates each scenario loss as a candidate threshold, uses exact positive-part functions, then selects the smallest candidate. It can be minimized, maximized, or used in constraints without relying on the objective direction. Losses need finite bounds; probabilities must be nonnegative and sum to one, and $0\le\alpha<1$.

`CvarEpigraphFunction` is the smaller LP epigraph formulation. Its reported expression is at least CVaR. Minimizing that expression makes it tight at an optimum. Constraining it above by a limit gives an exact feasibility formulation for `CVaR <= limit`, but does not force the reported expression to equal CVaR at every feasible point. Do not maximize the expression or use it to enforce a lower bound on CVaR; the auxiliaries may leave it above the true value.

```kotlin
val exactRisk = CvarFunction(
    losses = scenarioLosses,
    probabilities = probabilities,
    alpha = Flt64(0.95),
    converter = converter,
    name = "exact_tail_risk"
)
val riskEpigraph = CvarEpigraphFunction(
    losses = scenarioLosses,
    probabilities = probabilities,
    alpha = Flt64(0.95),
    converter = converter,
    name = "tail_risk_epigraph"
)
```

```rust
use std::collections::HashMap;
use std::sync::Arc;

use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{CvarEpigraphFunction, CvarFunction};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableRange};

let mut model = BasicModel::new("scenario_risk");
let loss_a = ContinuousVariableItem::with_range(
    VariableId::standalone(1), "loss_a", VariableRange::bounded(0.0, 10.0)
);
let loss_a_index = model.register_variable(loss_a)?;
let loss_b = ContinuousVariableItem::with_range(
    VariableId::standalone(2), "loss_b", VariableRange::bounded(0.0, 20.0)
);
let loss_b_index = model.register_variable(loss_b)?;
let scenario_losses = vec![
    Linear::new(vec![LinearMonomial::new(1.0, loss_a_index)], 0.0),
    Linear::new(vec![LinearMonomial::new(1.0, loss_b_index)], 0.0),
];
let probabilities = vec![0.5, 0.5];
let exact_risk = Arc::new(CvarFunction::new(
    20, "exact_tail_risk", scenario_losses.clone(), probabilities.clone(), 0.95
)?);
let risk_epigraph = Arc::new(CvarEpigraphFunction::new(
    21, "tail_risk_epigraph", scenario_losses, probabilities, 0.95
)?);
model.add_symbol(exact_risk.clone())?;
model.add_symbol(risk_epigraph.clone())?;
let symbol_to_index = model.tokens().iter()
    .map(|token| (token.id().unique_id() as usize, token.solver_index))
    .collect::<HashMap<_, _>>();
let exact_expression = exact_risk.result_expression(&symbol_to_index)?;
let epigraph_expression = risk_epigraph.result_expression(&symbol_to_index)?;
```

The Rust symbols must be registered before requesting `result_expression`; the method resolves helper IDs through the model's final solver indexes. This also applies to the epigraph, whose result is a combination of its threshold and excess variables rather than a single result column.

## Constraint-programming globals

Global constraints preserve their natural structure in the constraint-programming (CP) model:

| Constraint | Meaning |
|---|---|
| `allDifferent` / `all_different` | Integer expressions take pairwise distinct values |
| `noOverlap` / `no_overlap` | Intervals do not overlap; optional intervals respect their presence state |
| `cumulative` | Total demand of active intervals at any time does not exceed capacity |

Kotlin's `GlobalConstraintFunctions` and Rust's `GlobalConstraintFunctions` construct the same kinds of CP constraint nodes. Prefer a CP solver when its global-constraint support fits the application. Their presence in the function package does not mean that the CP constraint has been turned into a plain linear function.

```kotlin
val allDifferent = GlobalConstraintFunctions.allDifferent(integerExpressions)
val noOverlap = GlobalConstraintFunctions.noOverlap(intervals)
val cumulative = GlobalConstraintFunctions.cumulative(intervals, demands, capacity)
```

```rust
let all_different = GlobalConstraintFunctions::all_different(integer_expressions);
let no_overlap = GlobalConstraintFunctions::no_overlap(interval_ids);
let cumulative = GlobalConstraintFunctions::cumulative(tasks, capacity)?;
```

The Kotlin cumulative AST accepts CP expressions for demands and capacity. Rust's convenience factory uses fixed integer demand per task and an integer capacity. For MIP lowering, cumulative is opt-in and expands over a finite integer time horizon; demands and capacity must be supported constants, and slot/work budgets limit the expansion. `AllDifferent` and `NoOverlap` lowering also require supported finite domains and interval bounds. Use the strict default policy unless the model explicitly enables a supported lowering.

`Cumulative` MIP lowering requires finite integer start, end, and duration bounds. It supports optional intervals through their presence literals, while demands and capacity must be non-negative constants. The default MIP-lowering policy keeps the feature disabled. In Kotlin, opt in through `ConstraintProgrammingLoweringPolicy(allowCumulative = true)` when constructing `ConstraintProgrammingToLinearModelLowerer`; `MipBackedConstraintProgrammingSolver` reports the feature as `Unsupported` by default and `ExactLowering` when enabled. Kotlin currently rejects `Cumulative` nested under implication or reification. Rust uses the equivalent `MipLoweringOptions` fields and `lower_constraint_programming_with_options` also requires explicit opt-in; its truth-valued lowering preserves implication and equivalent reification semantics. The defaults cap each constraint at 4,096 integer time slots and 65,536 interval-slot work units:

```kotlin
import fuookami.ospf.kotlin.core.solver.constraint_programming.MipBackedConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.constraint_programming.lowering.ConstraintProgrammingLoweringPolicy
import fuookami.ospf.kotlin.core.solver.constraint_programming.lowering.ConstraintProgrammingToLinearModelLowerer

val lowerer = ConstraintProgrammingToLinearModelLowerer(
    ConstraintProgrammingLoweringPolicy(
        allowCumulative = true,
        maxCumulativeTimeSlots = 4096,
        maxCumulativeWork = 65_536
    )
)
val mipBackedSolver = MipBackedConstraintProgrammingSolver(linearSolver, lowerer)
```

```rust
use ospf_rust_core::solver::constraint_programming::{
    lower_constraint_programming_with_options, MipLoweringOptions,
};

let options = MipLoweringOptions {
    allow_cumulative: true,
    max_cumulative_time_slots: 4_096,
    max_cumulative_work: 65_536,
    ..MipLoweringOptions::default()
};
let lowered = lower_constraint_programming_with_options(&snapshot, options)?;
```

Large horizons, very wide domains, or richer CP expressions can exceed the budgets or supported subset; use a solver that handles these constraints natively in those cases.
