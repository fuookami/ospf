# Kerf, Cutting Process and Coils

After [your first cutting plan](./first-cutting), change kerf, cutting trees and coil supply one at a time. Regenerate patterns; changing a report's process label is insufficient.

## 1. Understand Kerf with One Cut

Cut a `10 × 6 m` region vertically at x=4 m with 0.5 m kerf. The left region is 4 m wide; the right starts at x=4.5 m and is 5.5 m wide. Conservation is `4 + 0.5 + 5.5 = 10`. Two children still totaling 10 m have not removed kerf.

The next section constructs both products and their demands explicitly. Validate manufacturing process together with geometry.

Exercise: request one `4 × 6 m` and one `6 × 6 m` product without rotation. One 10×6 sheet fits at zero kerf; positive kerf invalidates that layout. Regenerate and check whether the application uses other supply/more sheets or correctly returns no deliverable.

## 2. Change Freeform to Guillotine

### Construct Products, Kerf and Process

Build a 10×6 m sheet and one each of a=4×6 m and b=6×6 m, explicitly forbidding rotation. Use `makeProblem(0.5)` / `make_problem(0.5)` for positive kerf, or 0.0 for the baseline.

::: code-group

<<< ../_examples/csp2d-business.kt#input [Kotlin]

<<< ../_examples/csp2d-business.rs#input [Rust]

:::

| Required information | Reason |
| --- | --- |
| Region position and dimensions | Every child must derive from the actual parent |
| Cut orientation, position and kerf | Determine child boundaries and removed material |
| Process stages | Two/three stages are not arbitrary-depth cutting |
| Product leaves and coordinates | Placement must agree with leaf regions |

Do not only check that a tree exists: products must match leaf regions, cuts must conserve material and the maximum stage must respect the process limit. Reuse the backend configuration from the first lesson.

Update generation, pattern validation and acceptance together. No two-stage candidate does not prove impossibility under every process; report the selected process and search scope.

`makeProblem` sets `GuillotineTwoStage`; change process and kerf at input construction, then regenerate patterns.

## 3. Complete the Pool and Bound Generation

### Add a Real Through-Cut Pattern When the Pool Is Insufficient

The current default Rust shelf generator returns `NoCandidatePlan` in this tight-height, positive-kerf case: its row pitch reserves kerf and fails to generate these full-height products. That does not disprove cutting a and b on separate sheets. The specialized templates below place a through cut at each product’s right edge, derive remnants through `Region2.cut`, and validate geometry and tree before master registration.

::: code-group

<<< ../_examples/csp2d-business.kt#cut [Kotlin]

<<< ../_examples/csp2d-business.rs#cut [Rust]

:::

Section 4’s `solveRestricted(solver, kerf = 0.5)` / `solve_restricted(master_solver, 0.5)` sends these two patterns directly to the domain master with an application-supplied backend. Pass 0.0 for the zero-kerf no-mixing exercise. These templates only cover this lesson’s full-height parts with positive right-side remnants, not general nesting.

### Switch Entry and Bound Generation {#entry-budget}

For the kerf scenario without the custom no-mixing rule, compare once-generated and incrementally appended pools on the same input. Candidate limits bound patterns per round; iteration limits bound rounds.

::: code-group

<<< ../_examples/csp2d-business.kt#entry [Kotlin]

<<< ../_examples/csp2d-business.rs#entry [Rust]

:::

::: info Rust Execution Semantics
These Rust examples use default heuristic paths; method names, final statuses and a returned solution do not establish native optimization. CSP2D accepts a master solver, but its high-level path may fall back; call the domain master directly when fallback is forbidden, as in `solve_restricted`.
:::

## 4. Apply No-Mixing Rules and Define Waste

### Minimal Extension: One Product per Pattern

Implement `PatternGenerator2` by calling default process generation separately for each product to produce single-product alternatives, then reject mixed-product patterns. Wrap the entire Kotlin `PatternGenerationContext2`, not only one internal generator, so replayed patterns are checked too.

::: code-group

<<< ../_examples/csp2d-business.kt#rule [Kotlin]

<<< ../_examples/csp2d-business.rs#rule [Rust]

:::

Injection point:

::: code-group

<<< ../_examples/csp2d-business.kt#injection [Kotlin]

<<< ../_examples/csp2d-business.rs#injection [Rust]

:::

::: info High-Level Extension Boundary
Neither Kotlin high-level solver nor Rust `Csp2dSolvers` accepts a custom generator. `restrictedPlans` only produces a restricted pool; calling a normal high-level entry would regenerate patterns and bypass this restriction. The following code sends the restricted pool directly to the domain master and backend. This minimal assembly covers the lesson’s fixed-sheet piece demand; add machine, overproduction and coil pipelines as needed.
:::

::: code-group

<<< ../_examples/csp2d-business.kt#master [Kotlin]

<<< ../_examples/csp2d-business.rs#master [Rust]

:::

### Measure Coils by Consumed Length

A coil has fixed width, while each plan needs an actual segment length. A pattern consuming 4 m used three times consumes 12 m, not “one pattern” or “3 m”.

Start with sheet count for fixed sheets and consumed length for coils. Mixed materials, widths or prices require an explicit common objective; adding sheets to meters is not meaningful.

Distinguish net product area, consumed area, kerf loss and remaining regions. Reusable remnants require business width/height/process thresholds; not every blank region returns to inventory. Do not claim the framework exposes every decomposition without checking actual fields.

## 5. Application Acceptance

1. Contributions times usage satisfy demand without exceeding configured surplus allowances.
2. Material use respects stock; segment length times usage matches total coil length.
3. Coordinates obey bounds and non-overlap, with consistent kerf in generation and validation.
4. Guillotine trees satisfy stage, orientation, child-region and leaf rules.
5. Preserve actual heuristic/native execution and termination semantics; no added pattern is not automatically certified LP pricing convergence.

Retain input, process configuration and diagnostics using the [integration guide](../integration). For placement without manufacturing output, compare [BPP2D](../bpp2d/business-scenario).

Repository regression commands live in [Verification](../verification). Complete the business input, call and result checks before using them.


## Complete Application Sources

The snippets come from the same source files. Keep imports and helpers, then call `runBusiness` / `run_business` from your application. Constructors can also be used independently for input validation. Reuse first-lesson backend dependencies; these functions do not create an implicit backend.

:::: details Full Kotlin / Rust sources

::: code-group

<<< ../_examples/csp2d-business.kt [Kotlin]

<<< ../_examples/csp2d-business.rs [Rust]

:::

::::
