# Kerf, Cutting Process and Coils

After [your first cutting plan](./first-cutting), change kerf, cutting trees and coil supply one at a time. Regenerate patterns; changing a report's process label is insufficient.

## 1. Understand Kerf with One Cut

Cut a `10 × 6 m` region vertically at x=4 m with 0.5 m kerf. The left region is 4 m wide; the right starts at x=4.5 m and is 5.5 m wide. Conservation is `4 + 0.5 + 5.5 = 10`. Two children still totaling 10 m have not removed kerf.

Keep the first-lesson products and demands, change only the process or kerf, and solve again. Validate manufacturing process together with geometry.

Exercise: request one `4 × 6 m` and one `6 × 6 m` product without rotation. One 10×6 sheet fits at zero kerf; positive kerf invalidates that layout. Regenerate and check whether the application uses other supply/more sheets or correctly returns no deliverable.

## 2. Change Freeform to Guillotine

| Required information | Reason |
| --- | --- |
| Region position and dimensions | Every child must derive from the actual parent |
| Cut orientation, position and kerf | Determine child boundaries and removed material |
| Process stages | Two/three stages are not arbitrary-depth cutting |
| Product leaves and coordinates | Placement must agree with leaf regions |

Do not only check that a tree exists: products must match leaf regions, cuts must conserve material and the maximum stage must respect the process limit. Reuse the first-lesson real backend configuration; no test profile is required.

Update generation, pattern validation and acceptance together. No two-stage candidate does not prove impossibility under every process; report the selected process and search scope.

Inside the first-lesson runExample, pass changed instead of problem to solve. This modifies the existing input; it is not another standalone program:

```kotlin
val changed = problem.copy(
    process = CuttingProcess2.GuillotineTwoStage,
    kerf = FltX(0.5) * Meter
)
```

## 3. Measure Coils by Consumed Length

A coil has fixed width, while each plan needs an actual segment length. A pattern consuming 4 m used three times consumes 12 m, not “one pattern” or “3 m”.

## 4. Define Objectives and Waste

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
