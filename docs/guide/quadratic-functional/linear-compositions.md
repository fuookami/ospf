# Linear Functions in Quadratic Models

Linear and quadratic model functions serve different purposes. A linear-input function can often be reused in a quadratic model because its generated rows remain linear. Functions that accept a quadratic polynomial compose the same operation over a nonlinear input and may add quadratic equalities.

## Reusing linear-input functions

Linear expressions are valid components of quadratic objectives and constraints. Existing linear functions such as `MinFunction`, `MaxFunction`, `AbsFunction`, `MaskingFunction`, `IfFunction`, and the exact composite functions can be reused when their inputs are linear and their generated formulation is linear. Their ordinary MILP exactness and finite-bound requirements continue to apply. This includes [cardinality, positive-part, clamp, complementarity, and distance functions](../linear-functional/composite-cardinality), [order statistics](../linear-functional/order-statistics), and [CVaR](../linear-functional/risk-global-constraints). In Rust, build expressions that use helper variables from the registered solver-index map exposed by the owning symbol.

`ProductFunction` multiplies two linear expressions and returns a quadratic expression. It does not introduce a result variable or a linearization. A solver that accepts quadratic expressions is required.

`QuadraticLinearFunction` binds a quadratic expression to a signed real helper variable through an equality, so the value can be reused in linear-input function interfaces. That equality is quadratic; the resulting model is MIQCP and may be nonconvex.

## Composing functions over quadratic inputs

Quadratic counterparts such as `QuadraticAbsFunction`, `QuadraticMaxFunction`, `QuadraticMinMaxFunction`, `QuadraticMaskingFunction`, and `QuadraticIfFunction` apply familiar function semantics to quadratic input polynomials. Each genuinely quadratic input is represented by a signed bridge variable and an exact quadratic equality. Affine inputs need no bridge. The function's other linearization rows are then formed over the bridge value.

This avoids expanding products of quadratic expressions into cubic or quartic terms. It does not make the model linear: expect MIQCP constraints and use a solver that supports the required quadratic constraints. Convexity and performance depend on the whole model.

## Model guarantees

| Construction | Guarantee | Solver/model class |
|---|---|---|
| Bounded integer product or exact selector | Exact for all objective directions | MILP |
| McCormick envelope for continuous factors | Convex-hull relaxation over the declared box | LP/MILP relaxation |
| Nonlinear piecewise function | Linear interpolation of supplied points; no automatic error bound | MILP with segment selectors |
| CVaR exact candidate enumeration | Exact for all objective directions | MILP |
| CVaR epigraph | Tight at an optimum when minimized; a result upper bound projects exactly to `CVaR <= limit` but does not force equality | LP/MILP epigraph |
| Quadratic bridge equality | Exact equality to quadratic input | MIQCP, possibly nonconvex |

The correct choice depends on the semantics needed. A relaxation may be useful in a bound or decomposition but should not be described as an exact product. A PWL model approximates the source curve. A CVaR epigraph becomes tight when its expression is minimized; a result upper bound preserves the exact risk-feasibility test without fixing the auxiliary expression to CVaR. A quadratic bridge preserves an equality but changes the solver class.

See [product expressions](./product), [quadratic input functions](./quadratic-abs), and the linear-model pages for [composite, cardinality, and distance functions](../linear-functional/composite-cardinality), [order statistics](../linear-functional/order-statistics), [slack ranges](../linear-functional/slack-range), [products and selection](../linear-functional/products-selection), [nonlinear approximations and tariffs](../linear-functional/nonlinear-tariffs), and [risk measures and global constraints](../linear-functional/risk-global-constraints).
