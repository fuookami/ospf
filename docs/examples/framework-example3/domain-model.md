# Framework example 3: csp1d domain model

[中文](../../zh-cn/examples/framework-example3/domain-model.md)

This page makes the csp1d context boundaries explicit. It complements the
[example overview](../framework-example3) and follows the domain-model
template. The current application uses generated plan columns; it does not
construct a fixed all-pattern RMP by hand.

## 1. Overview and dependent contexts

The bounded contexts are:

| Context | Responsibility | Dependency |
| --- | --- | --- |
| material | product widths, material width range, and demand | none |
| produce | current plan columns, usage variables, demand contribution, material/machine expressions | material and generated plans |
| cutting-plan-generation | initial and reduced-cost plan generation | material and produce duals |
| length-assignment | optional dynamic-product length restrictions | material and produce |
| wasting-minimization | waste/rest/material-cost and overproduction terms | produce and configuration |
| application | column-generation orchestration, solver, trace, and output | all above |

## 2. Concepts, predicates, and sets

Let `D` be products, `M` materials, `J_t` the columns available at iteration
`t`, and `K` machines. Product `d` has width `width_d` (Meter) and demand
`demand_d` (roll count). A generated cutting plan `j` contains integer
coefficients `a_{dj}` (pieces of product `d`) and a material-usage amount.

Useful predicates are `dynamic(d)` (the product has an optional length rule),
`active(j)` (the plan is in the current pool), and `feasible(j,m)` (the plan
fits material `m` and the configured knife/length rules). Thus

$$
D^{dynamic}=\{d\in D\mid dynamic(d)\},\qquad
J_t^{active}=\{j\mid j\text{ has been inserted before iteration }t\}.
$$

The application fixture uses one material of width 1000 Meter and four
products `(450,97)`, `(360,610)`, `(310,395)`, and `(140,211)` where each pair
is `(width, demand)`.

## 3. Variables and intermediates

The restricted master usage variable is:

$$
x_j\in\mathbb Z_{\ge 0}\quad(j\in J_t^{active}),
$$

where `x_j` is the number of times plan `j` is used. Product yield and the
core resource expressions are:

$$
q_d=\sum_{j\in J_t^{active}}a_{dj}x_j,
\qquad
u_m=\sum_{j\in J_t^{active}}usage_{mj}x_j,
\qquad
b_k=\sum_{j\in J_t^{active}}batch_{kj}x_j.
$$

When yield slack is configured, the produce context additionally uses
non-negative `under_d` and `over_d` with

$$
q_d-over_d+under_d=demand_d\quad(d\in D).
$$

The length-assignment and wasting contexts contribute optional expressions
only when their predicates/configuration are present.

## 4. Assertions and constraints

For the ordinary no-slack path, every demand must be met:

$$
s.t.\quad q_d\ge demand_d\qquad(d\in D).
$$

Material, machine, and configured length limits are added by their respective
pipelines:

$$
s.t.\quad u_m\le capacity_m\quad(m\in M),
\qquad
s.t.\quad b_k\le capacity_k\quad(k\in K),
$$

with the exact coefficients supplied by generated plans and machine/material
data. A dynamic-product length assertion is active only for
`d∈D^{dynamic}`. `J_t` changes after an `addColumns` call; therefore the
quantifier is iteration-local and must not be replaced by a fixed global
pattern set.

## 5. Objective function

The base active objective minimizes plan usage:

$$
\min\sum_{j\in J_t^{active}}x_j.
$$

Configured wasting-minimization terms can add waste, rest/material-cost, or
overproduction penalties. In the small example invocation these optional
configuration overrides are absent, so they are not asserted as active
objective terms.

## 6. Column-generation algorithm boundary

At iteration `t`, the RMP is solved over `J_t^{active}`. Dual prices from its
demand/resource constraints are passed to the pricing generator, which
searches feasible plans and returns negative-reduced-cost columns. The
application inserts deduplicated columns to obtain `J_{t+1}^{active}` and
repeats until pricing completion or a configured limit. `RMP` and `pricing`
are algorithm roles; neither is a second, fixed variable family in `Main.kt`.

## 7. Ubiquitous language and design decisions

| Term | Symbol | Meaning |
| --- | --- | --- |
| product | `d` | requested cut width and demand |
| plan/column | `j` | one feasible cutting pattern |
| yield | `q_d` | product pieces produced by selected plans |
| usage | `x_j` | integer number of uses of a plan |
| RMP | `J_t` | current restricted column pool |
| pricing | — | search for improving feasible plans |

| Decision | Alternative | Rationale |
| --- | --- | --- |
| generate columns incrementally | fixed all-pattern RMP | keeps the master bounded and matches `addColumns` |
| represent demand by roll count | treat demand as length | source `legacyRoll` uses count semantics |
| keep optional penalties configuration-gated | always add penalties | avoids claiming inactive objective terms |

## 8. Source and change boundary

The implementation evidence is the csp1d `Csp1dProblem`, produce aggregate,
`Csp1dColumnGeneration`, and `Main.kt` linked from the parent page. This page
records the model contract; source pipelines and tests decide which optional
constraint is active for a particular configuration.
