# Sheet Types, Rotation and Spacing

After [your first 2D layout](./first-layout), separate geometry, material consumption and cost with four items. Replace the first-lesson items and sheet types with this input.

## 1. Two Sheets and One Rectangle Type

| Object | Dimensions / amount | Price |
| --- | --- | ---: |
| R | 2 × 2 m, four pieces | — |
| S | 4 × 4 m, at least one available | 5 |
| L | 5 × 5 m, at least one available | 3 |

Use ample weight capacity, no rotation and zero spacing first. Four items fit an exact 2×2 grid on S and also fit on L. Pure area consumption favors S (16 m²); pure purchase cost favors L (3). Minimum sheet count cannot distinguish them.

Set spacing to 0.2 m: two items need `2 + 0.2 + 2 = 4.2 m` per row. S loses the 2×2 layout while L remains feasible. Total item area of 16 m² does not establish fit; four axis-aligned 2×2 squares have no alternative four-piece layout with positive gaps on a 4×4 sheet either.

## 2. Send Every Sheet Type to Generation

### Construct This Lesson’s Request

`makeRequest` / `make_input` builds R×4 and sheet types S/L, explicitly binding geometry to metres. Kotlin binds load to kilograms separately. Change `spacing` to 0.2 and solve again.

::: code-group

<<< ../_examples/bpp2d-business.kt#input [Kotlin]

<<< ../_examples/bpp2d-business.rs#input [Rust]

:::

::: warning Rust Unit Boundary
The current Rust interface shares a unit type between weight and length. This example is geometry-only with zero weight/load; it does not provide type-safe kilogram modeling. Production load handling needs explicit normalization and independent unit checks.
:::

### Put Costs in Objective Configuration


::: code-group

<<< ../_examples/bpp2d-business.kt#objective [Kotlin]

<<< ../_examples/bpp2d-business.rs#objective [Rust]

:::

Kotlin passes configuration through `request.executorConfig`. Supply Rust `cost_config()` to the application’s standard executor adapters; `run_business` receives those configured restricted-master and final-integer adapters rather than creating a backend.

| Responsibility | Kotlin | Rust |
| --- | --- | --- |
| Multiple types | `Bpp2dLayerGenerationRequest.sheets` | Sheet collection on `Bpp2dLayerGenerationRequest` |
| Candidate identity | `Layer.sheet` retains its type | Each candidate retains its actual sheet type |
| Spacing | Request `kerf` | Request `with_kerf(...)` |
| Objective | Area, count and per-type cost pipelines | Corresponding area, count and type-cost pipelines |

The primary/default `sheet` does not replace the full `sheets` collection. Identical coordinates on S and L are different columns: do not deduplicate by coordinates alone.

Use the first-lesson item constructors, create S and L sheet types and pass both into the request. Wire cost into the master objective rather than only adding prices to an output table.

## 3. Check the Spacing Path Separately

After changing spacing, regenerate candidates, solve again and independently measure actual edge-to-edge gaps. Fewer candidates do not prove every placement has legal spacing.

### Entry and Generation Budget {#entry-budget}

This service already uses column generation. Bound iterations and columns per round; these are neither exhaustive-enumeration evidence nor backend node limits.

::: code-group

<<< ../_examples/bpp2d-business.kt#entry [Kotlin]

<<< ../_examples/bpp2d-business.rs#entry [Rust]

:::

## 4. Add Approval Rules, Then Isolate Rotation

### Minimal Extension: Approved Sheet Types

Suppose L is not approved for purchase. Wrap generation to reject L. Kotlin injects the wrapper through `algorithmFactory` and applies it to both initial candidates and later generation, so initial columns cannot bypass the rule.

::: code-group

<<< ../_examples/bpp2d-business.kt#rule [Kotlin]

<<< ../_examples/bpp2d-business.rs#rule [Rust]

:::

Injection point:

::: code-group

<<< ../_examples/bpp2d-business.kt#injection [Kotlin]

<<< ../_examples/bpp2d-business.rs#injection [Rust]

:::

Rust injects generation into the lower-level algorithm and explicitly runs the packing analyzer. The final adapter must preserve sheet-type identity. A per-candidate approval rule is not a cross-sheet purchase-budget constraint.

Use one `3 × 2 m` sheet and one `2 × 3 m` rectangle with zero spacing. It fits with 90° rotation and does not fit without rotation. This isolates `allowRotate`; a square cannot demonstrate the difference.

## 5. Business Acceptance

- At zero spacing, a pure area objective favors S and pure cost favors L. With weighted objectives, recompute the configured score instead of retaining both single-objective expectations.
- Regenerate, solve and validate after a spacing change; moving coordinates in a picture does not enforce a rule.
- Stock/procurement limits require real master-model constraints. A catalog of sheet types is not modeled stock.
- Report net item area, consumed sheet area, free regions and reusable remnants separately. Reuse needs business rules such as minimum width/height.

Continue to [CSP2D process requirements](../csp2d/business-scenario) for a guillotine tree, not a renamed BPP2D layout. Complete the [integration checks](../integration) before delivering business results.

Repository regression commands live in [Verification](../verification). Complete the business input, call and result checks before using them.


## Complete Application Sources

The snippets come from the same source files. Keep imports and helpers, then call `runBusiness` / `run_business` from your application. Constructors can also be used independently for input validation. Reuse first-lesson backend dependencies; these functions do not create an implicit backend.

:::: details Full Kotlin / Rust sources

::: code-group

<<< ../_examples/bpp2d-business.kt [Kotlin]

<<< ../_examples/bpp2d-business.rs [Rust]

:::

::::
