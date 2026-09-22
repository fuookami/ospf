# Gurobi Solver Notes

:us: English | :cn: [简体中文](README_ch.md)

## Prerequisites

`ospf-rust-core` Gurobi integration requires:

1. Gurobi installed on the machine.
2. A valid Gurobi license.
3. One of these cargo features enabled:
- `gurobi10`
- `gurobi11`
- `gurobi12`

## Key Capabilities

1. LP/MIP/QP/MIQP solving.
2. Stage callback and telemetry callback.
3. Native callback and native observers.
4. Numeric diagnostics and numeric profile recommendation.

## CP Boundary

Gurobi does not provide the CP model component. Gantt task-compilation code builds an immutable
CP snapshot first; this backend consumes only the exact `ExactLowering` MIP facade and returns the
unified `SolveReport`. Native optional/variable-duration intervals and unverified global-constraint
decompositions are not advertised as native CP support.

## Native Callback Semantics

1. `add_native_callback` uses override semantics (latest wins).
2. `add_native_observer` uses append/multicast semantics.
3. Native observer can return `Terminate` to request solve termination.

## Native Function-Symbol Lowering

Function symbols expand into generic constraints by default (the `Eager` policy). When
`SolverConfig::resolved_function_expansion_policy` (or `MetaModel::set_function_expansion_policy`)
selects `DeferredNativeFirst`, a symbol may instead keep a solver-neutral
`DeferredFunctionStructure` and let this backend write it as a Gurobi general constraint.

`GurobiSolver::solve_linear_with_native_lowering(mechanism, options)` is the entry point. It performs
the two-phase flow that makes the native write possible:

1. create the SDK columns from `MechanismModel::linear_column_view()` — the same order and bound rules
   as the linear triad produced afterwards, so phase one's columns *are* the final columns;
2. hand the container and the writer registry to `MechanismModel::lower_deferred_functions`: every
   structure a writer claims becomes a native constraint and is dropped from the pending list, while
   the rest are materialized as the generic fallback in one step (a writer error propagates so the
   caller can fall back for the whole model);
3. convert to the linear triad and load the rows and the objective into the **same** SDK model, then
   solve.

Writers registered today:

| writer | schema | accepted structures | native call |
| --- | --- | --- | --- |
| `gurobi_abs` | `functions-abs-1` | `AbsStructure` whose input is a single unit-coefficient monomial with a zero constant and an argument column distinct from the result | `add_genconstr_abs` |
| `gurobi_max` | `functions-max-1` | `MaxStructure` whose candidates are all single unit-coefficient monomials sharing one constant, with operand columns distinct from the result | `add_genconstr_max` |
| `gurobi_min` | `functions-min-1` | `MinStructure` under the same admission rules as `gurobi_max` | `add_genconstr_min` |
| `gurobi_pwl` | `functions-pwl-1` | `SinStructure`, `CosStructure` and `SigmoidStructure` whose input is a single unit-coefficient monomial with a zero constant and whose point table has at least two finite, strictly increasing points | `add_genconstr_pwl` |
| `gurobi_indicator` | `functions-indicator-1` | `InequalityStructure` of the non-strict and strict kinds (`Equal`/`NotEqual` are rejected: the false side is a disjunction that two indicators cannot express) | `add_genconstr_indicator` |
| `gurobi_if_in` | `functions-if-in-1` | `IfInStructure` whose value set is non-empty, whose input is a single unit-coefficient monomial with a zero constant, and whose condition boxes are finite | `add_genconstr_indicator` (four per candidate value) + `add_genconstr_or` (aggregation) |
| `gurobi_and` / `gurobi_or` | `functions-and-1` / `functions-or-1` | `AndStructure` / `OrStructure`, which the model layer only exposes when every operand is a direct binary variable | `add_genconstr_and` / `add_genconstr_or` |
| `gurobi_binaryzation` | `functions-binaryzation-1` | `BinaryzationStructure` whose input is a single unit-coefficient monomial with a zero constant and whose input box is finite (both the threshold and the big-M variants; a separate writer from `gurobi_indicator` because the eager shape differs) | `add_genconstr_indicator` (core rows) plus the big-M redundancy proof |
| `gurobi_imply` | `functions-imply-1` | `ImplyStructure`, which carries two inner sub-indicators (premise and consequence). Because the eager coupling row `r ≥ c` is unconditional, the reconstruction from three indicator constraints is equivalent only in the **binary domain**, so the writer verifies on the SDK (`grb::VarType`) that the result and both sub-indicator columns are binary | `add_genconstr_indicator` (two per sub-indicator plus three coupling rows) plus the big-M redundancy proof for each sub-indicator |

Every writer applies the same gates before writing:

* a result column that is already fixed (`FunctionUsageSummary::forbids_native_write`) is rejected, since
  substituting it would change what the native structure means;
* helpers referenced outside the function's own rows (`helpers_are_exclusive` returning `false`) are
  rejected: the native relation constrains the result column only, so a selector or branch column would
  silently lose its meaning if the rest of the model refers to it;
* structures the writer cannot express exactly — a general affine input that would need a bridge column,
  candidates with distinct constants, operand columns that coincide with the result — are rejected too.

`gurobi_pwl` adds one more gate that only applies to piecewise-linear writes, the **range proof**: eager
expansion builds `x` as a convex combination of the breakpoints (`x = Σ λᵢ xᵢ`), so it pins the input
inside `[x₀, xₙ]`, while a native PWL constraint extrapolates outside it. The writer therefore reads the
input column's *actual* bounds from the SDK model (`Model::get_obj_attr(attr::LB/UB, &var)` — after
`Model::update()`, because freshly created columns are pending objects in Gurobi's lazy-update mode) and
writes natively only when `lb ≥ x₀ - 1e-9` and `ub ≤ xₙ + 1e-9`; otherwise it falls back. Reading the
bounds is the point: the declared token bounds are not enough, because the solve model is the authority.

`gurobi_indicator` adds the second domain-specific gate, a **big-M redundancy proof**. Eager expansion of a
relation indicator writes two rows per indicator value: a core row (which the native indicator reproduces
exactly, using the same `INDICATOR_TOLERANCE`) and a row relaxed by big-M. Writing only the core rows would
**enlarge the feasible region**, so the writer proves that the relaxed rows hold on the condition's actual
box before writing: it derives `s_min`/`s_max` of `s = Σ cₖxₖ + constant` from the SDK bounds
(`get_obj_attr(attr::LB/UB)`) — comparing against `grb::INFINITY` explicitly, because Gurobi represents an
infinite bound as the *finite* value `±1e100` — and falls back when the box is unbounded or unreadable. The
tolerance is the named constant `GUROBI_INDICATOR_BIG_M_TOLERANCE`. Rows that only the big-M encodes are
never dropped silently.

A rejected structure is not an error: it falls back to the generic expansion in the same lowering pass.
A *write* failure (rather than a rejection) triggers a **whole-model fallback**: the SDK model built so
far may already carry some native constraints that cannot be rolled back one by one, so the solver
discards it and rebuilds from the same mechanism model along the generic path — the solve still
succeeds and the report records every structure as failed-over.

Implementation notes:

* `GurobiNativeContainer` **owns** the Gurobi model: the writer registry requires its container type to
  be `'static`, which a borrowing container with a lifetime parameter cannot satisfy. `into_parts()`
  hands the model back for row loading.
* The native path never adds or removes a public column, and every structure a writer does not claim
  still receives its fallback rows, so a partially native model never leaves a relation unwritten.
* The argument of a native write is the **column the input monomial points at**, not the symbol's
  branch-selector column; confusing the two writes a semantically wrong relation, which only a
  real-solve test catches. `plan_abs_native` is deliberately SDK-free so admission stays testable
  without a licence.

Acceptance for this path is a real solve (so the licence above is required):

```bash
cargo test -p ospf-rust-core --features gurobi10 --test gurobi_native_function_lowering
cargo test -p ospf-rust-core --features gurobi10 --test native_release_matrix -- --include-ignored
```

The first command asserts that the ABS structure is written natively exactly once, that no fallback was
materialized for it, that the real solution satisfies `y = |x|` and the model's own constraints, and
that the eager and native paths agree on feasibility. See the
[function symbol support matrix](../../../../README.md#function-symbol-support-matrix) in the core
README for which symbols expose a structure today.

## Minimal Validation Commands

From workspace root:

```bash
cargo test -p ospf-rust-core gurobi_native_observer_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_telemetry_callback_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_stage_callback_integration --features gurobi10 -- --nocapture
```

This workspace is validated against the local Gurobi 10 environment. If your environment is pinned
to Gurobi 11/12, replace `gurobi10` with `gurobi11`/`gurobi12`.

The shared native contract also checks report identity, best bound, gap, solution
values, and constraint residuals. A missing license is a `LICENSE` error (including
native code `10009`); it must not be reported as a successful environment skip.
See the [native validation matrix](../../../../README.md#native-validation-matrix) in the core README.
