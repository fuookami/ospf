# Conditional Function Contract

English | [Simplified Chinese](conditional-function-contract_ch.md)

## Stable Names

- `IfFunction` remains the legacy ternary expression `condition != 0 ? then : else`.
- `IfElseFunction` is the preferred ternary function when the condition is an explicit binary variable.
- `ConditionalIndicatorFunction` is the registerable relation indicator. It requires a `ConditionRelation`, a positive `strict_boundary`, and finite `ConditionBounds`.
- `ConditionalIfFunction` is the non-registering relation descriptor used by pure classification APIs.
- `semantic::if_` and `semantic::if_named` construct the explicit range-driven indicator. `if_legacy` preserves the previous threshold binaryization behavior.
- `IfInFunction` retains discrete-set membership semantics. `IfInRangeFunction` is the distinct closed-range descriptor, and `RegisterableIfInRangeFunction` registers its two indicators and AND result.
- `IfThenConstraintFunction` remains the legacy inequality-implication alias. `ConditionalThenFunction` registers a result that is zero on the false branch and equals `then_poly` on the true branch; non-constant `then_poly` requires explicit finite bounds.
- `ConditionalImplyFunction` is the range-driven, registerable implication. It gates consequent rows when the premise is false. `imply_constraint` remains the explicit legacy Big-M entry.
- `SigmoidStepFunction` is the registerable relation-step form; `SigmoidFunction` remains the continuous PWL form.

## Relation Semantics

For `d = lhs - rhs`, an indicator is true or false only in the following branch regions. Values in the gap are `Undefined`.

| Relation | True | False |
| --- | --- | --- |
| `Greater` | `d >= g` | `d <= 0` |
| `GreaterEqual` | `d >= 0` | `d <= -g` |
| `Less` | `d <= -g` | `d >= 0` |
| `LessEqual` | `d <= 0` | `d >= g` |

`g` is the positive business `strict_boundary`; it is not a solver feasibility tolerance. Equality relations are deliberately rejected by generic unary indicators.

## Registration Rules

Registerable relation indicators never infer a default Big-M. Callers must provide finite, ordered bounds that cover the condition polynomial. Bounds entirely inside the Undefined gap are rejected. A range contained in one branch fixes both helper variables; a range crossing the branches emits two range-driven rows and links the stable result variable to the internal indicator.

`evaluate` returning `None` can mean an `Undefined` condition or unavailable input. Use `classify` where that distinction matters. A false implication premise short-circuits to true, and the range-driven implication relaxes the consequent rows in that case.

## Discrete Conditions and Atomicity

A non-unit `delta` cannot pass merely because a caller declares a step. `ConditionalIndicatorFunction::from_discrete_condition` derives `DiscreteConditionLatticeProof` from the linear polynomial and the registered token metadata before it creates a registerable indicator: every participating variable has an integer type, every coefficient is a finite integer, the step is the gcd of absolute coefficients, and the constant term is normalized to a remainder modulo that step. Relation-aware validation checks `delta`, `strict_boundary`, and the nearest lattice distances on both sides of zero.

`MetaModel::add_symbols` and `register_combination` commit in one transaction. On failure, the transaction restores tokens, symbols, and constraints; `BasicModel` invalidates the old cache contexts and rebuilds/rebinds them from the restored token table, so stale cache references are not retained. `MutableTokenList::try_add_tokens` and `MutableTokenTable::register_batch` no longer have per-item default implementations. The repository's `VecTokenList`/`VecTokenTable` and `ConcurrentTokenList`/`ConcurrentTokenTable` write-guard paths provide atomic implementations; third-party trait implementers must add an explicit atomic transaction implementation, which is a source migration impact. `try_add_tokens` prevalidates variable IDs, names, and assigned solver indices; on failure it writes no token and preserves existing token result caches. The legacy unit-returning `add_tokens` entry is retained for source compatibility; new code must use `try_add_tokens` to observe registration errors.

`ConcurrentTokenList` and `ConcurrentTokenTable` expose a real trait view through `read()` guards. For a `ConcurrentTokenTable`, declare and hold the guard while creating the view: `let guard = concurrent.read(); let view: &dyn TokenTable<_> = &*guard;` The analogous `ConcurrentTokenList` view uses `let guard = concurrent.read(); let view: &dyn TokenList<_> = &*guard;`. They do not directly implement traits that return borrowed token references, because doing so would release the lock before the reference is used.

Legacy Big-M helpers reject non-finite, zero, and negative values; small positive values are raised to the minimum stable value. `IfInRangeFunction::new` accepts only a single shared variable with `x - lower >= 0` and `upper - x >= 0`, and validates both finite side bounds and endpoint order.
