# Scenario Analysis and Plan Comparison

Scenario analysis asks how plans change when business conditions change. Compare feasibility, scope, execution cost, and plan differences—not just objective values. Scenarios use independent inputs and do not overwrite published plans.

## 1. Freeze a baseline and identify changes

Establish a common baseline and specify which parameters, rules, or objectives each scenario changes. Retain model identity, horizon, units, and solve conditions. Otherwise an apparent gain may merely reflect changed demand rather than the value of an intervention.

Parameter changes, rule activation, and objective-priority changes are distinct. Activating rules can change the meaning of feasibility and needs clearer business justification than ordinary parameter substitution.

## 2. Example: overtime and outsourcing

### Overview, entities, and variables

Single-period delivery demand is $D=10$ items, with regular internal capacity 8. Internal production is $x\in\mathbb Z_{\ge0}$, extra overtime capacity $h\in\{0,\ldots,H\}$, and outsourced quantity $o\in\{0,\ldots,O\}$, all in items.

Parameters $(H,O)$ determine the options available in each scenario. Data assertions require nonnegative limits and actual availability. No additional predicates are needed.

### Intermediate values, constraints, and objective

Total delivery and additional cost are:

$$
Q=x+o,\qquad C=3h+5o.
$$

Assume regular capacity is contracted at a fixed charge. Compare only additional overtime and outsourcing costs, without a separate cost for changes in internal quantity. Require:

$$
\text{s.t.}\quad x\le8+h,\quad Q\ge10,\qquad \min C.
$$

$H,O$ are scenario parameters, not bounds that can be increased without approval.

### Scenario results

| Scenario | $H$ | $O$ | Optimal $(x,h,o)$ | Additional cost |
|---|---:|---:|---|---:|
| Original conditions | 0 | 0 | Infeasible | Not applicable |
| Overtime only | 1 | 0 | Infeasible | Not applicable |
| Outsourcing only | 0 | 2 | $(8,0,2)$ | 10 |
| Both options | 1 | 2 | $(9,1,1)$ | 8 |

In the joint scenario, one overtime item replaces one outsourced item, saving 2. Do not assign zero cost to an infeasible baseline and call the joint scenario a cost increase of 8: the baseline provides no comparable demand-satisfying plan.

## 3. Gains and interactions

The benefit of two changes need not equal the sum of individual benefits. They can substitute, enable one another, or jointly cross an integer threshold. Solve individual and combined scenarios for a fixed objective rather than extrapolating from one local sensitivity.

A finite comparison establishes results only under its modeled assumptions. It is not a real-world causal experiment or proof that the same intervention is preferable at every demand level.

## 4. Compare solve quality

Equal time limits do not guarantee equal optimality. If one scenario is proven optimal and another has only an incumbent, show bounds and gaps rather than calling their difference a change in optimal value.

For minimization scenarios A and B with optima in $[L_A,U_A]$ and $[L_B,U_B]$, cost improvement $z_A^*-z_B^*$ lies in:

$$
[L_A-U_B,\ U_A-L_B].
$$

If this interval crosses zero, the evidence may not determine which scenario has the lower optimal cost. This requires consistent cost definitions and valid bounds.

## 5. Compare plans, not just objectives

Equal-cost plans may use different suppliers, assignments, execution risks, or stability profiles. Report important indicators separately. If an indicator should influence selection, encode it as a constraint or objective rather than assuming an unstated preference.

Measure changes against a published plan through stable business identities, not variable addresses from another scenario. See [Rolling Optimization](./rolling-optimization).

## 6. Scenario execution and publication

Business contexts supply parameters and rules; the application creates independent scenarios and invokes the modeling workflow. Each result references its inputs and objective. Only an explicitly selected and approved plan enters publication; analysis does not change production configuration.

Warm starts can reuse candidate information, but feasibility must be checked in the new scenario. Result caches include scenario conditions and solve semantics, not merely model structure.

[Infeasibility Analysis](./infeasibility-analysis) explains conditions to adjust, [Critical Constraint Analysis](./critical-constraint-analysis) explains barriers to a better target, and the [LLM Business Path](./integrating-llms) shows controlled natural-language what-if analysis.
