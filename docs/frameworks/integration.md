# From Business Examples to Your Application

After a first lesson, keep the small example application before adding an HTTP service: read one fixed input, make one real framework call and print independently checkable results. Add databases, queues and UI after input mapping, outcomes and error handling are stable.

## 1. Keep the Shortest Path

```mermaid
flowchart LR
    A[Order and resource snapshot] --> B[Business mapping and unit checks]
    B --> C[Domain problem and configuration]
    C --> D[Framework and solver backend]
    D --> E[Terminal and domain validation]
    E --> F[Business result and audit record]
```

This is an application responsibility diagram, not a shared callable API. Framework entry points and return types differ. Do not discard original reports, quantities or errors to force them into a uniform interface.

| Layer | Include | Keep out |
| --- | --- | --- |
| Input adapter | Explicit conversion of order IDs, units, time, stock and resources | Silent zeros, dropped orders or merging distinct IDs by dimensions |
| Application orchestration | Domain problem, configuration, backend, budgets and extension wiring | A second implementation of existing conservation or overlap constraints |
| Business rules | Single-candidate policies or global modeling extensions | Discovering hard-rule violations only during report export |
| Output adapter | Validated plan, status, metrics, shortfalls and diagnostics | Presenting mocks, empty render objects or no-incumbent terminals as plans |

## 2. Change Five Things in Order

1. **Retain the hand-checkable baseline:** run the first-lesson business function and main before importing production orders.
2. **Confirm the execution path:** real backend examples need real backends; preserve heuristic semantics where documented. Connect both RMP and final phases for BPP2D/BPP3D, not preset prices or usages.
3. **Handle request-boundary errors:** translate CLI errors into structured server responses and retain reports for normal no-solution terminals. Entry points have different return types.
4. **Replace one data category at a time:** demand, then resources, then rules, preserving independently checkable invariants.
5. **Add infrastructure last:** databases, queues and UI; reject stale outputs and redact credentials and licenses from diagnostics.

Configure dependencies and native libraries using [Setup](./setup). First-lesson sources include imports; similarly named Kotlin/Rust constructors are not interchangeable.

## 3. Check Input Before Modeling

| Input | Business decision required |
| --- | --- |
| IDs and references | Unique identities and existing product, container, resource and node references |
| Counts and units | Integer pieces/rolls, explicit length/weight conversions and zero-demand treatment |
| Stock and bounds | Meaning of absent stock; distinguish `null`, zero and a missing field |
| Time and calendars | Time zone, allowed starts, intervals, frozen choices and execution facts |
| Objective | Priority of count, consumption, cost or service level, with explicit weight normalization |
| Budgets | Candidate, node, iteration and total-time limits, including how partial results are displayed |

Do not invent cancellation support where an entry point does not provide it; check backend capabilities. Prevent a cancelled or stale run from overwriting a newer schedule.

## 4. Success Still Requires Two Outcome Checks

Outer success means a report or result was returned. It may still describe infeasibility, a time limit without an incumbent, a time-limited incumbent or optimality. Read the actual language-specific fields and enums.

| Observation | Application action |
| --- | --- |
| Input or technical error | Return a locatable error; do not present an old plan as this run's success |
| Normal terminal without a deliverable | Show diagnostics; distinguish full-problem infeasibility from an insufficient pool |
| An incumbent exists | Validate business feasibility and retain its real terminal status |
| Current model proven optimal | Preserve model scope, candidate completeness, custom rules and evidence source |
| Geometry, stock or process validation fails | Do not deliver; retain input/report and inspect modeling or extraction |

A business response should retain request/input revision, configuration, actual backend, deliverability, original terminal, objective with units, plan, shortfalls and diagnostics. **This is an application design recommendation, not a common OSPF serialization schema.** Missing bounds/gaps remain unavailable, not zero.

## 5. Protect Integration with Three Test Groups

| Group | Examples | Acceptance |
| --- | --- | --- |
| Feasible baseline | One container, one pattern, two activities, four-node flow | Exact demand, geometry/process/conservation, resources and recomputed cost |
| Deliberately infeasible | Oversized item, frozen-calendar conflict, insufficient shared resource | No fabricated plan; correctly scoped conclusions |
| Boundaries and failures | Duplicate IDs, wrong units, zero stock, backend failure, no-incumbent time limit | No silent input changes; technical errors separate from business terminals |

Do not assert only the objective. Equivalent optima can differ in coordinates or activity order; prefer invariants and objective tolerances. Lock down a particular plan only when the business actually requires it.

Second-lesson hand calculations are exercises to implement, not production cases already executed in the repository. Linked contract tests demonstrate local capabilities, not the entire extended problem.

## 6. Continue with a Concrete Scenario

- [BPP1D: container types, stock and cost](./bpp1d/business-scenario)
- [BPP2D: sheet types, rotation and spacing](./bpp2d/business-scenario)
- [BPP3D: bin types and objectives](./bpp3d/business-scenario)
- [CSP1D: demand units and business rules](./csp1d/business-scenario)
- [CSP2D: kerf, cutting process and coils](./csp2d/business-scenario)
- [Gantt: calendars and rolling recovery](./gantt/business-scenario)
- [Network: shared resources, shortage and embedding](./network/business-scenario)

Determine rule ownership first: single-candidate rules belong in generation/policies; cross-candidate or cross-domain rules belong in a joint model. Use the [modeling workflow](/guide/modeling-workflow) and [solver result guide](/guide/solving-results), rather than extending by copying the solve loop.
