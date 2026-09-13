# Framework Example 4: Context Model Index

[中文](/zh-cn/examples/framework-example4/domain-models)

Demo4 is an architecture sample rather than a complete Kotlin application:
`Application` is empty and only `Demo4GenericQuantitySample` is executable.
This index therefore separates context contracts from application wiring. The
local model pages below are maintained model prose; data-only and
orchestration contexts explicitly document their absence of independent solver
variables rather than inventing a global model.

## 1. Context map

```text
task + rule + crew + cargo → bunch_generation → bunch_compilation
                                      ↘ bunch_selection (branch-and-price policy)
passenger ───────────────────────────→ bunch_compilation (optional pipelines)
```

The directed arrows describe data and service dependencies, not a claim that
`Application` registers every context in one model.

## 2. Context model pages

| Context | Model boundary | Local model |
| --- | --- | --- |
| task | flight tasks, legs, cycles, aircraft and recovery data; input to generation and compilation | [task](domain-task/domain-model); no independent optimization model is registered by `Application` |
| rule | links, restrictions, flow control, and feasibility/time/cost calculators | [rule](domain-rule/domain-model); predicates used by bunch generation |
| crew | crew members, ranks, schedules, and transit-time data | [crew](domain-crew/domain-model) |
| cargo | cargo data and capacity/disruption placeholders | [cargo](domain-cargo/domain-model) |
| passenger | passenger amount, cancellation, class/flight changes and their objective/constraint pipelines | [passenger](domain-passenger/domain-model) |
| bunch_generation | feasible bunch/column generation from task/rule/crew/cargo data and shadow prices | [bunch generation](domain-bunch_generation/domain-model); generation service, not a standalone master model |
| bunch_compilation | master-model column selection, task/flow/fleet/link/capacity expressions and incremental columns | [bunch compilation](domain-bunch_compilation/domain-model) |
| bunch_selection | branch-and-price policy, reduced-cost callback and orchestration | [bunch selection](domain-bunch_selection/domain-model); algorithm service, no independent variable family |

## 3. Shared notation and master contract

Shared symbols (`B_k`, `T`, `A`, `L`) and generated-column incidence coefficients are
defined in the linked context pages. The compilation page is the source of truth
for registered variables, slacks, constraints, and objective; this index does not
duplicate those formulas.

## 4. Context boundaries and evidence

1. `BunchGenerationContext.generateFlightTaskBunch` is the pricing callback.
   It returns generated bunches for a shadow-price map; it is not a solver
   constraint.
2. `BunchCompilationContext.register` creates the linear meta-model symbols
   and returns the active pipeline list. A model class that is not returned by
   that list is not an active assertion.
3. `PassengerContext.register` is an independent registration path. It should
   not be silently folded into the compilation equations unless the caller
   invokes it.
4. `BranchAndPriceAlgorithm` coordinates policy, bounds, reduced costs, and
   generated columns. It supplies algorithm state, not an additional domain
   variable family.

The parent page documents the generic-quantity executable and provides Kotlin/Rust
source entry points. Use these split pages for context-level vocabulary and
formula review, and the source links below for implementation line evidence.

5. [Kotlin Demo4 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo4)
6. [Rust Demo4 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo4.rs)
