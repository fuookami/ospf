# Framework example 2: context domain-model index

[中文](../../zh-cn/examples/framework-example2/domain-models.md)

This index is the local documentation boundary for the eleven bounded contexts
under `framework_demo/demo2`. Each context has a split model page, and the
coverage matrix below fills the cross-context variable/intermediate/assertion/
constraint/objective contract where a source context is data-only or exposes
only a pipeline. The parent page remains the source-of-truth for mode-specific
registration. A model page documents a context's contract; it does not imply
that the context is registered in every mode.

## 1. Context map and notation

The common dependency direction is:

```text
aircraft → stowage → {mac, airworthiness_security, soft_security,
                      mac_optimization, express_effectiveness,
                      loading_effectiveness, redundancy,
                      recommended_weight_equalization, payload_maximization}
```

`I` is the set of cargo items and `J` the set of aircraft positions. `P` is
the set of flight phases. `w_i` is item weight in the configured aircraft
weight unit, `a_{ij}` is the assignment/adjustment-derived stowage value, and
`L_j` is the load weight at position `j`. All formulas below are schematic
contracts; status predicates and variable fixing in the source determine the
active index sets.

## 2. Context model pages

The long model is split into one page per context so that each page can be
reviewed against its aggregate and pipeline generator.

| Context | Mathematical responsibility | Local model |
| --- | --- | --- |
| aircraft | physical aircraft, deck, position, fuel, ULD, and adjacency data; no solver variables | [aircraft](domain-aircraft/domain-model) |
| stowage | item-position assignment, adjustment, load amount, predicted/recommended weight, and core loading limits | [stowage](domain-stowage/domain-model) |
| mac | torque, CLIM, index, and MAC intermediates derived from aircraft and stowage | [mac](domain-mac/domain-model) |
| airworthiness_security | density, cumulative/zone load, payload, total-weight, envelope, trim, and CLIM limits | [airworthiness security](domain-airworthiness_security/domain-model) |
| soft_security | soft empty-position, door, divide-empty, and ballast preferences | [soft security](domain-soft_security/domain-model) |
| mac_optimization | longitudinal/lateral balance and horizontal-stabilizer limits/objective | [MAC optimization](domain-mac_optimization/domain-model) |
| express_effectiveness | must-ship and item-priority ordering | [express effectiveness](domain-express_effectiveness/domain-model) |
| loading_effectiveness | source/destination adjacency, loading order, reweigh, trailer, and sequence policies | [loading effectiveness](domain-loading_effectiveness/domain-model) |
| redundancy | redundancy and experimental longitudinal-balance limits | [redundancy](domain-redundancy/domain-model) |
| recommended_weight_equalization | item order, priority appointments, and recommended-weight deviation | [recommended weight equalization](domain-recommended_weight_equalization/domain-model) |
| payload_maximization | maximum payload limit and payload objective | [payload maximization](domain-payload_maximization/domain-model) |

Chinese mirrors are linked from each page and are also available under
`docs/zh-cn/examples/framework-example2/`.

### 2.1 Template coverage matrix

The matrix makes the mathematical role of every context reviewable without
forcing all eleven models into one page. “Reuse” means that the context consumes
the stowage/MAC symbols; it does not create a duplicate solver variable.

| Context | Variables | Intermediate contract | Assertion / active constraint boundary | Objective |
| --- | --- | --- | --- | --- |
| aircraft | none (configuration data) | `neighbor(j,j')`, loading order, and phase/fuel lookup | unique identifiers, valid coordinates and unit-compatible aircraft data; no solver rows | none |
| stowage | `x_{ij}\in{0,1}`, `u_{ij}\in{-1,0,1}`, `y_j\ge0`, `z_j\in\mathbb Z_{\ge0}` | `s_{ij}=x_{ij}+u_{ij}+loaded_{ij}`, `L_j=\sum_i s_{ij}`, `W_j=\sum_i w_i s_{ij}` | `\sum_j s_{ij}=1` for assigned items; MLA/MLW and appointment pipelines | none directly |
| mac | reuse stowage variables | `T_p=\sum_j arm_jW_{jp}+fuelMoment_p+dryMoment`, `MAC_p=index_p/weight_p` | valid phase formulas/units; no direct solver row | none directly |
| airworthiness_security | reuse `W_j`, MAC, payload | `\rho_z=\sum_{j\in z}W_j/length_z`, cumulative endpoint loads, envelope point | density, zone/cumulative, payload, total-weight and envelope limits | none directly |
| soft_security | reuse stowage load/empty symbols | `empty_j=\mathbf1[L_j=0]`, ballast/advice deviation | soft empty/door/division/ballast pipelines (penalty rows) | minimize weighted soft penalties |
| mac_optimization | reuse MAC/torque | `d_p=|MAC_p-target_p|` and lateral moment | longitudinal range, lateral balance, stabilizer pipelines | minimize balance deviation when configured |
| express_effectiveness | reuse load indicators | `priorityGap_{ij}=priority_i-priority_j` and relative order | must-ship and priority-order pipelines | minimize priority violations |
| loading_effectiveness | reuse stowage/order symbols | source/destination adjacency, sequence and trailer-change expressions | adjacency, ahead/reserve/reweigh/order/trailer pipelines | minimize configured loading-operation penalties |
| redundancy | reuse MAC/torque | `R_p=availableMargin_p-usedMargin_p`, experimental balance | redundancy and experimental longitudinal-balance pipelines | none directly |
| recommended_weight_equalization | reuse `z_j` and order | `D_j=|z_j-recommended_j|`, appointment/order deviation | item-order, priority-appointment and equalization pipelines | minimize `\sum_jD_j` |
| payload_maximization | reuse `W_j` | `Payload=\sum_jW_j` | max-payload limit in WeightRecommendation | maximize `Payload` |

## 3. Cross-context mathematical contract

### 3.1 Stowage bridge

For an item-position pair that needs stowage, the core expression is:

$$
s_{ij}=x_{ij}+u_{ij}+loaded_{ij},
\qquad
L_j=\sum_{i\in I}s_{ij},
\qquad
W_j=\sum_{i\in I}w_i s_{ij}.
$$

Here `x` is binary assignment, `u` is the balance-ternary adjustment used by
the current implementation, and `loaded` is the fixed contribution of items
already on board. For pairs outside the status-driven feasible set, the source
fixes the expression to zero. `W_j` is then consumed by MAC, airworthiness,
soft-security, and objective contexts.

### 3.2 Typical invariant families

The shared stowage invariants are:

$$
\sum_{j\in J}s_{ij}=1 \quad(i\in I^{assign}),
\qquad
L_j\leq MLA_j,
\qquad
W_j\leq MLW_j.
$$

Context-specific limits add restrictions to these shared expressions. For
example, an airworthiness zone `z` may impose:

$$
\underline{\rho}_z\leq
\frac{\sum_{j\in z}W_j}{length_z}
\leq\overline{\rho}_z,
\qquad
\sum_{j\in z}W_j\leq\overline W_z.
$$

An envelope context constrains the pair `(phaseWeight_p, MAC_p)` to a
phase-specific feasible region; it is not equivalent to a single scalar
weight bound.

### 3.3 Mode registration

| Mode | Contexts in the ordinary model | Important boundary |
| --- | --- | --- |
| LoadingOrder | aircraft | export only; no meta-model solve |
| FullLoad | stowage, mac, airworthiness_security, soft_security, mac_optimization, express_effectiveness, loading_effectiveness | airworthiness may be moved to the Benders subproblem |
| Predistribution | FullLoad contexts plus redundancy | loading-order pipeline is mode-gated |
| WeightRecommendation | stowage, mac, airworthiness_security, express_effectiveness, recommended_weight_equalization, payload_maximization | recommendation deviation and payload objective are mode-specific |

The parent page documents the exact registration calls and the Benders
master/subproblem split. In particular, a class or model file that exists under
a context is not an active constraint until its pipeline is returned by the
mode's pipeline generator.

## 4. Review checklist

When extending a context page, preserve these invariants:

1. Every variable states whether it is binary, ternary, integer, or real, its
   physical unit, its status-driven index set, and its fixing rule.
2. Every intermediate is defined in prose and with a quantified formula; a
   derived expression is not itself a constraint.
3. Every assertion distinguishes an always-true data invariant from a solver
   constraint, and every constraint names its active pipeline.
4. Objectives are documented per application mode. Do not combine FullLoad,
   Predistribution, and WeightRecommendation into one objective.
5. Units follow `aircraftModel.weightUnit`; kilogram notation is valid only
   when that configured unit is kilogram.
