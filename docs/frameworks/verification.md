# Repository Verification Reference

This reference is for source verification and framework maintenance, not a required onboarding step. Start with the application examples from [Choose a Framework](./). These commands run repository checks; they are not application examples.

Configure toolchains and native backends using [Setup](./setup). Offline contracts, fixture reports, enumeration oracles and native optimization are different. Zero tests or skipped tests do not establish success.

## BPP1D

### Kotlin: bounded integer enumeration

```powershell
# Working directory: ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-bpp1d/bpp1d-application -am test -T 0.75C `
    "-Dtest=Bpp1dTest" "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: packing verification

```powershell
# Working directory: ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp1d --lib packing_verifies_demands_inventory_materials_and_exact_coordinates -- --nocapture
```

```powershell
# Working directory: ospf-rust; requires the SCIP integration
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp1d --features "scip,scip-bundled" --lib scip_native_milp_optimal_lp_duals_and_inventory_seed_recovery -- --nocapture
```

Application usage: [BPP1D first lesson](./bpp1d/first-packing).

## BPP2D

### Kotlin: sheet-type candidates

```powershell
# Working directory: ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-bpp2d/bpp2d-application -am test -T 0.75C `
    "-Dtest=MixedSheetTypeContractTest" "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: candidates and geometry

```powershell
# Working directory: ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp2d --test layer_generation_contract multi_sheet_request_generates_per_type -- --nocapture
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp2d --test packing_contract -- --nocapture
```

```powershell
# Working directory: ospf-kotlin; requires Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-bpp2d/bpp2d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=GurobiColumnGeneration2DTest#gurobiSolvesRectanglePacking" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

```powershell
# Working directory: ospf-rust; offline generation, not native optimization
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp2d --test layer_generation_contract kerf_reaches_every_rectangle_generator -- --nocapture
```

Application usage: [BPP2D first lesson](./bpp2d/first-layout).

## BPP3D

### Kotlin: orchestration contract

```powershell
# Working directory: ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-bpp3d/bpp3d-application -am test -T 0.75C `
    "-Dtest=ColumnGenerationAlgorithmTest#algorithmShouldCompleteLpSpAddColumnAndFinalMilpFlow" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: materialization and trace replay

```powershell
# Working directory: ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp3d --lib application_service_replays_generated_layer_traces_to_render -- --nocapture
```

### Kotlin: minimal Gurobi assembly

```powershell
# Working directory: ospf-kotlin; requires Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-bpp3d/bpp3d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=GurobiColumnGenerationTest#applicationServiceShouldWorkWithGurobiDelegateAndPackingAnalyzer" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: SCIP CSV smoke

```powershell
# Working directory: ospf-rust; requires SCIP integration, without async
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp3d --features "serde,scip,scip-bundled" --lib csv_materialized_runs_real_scip_backend_smoke -- --nocapture
```

Application usage: [BPP3D first lesson](./bpp3d/first-loading).

## CSP1D

### Kotlin: fake application contract

```powershell
# Working directory: ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-csp1d/csp1d-application -am test -T 0.75C `
    "-Dtest=Csp1dApplicationAcceptanceTest#milpShouldSolveRollDemandWithoutDownstreamDependency" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: registration and heuristic extraction

```powershell
# Working directory: ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp1d --test csp1d_migration milp_solver_surface_registers_context_and_extracts_results -- --nocapture
```

```powershell
# Working directory: ospf-kotlin; requires Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-csp1d/csp1d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=Csp1dColumnGenerationRealSolverTest#milpDirectSolveWorksOnRealSolver" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

```powershell
# Working directory: ospf-rust; offline domain contract
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp1d --test csp1d_migration product_demand_legacy_helpers_and_domain_flags_match_kotlin -- --nocapture
```

Application usage: [CSP1D first lesson](./csp1d/first-slitting).

## CSP2D

### Kotlin: generation, assembly and recovery

```powershell
# Working directory: ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-csp2d/csp2d-application -am test -T 0.75C `
    "-Dtest=Csp2dApplicationContractTest" "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: default heuristic coverage

```powershell
# Working directory: ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp2d --test master_problem_contract milp_path_covers_demand -- --nocapture
```

```powershell
# Working directory: ospf-kotlin; requires Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-csp2d/csp2d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=GurobiCsp2dTest#sheetFreeform" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

```powershell
# Working directory: ospf-rust; offline geometry contract
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp2d --test cutting_geometry_contract vertical_cut_removes_kerf -- --nocapture
```

```powershell
# Working directory: ospf-rust; offline assignment contract
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp2d --test sheet_assignment_contract coil_accumulates_segment_height -- --nocapture
```

Application usage: [CSP2D first lesson](./csp2d/first-cutting).

## GANTT

### Kotlin: exclude one candidate

```powershell
# Working directory: ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-application -am test -T 0.75C `
    "-Dtest=ActivitySchedulingApplicationServiceTest#externalConstraintChangesOptimalChoiceWithoutModifyingApplication" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: application and extension

```powershell
# Working directory: ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --lib activity_application_single_call_extension_and_zero_column_solution -- --nocapture
```

### Kotlin: SCIP modes and precedence

```powershell
# Working directory: ospf-kotlin; requires JSCIP/SCIP
mvn -B -ntp -pl ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-application -am test -T 0.75C `
    -Pgantt-scip-integration-tests `
    "-Dtest=ActivitySchedulingNativeIntegrationTest#actualMilpChoosesFlexibleMachineAndPrecedenceFeasibleOrder" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: SCIP finite candidates

```powershell
# Working directory: ospf-rust; requires SCIP integration, without async
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --features "scip,scip-bundled" --lib activity_native_scip_flexible_job_shop -- --nocapture
```

```powershell
# Working directory: ospf-rust; finite enumeration/domain checks, not a native backend
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --lib activity_joint_phases_half_open_and_mid_activity_calendar_oracle -- --nocapture
```

Application usage: [GANTT first lesson](./gantt/first-schedule).

## NETWORK

### Kotlin: register and extract a witness

```powershell
# Working directory: ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-network-scheduling/network-scheduling-domain-flow-context -am test -T 0.75C `
    "-Dtest=FlowContextTest#knownTransshipmentOptimumAndParallelServicesAreExtractedByIdentity" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: bounded integer-flow oracle

```powershell
# Working directory: ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --lib heterogeneous_commodities_steps_cost_overrides_and_resources_match_oracle -- --nocapture
```

### Kotlin: SCIP four-node flow

```powershell
# Working directory: ospf-kotlin; requires JSCIP/SCIP
mvn -B -ntp -pl ospf-kotlin-framework-network-scheduling/network-scheduling-domain-flow-context -am test -T 0.75C `
    -Pscip-integration-tests "-Dtest=ScipMinCostFlowIntegrationTest" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust: SCIP standalone services

```powershell
# Working directory: ospf-rust; requires SCIP integration
cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --features "scip,scip-bundled" --lib scip_standalone_flow_and_generalized_flow_execute_real_backend_and_extract_domain_results -- --nocapture
```

```powershell
# Working directory: ospf-rust; finite integer oracle and extraction checks, not native solving
cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --lib shared_resource_can_make_an_otherwise_feasible_flow_infeasible -- --nocapture
```

Application usage: [NETWORK first lesson](./network/first-flow).

## Additional Source Reading

These links check local contracts and compatibility, not application usage.

### BPP1D

Read the [Kotlin contracts](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/bpp1d-application/src/test/fuookami/ospf/kotlin/framework/bpp1d/application/Bpp1dTest.kt) and [Rust contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/src/tests.rs) for extensions and stock. They are not native execution records for this new input.

### BPP2D

See complete factories and calls in the [Kotlin mixed-sheet contract](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp2d/bpp2d-application/src/test/fuookami/ospf/kotlin/framework/bpp2d/application/service/MixedSheetTypeContractTest.kt) and [Rust generation contract](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/tests/layer_generation_contract.rs). Read `generationProducesColumnsForEverySheetType` / `multi_sheet_request_generates_per_type` and the cost helpers. A cost helper alone does not enable an application objective; wire it into the actual model.

Check actual orientation, exact counts, sheet identity, bounds and overlap rather than only the renderer. See the [Rust packing contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/tests/packing_contract.rs) for geometry and remnant conventions.

### BPP3D

See [Kotlin MixedBinObjectivesTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp3d/bpp3d-domain-layer-assignment-context/src/test/fuookami/ospf/kotlin/framework/bpp3d/domain/layer_assignment/service/limits/MixedBinObjectivesTest.kt) and [Rust limits tests](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp3d/src/domain/layer_assignment/service/limits/tests.rs) for local coefficients, identity and metrics. They do not prove global optimality of this exercise.

### CSP1D

See [Kotlin ProductDemandModelTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp1d/csp1d-domain-material-context/src/test/kotlin/fuookami/ospf/kotlin/framework/csp1d/domain/material/model/ProductDemandModelTest.kt) and [Rust migration tests](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp1d/tests/csp1d_migration.rs). Read `demandDomainShouldBeDrivenByQuantityUnitInsteadOfMode` and Rust's helper contract before contribution calculations. Compatibility fixtures may simplify units; do not copy such simplifications into production density data.

Kotlin's [Csp1dDomainPolicyTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp1d/csp1d-application/src/test/kotlin/fuookami/ospf/kotlin/framework/csp1d/application/service/Csp1dDomainPolicyTest.kt) demonstrates material/machine policies with fake rules, not a complete capacity solve. Rust's `domain_policy_context_exposes_kotlin_contribution_for_helper` in the migration tests shows the plan context received by policies.

### CSP2D

These are actual inputs/assertions in the [Rust cutting geometry test](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/tests/cutting_geometry_contract.rs), not an optimization run.

The [Rust assignment test](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/tests/sheet_assignment_contract.rs) asserts three coil segments, zero sheets and total length 12. Kotlin's separate [coilProblemRecoversConsumedLength](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp2d/csp2d-application/src/test/fuookami/ospf/kotlin/framework/csp2d/application/service/Csp2dApplicationContractTest.kt) checks positive `consumedLength` and counts `sheetCount` as 1 in its fixture. **Do not translate these count fields directly across languages.** Define sheets, coil segments, pattern types and consumed length before normalizing business output.

### GANTT

This is Kotlin's `resourceCalendarAndMidActivityCapacityDropAreNotMissed` in [ActivitySchedulingContextTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-domain-activity-context/src/test/kotlin/fuookami/ospf/kotlin/framework/gantt_scheduling/domain/activity/ActivitySchedulingContextTest.kt). Half-open intervals matter: a 4 h start does not occupy `[3,4)`.

Rust's [scheduling contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-gantt-scheduling/src/domain/activity/tests/scheduling.rs) use a different joint-resource fixture:

### NETWORK

The [Rust flow contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-network-scheduling/src/domain/flow/tests.rs) contain the complete input and reject a supplied vector that violates the resource.

Kotlin's [commodityServiceCostsAvailabilityAndCrossArcResourceAreJointlyEnforced](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-flow-context/src/test/kotlin/fuookami/ospf/kotlin/framework/network_scheduling/domain/flow/FlowContextTest.kt) uses bulk=2 and parcel=1, total shared usage 3 and cost 8. It is a different registration/preset-vector extraction fixture, not native optimization of the first scenario.
