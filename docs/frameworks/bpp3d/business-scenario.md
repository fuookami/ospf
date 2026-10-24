# Bin Types and Objectives

After [your first 3D loading](./first-loading) connects final selection and packing analysis, introduce a second bin type. This exercise uses cubes so orientation complexity does not hide objective configuration.

## 1. A Scenario That Separates Objectives

Load ten `2 × 2 × 2 m` cubes, net volume 80 m³, with ample weight capacity and no extra spacing, support or balance rules:

| Type | Inner dimensions | Volume | Available bins | Cost per bin |
| --- | --- | ---: | ---: | ---: |
| S | 4 × 4 × 4 m | 64 m³ | 2 | 4 |
| L | 4 × 4 × 6 m | 96 m³ | 1 | 10 |

S fits at most eight cubes. L fits twelve on a 2×2×3 grid, so one L or two S can carry demand. Consumed volume is 96 versus 128 m³; cost is 10 versus 8. Minimum count and volume favor L; minimum cost favors two S.

The following constructors make this scenario callable. Compare the selected physical bins with this calculation, then verify actual placements and full demand coverage.

## 2. Connect Three Multi-Type Responsibilities

### Construct Requests from the Scenario

Kotlin reuses the first lesson’s `makeItem(edge = 2.0)`, creates physical bins S/S/L and enumerates actual placements containing the first one, two, and then more items in grid order; see the full function below. Rust uses one record per carton and loads and converts metric geometry, bins and initial layers. Retain recorded quantities and independently count delivered placements.

::: code-group

<<< ../_examples/bpp3d-business.kt#input [Kotlin]

<<< ../_examples/bpp3d-business.rs#input [Rust]

:::

::: info Two Input Paths
Kotlin candidates contain real placements. Rust CSV layer rows contain initial-layer metadata; geometry still depends on executor generation and reconstruction. Layer-row counts are not deliverable bins. The metric Rust interface also shares the weight/length unit type, requiring separate weight normalization checks in production.
:::

1. Include every type in generation: Kotlin `Bpp3dLayerGenerationRequest.bins`; Rust `LayerGenerationRequest.bins`.
2. Retain originating bin type in column identity. Identical coordinates on different types are different layers.
3. Supply actual available final bins: Kotlin `finalBins`, and Rust's corresponding final-executor input. A type catalog does not mean unlimited physical bins; array indices should not become cross-request identities.

Set exact demand to ten for the same item ID. Follow the first lesson's request factory to keep demand representations, initial columns and final bins consistent. A small bin's capacity of eight is not the order quantity.

## 3. Configure Objectives Explicitly

| Meaning | Kotlin field | Rust field |
| --- | --- | --- |
| Consumed volume weight | `finalBinVolumeWeight` | `final_bin_volume_weight` |
| Count coefficient | `finalBinAmountCoefficient` | `final_bin_amount_coefficient` |
| Type-cost weight | `finalBinTypeCostWeight` | `final_bin_type_cost_weight` |
| Type-cost map | `finalBinTypeCosts` | `final_bin_type_costs` |

See the [Kotlin standard executors](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp3d/bpp3d-application/src/main/fuookami/ospf/kotlin/framework/bpp3d/application/service/ColumnGenerationStandardExecutors.kt) and [Rust executor configuration](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp3d/src/application/service/executor/config.rs). Current defaults are volume weight 1, count coefficient `1e-6` and cost weight 0. Filling the cost map alone does not make cost dominate.

First isolate pure volume and pure cost. Then set the count coefficient to zero and explore normalized `score = consumed_volume + λ × cost`: L scores `96 + 10λ`; two S score `128 + 8λ`. They tie at λ=16; larger λ reverses the preference. Volume numbers here use m³, so define the weight's conversion convention.

A weighted sum is not strict volume-first, cost-second optimization. Such a requirement needs proven bounded weights or an explicitly evaluated staged optimization that fixes the first optimum. Do not claim default `1e-6` guarantees lexicographic priority at every scale.

### Configure a Pure Cost Objective

::: code-group

<<< ../_examples/bpp3d-business.kt#objective [Kotlin]

<<< ../_examples/bpp3d-business.rs#objective [Rust]

:::

### Entry and Generation Budget {#entry-budget}

Kotlin configures `cgConfig` on the request; Rust configures the service and also exposes a total-column cap. Both paths still require real restricted-master and final-integer backends.

::: code-group

<<< ../_examples/bpp3d-business.kt#entry [Kotlin]

<<< ../_examples/bpp3d-business.rs#entry [Rust]

:::

## 4. Validate Packing, Not Candidate Volume

### Minimal Rule: Only S Bins Are Approved

Kotlin `smallOnly` restricts physical bins, initial columns and `generators` together. The snippet shows the generator signature and request injection. Apply the restriction before optimization and audit delivered bins afterward.

::: code-group

<<< ../_examples/bpp3d-business.kt#rule [Kotlin]

<<< ../_examples/bpp3d-business.rs#rule [Rust]

:::

Rust filters available types and initial candidates before solving, then adds delivery validation through the public geometry validation interface:

<<< ../_examples/bpp3d-business.rs#injection [Rust]

::: warning Generation Rules Versus Delivery Guards
The custom geometry validator runs during packing analysis: rejection does not insert a constraint into the restricted master or pricing. The full source also restricts bins and initial candidates before solving. Model new mixing, cross-layer or total-budget rules in their generation/modeling paths, then audit them at delivery.
:::

- Place exactly ten items without counting one item through multiple selected layers.
- Validate bounds, overlap, orientations, weight and enabled support rules on actual placements.
- Recompute actual bin count, 96/128 m³ consumption and cost; unused volume is 16/48 m³ respectively.
- Check restricted-master and final-integer extensions and backends, `finalSolved` or corresponding diagnostics, packing snapshots and placements together.

Record objective coefficients separately from solve scope. Correct coefficients do not establish a complete candidate pool or global optimality of the new scenario.

If one item cannot fit, inspect orientation and generation first. If layers exist but final packing fails, inspect physical bins, demand and cross-layer geometry. Preserve original diagnostics using the [integration guide](../integration), rather than only increasing iterations.

Repository regression commands live in [Verification](../verification). Complete the business input, call and result checks before using them.


## Complete Application Sources

The snippets come from the same source files. Keep imports and helpers, then call `runBusiness` / `run_business` from your application. Constructors can also be used independently for input validation. Reuse first-lesson backend dependencies; these functions do not create an implicit backend.

:::: details Full Kotlin / Rust sources

::: code-group

<<< ../_examples/bpp3d-business.kt [Kotlin]

<<< ../_examples/bpp3d-business.rs [Rust]

:::

::::
