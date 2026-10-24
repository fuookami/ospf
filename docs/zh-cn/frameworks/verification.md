# 仓库验证参考

本页面向需要复核源码或维护框架的读者，不是入门必做步骤。学习业务调用请从[框架选型](./)进入第一课。以下命令是仓库回归入口，不是应用示例代码。

先按[环境页](./setup)配置工具链与原生后端。规则检查、模拟求解报告、穷举对照和调用原生求解器验证的是不同内容；请按测试实际覆盖的范围理解结果。未运行测试或测试被跳过时，不计为验证通过。

## BPP1D

### Kotlin：有界整数枚举

```powershell
# 工作目录：ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-bpp1d/bpp1d-application -am test -T 0.75C `
    "-Dtest=Bpp1dTest" "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：装载复核

```powershell
# 工作目录：ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp1d --lib packing_verifies_demands_inventory_materials_and_exact_coordinates -- --nocapture
```

```powershell
# 工作目录：ospf-rust；需 SCIP 原生环境，或使用其 bundled 集成
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp1d --features "scip,scip-bundled" --lib scip_native_milp_optimal_lp_duals_and_inventory_seed_recovery -- --nocapture
```

学习调用：[BPP1D 第一课](./bpp1d/first-packing)。

## BPP2D

### Kotlin：候选板型

```powershell
# 工作目录：ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-bpp2d/bpp2d-application -am test -T 0.75C `
    "-Dtest=MixedSheetTypeContractTest" "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：候选与最终装箱几何

```powershell
# 工作目录：ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp2d --test layer_generation_contract multi_sheet_request_generates_per_type -- --nocapture
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp2d --test packing_contract -- --nocapture
```

```powershell
# 工作目录：ospf-kotlin；需要 Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-bpp2d/bpp2d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=GurobiColumnGeneration2DTest#gurobiSolvesRectanglePacking" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

```powershell
# 工作目录：ospf-rust；候选生成规则测试，不调用原生求解器
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp2d --test layer_generation_contract kerf_reaches_every_rectangle_generator -- --nocapture
```

学习调用：[BPP2D 第一课](./bpp2d/first-layout)。

## BPP3D

### Kotlin：求解流程测试

```powershell
# 工作目录：ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-bpp3d/bpp3d-application -am test -T 0.75C `
    "-Dtest=ColumnGenerationAlgorithmTest#algorithmShouldCompleteLpSpAddColumnAndFinalMilpFlow" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：数据转换与放置记录读取

```powershell
# 工作目录：ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp3d --lib application_service_replays_generated_layer_traces_to_render -- --nocapture
```

### Kotlin：Gurobi 最小调用示例

```powershell
# 工作目录：ospf-kotlin；需要 Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-bpp3d/bpp3d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=GurobiColumnGenerationTest#applicationServiceShouldWorkWithGurobiDelegateAndPackingAnalyzer" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：SCIP CSV 基本流程测试

```powershell
# 工作目录：ospf-rust；需要 SCIP 集成，不启用 async
cargo +nightly-2026-08-01 test -p ospf-rust-framework-bpp3d --features "serde,scip,scip-bundled" --lib csv_materialized_runs_real_scip_backend_smoke -- --nocapture
```

学习调用：[BPP3D 第一课](./bpp3d/first-loading)。

## CSP1D

### Kotlin：使用模拟求解器检查应用接口

```powershell
# 工作目录：ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-csp1d/csp1d-application -am test -T 0.75C `
    "-Dtest=Csp1dApplicationAcceptanceTest#milpShouldSolveRollDemandWithoutDownstreamDependency" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：注册与启发式提取

```powershell
# 工作目录：ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp1d --test csp1d_migration milp_solver_surface_registers_context_and_extracts_results -- --nocapture
```

```powershell
# 工作目录：ospf-kotlin；需要 Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-csp1d/csp1d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=Csp1dColumnGenerationRealSolverTest#milpDirectSolveWorksOnRealSolver" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

```powershell
# 工作目录：ospf-rust；领域规则测试，不调用原生求解器
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp1d --test csp1d_migration product_demand_legacy_helpers_and_domain_flags_match_kotlin -- --nocapture
```

学习调用：[CSP1D 第一课](./csp1d/first-slitting)。

## CSP2D

### Kotlin：生成、配置与恢复

```powershell
# 工作目录：ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-csp2d/csp2d-application -am test -T 0.75C `
    "-Dtest=Csp2dApplicationContractTest" "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：默认启发式覆盖

```powershell
# 工作目录：ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp2d --test master_problem_contract milp_path_covers_demand -- --nocapture
```

```powershell
# 工作目录：ospf-kotlin；需要 Gurobi
mvn -B -ntp -pl ospf-kotlin-framework-csp2d/csp2d-application -am test -T 0.75C `
    -Pgurobi-cg-test "-Dtest=GurobiCsp2dTest#sheetFreeform" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

```powershell
# 工作目录：ospf-rust；几何规则测试，不调用原生求解器
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp2d --test cutting_geometry_contract vertical_cut_removes_kerf -- --nocapture
```

```powershell
# 工作目录：ospf-rust；原料分配规则测试，不调用原生求解器
cargo +nightly-2026-08-01 test -p ospf-rust-framework-csp2d --test sheet_assignment_contract coil_accumulates_segment_height -- --nocapture
```

学习调用：[CSP2D 第一课](./csp2d/first-cutting)。

## Gantt

### Kotlin：用扩展排除一个候选

```powershell
# 工作目录：ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-application -am test -T 0.75C `
    "-Dtest=ActivitySchedulingApplicationServiceTest#externalConstraintChangesOptimalChoiceWithoutModifyingApplication" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：应用调用与扩展

```powershell
# 工作目录：ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --lib activity_application_single_call_extension_and_zero_column_solution -- --nocapture
```

### Kotlin：SCIP 模式与依赖

```powershell
# 工作目录：ospf-kotlin；需要 JSCIP/SCIP
mvn -B -ntp -pl ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-application -am test -T 0.75C `
    -Pgantt-scip-integration-tests `
    "-Dtest=ActivitySchedulingNativeIntegrationTest#actualMilpChoosesFlexibleMachineAndPrecedenceFeasibleOrder" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：SCIP 有限候选排程

```powershell
# 工作目录：ospf-rust；需要 SCIP 集成，不启用 async
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --features "scip,scip-bundled" --lib activity_native_scip_flexible_job_shop -- --nocapture
```

```powershell
# 工作目录：ospf-rust；有限枚举与领域验证，不运行原生后端
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --lib activity_joint_phases_half_open_and_mid_activity_calendar_oracle -- --nocapture
```

学习调用：[Gantt 第一课](./gantt/first-schedule)。

## Network

### Kotlin：模型注册与已知解的结果解析

```powershell
# 工作目录：ospf-kotlin
mvn -B -ntp -pl ospf-kotlin-framework-network-scheduling/network-scheduling-domain-flow-context -am test -T 0.75C `
    "-Dtest=FlowContextTest#knownTransshipmentOptimumAndParallelServicesAreExtractedByIdentity" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：有限整数流穷举对照

```powershell
# 工作目录：ospf-rust
cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --lib heterogeneous_commodities_steps_cost_overrides_and_resources_match_oracle -- --nocapture
```

### Kotlin：SCIP 四节点网络

```powershell
# 工作目录：ospf-kotlin；需要 JSCIP/SCIP
mvn -B -ntp -pl ospf-kotlin-framework-network-scheduling/network-scheduling-domain-flow-context -am test -T 0.75C `
    -Pscip-integration-tests "-Dtest=ScipMinCostFlowIntegrationTest" `
    "-Dsurefire.failIfNoSpecifiedTests=false"
```

### Rust：SCIP 独立应用服务

```powershell
# 工作目录：ospf-rust；需要 SCIP 集成
cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --features "scip,scip-bundled" --lib scip_standalone_flow_and_generalized_flow_execute_real_backend_and_extract_domain_results -- --nocapture
```

```powershell
# 工作目录：ospf-rust；有限整数穷举对照与结果解析复核，不调用原生求解器
cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --lib shared_resource_can_make_an_otherwise_feasible_flow_infeasible -- --nocapture
```

学习调用：[Network 第一课](./network/first-flow)。

## 源码阅读补充

以下链接用于核对接口行为与兼容行为，不替代第一课业务示例。

### BPP1D

扩展接口与库存规则可读 [Kotlin 完整测试](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/bpp1d-application/src/test/fuookami/ospf/kotlin/framework/bpp1d/application/Bpp1dTest.kt)及 [Rust 完整测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/src/tests.rs)。每个测试的结论适用于其列出的输入和条件。

### BPP2D

输入构造和完整调用见 [Kotlin 多板型测试](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp2d/bpp2d-application/src/test/fuookami/ospf/kotlin/framework/bpp2d/application/service/MixedSheetTypeContractTest.kt)与 [Rust 候选生成测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/tests/layer_generation_contract.rs)。重点读 `generationProducesColumnsForEverySheetType` / `multi_sheet_request_generates_per_type`，以及费用辅助函数。费用辅助函数存在不等于应用已经启用该目标；还要接到实际模型。

输出应检查实际朝向、精确件数、板型 ID、越界与重叠，而不是只检查渲染器的图片。几何与余料统计方式见 [Rust 装箱测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/tests/packing_contract.rs)。

### BPP3D

目标与身份局部测试见 [Kotlin MixedBinObjectivesTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp3d/bpp3d-domain-layer-assignment-context/src/test/fuookami/ospf/kotlin/framework/bpp3d/domain/layer_assignment/service/limits/MixedBinObjectivesTest.kt)和 [Rust 目标与约束测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp3d/src/domain/layer_assignment/service/limits/tests.rs)。这些验证目标系数/指标，不证明新练习的全局最优。

### CSP1D

完整对照见 [Kotlin ProductDemandModelTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp1d/csp1d-domain-material-context/src/test/kotlin/fuookami/ospf/kotlin/framework/csp1d/domain/material/model/ProductDemandModelTest.kt)和 [Rust 兼容性测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp1d/tests/csp1d_migration.rs)。先检查 `demandDomainShouldBeDrivenByQuantityUnitInsteadOfMode` 及对应 Rust 辅助函数测试，再读单次产出计算。不要照搬测试里用于兼容验证的单位简化作为生产密度配置。

Kotlin 的 [Csp1dDomainPolicyTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp1d/csp1d-application/src/test/kotlin/fuookami/ospf/kotlin/framework/csp1d/application/service/Csp1dDomainPolicyTest.kt)用模拟策略展示原料与设备的兼容检查；完整产能约束需另行建模和求解。Rust 可从兼容性测试中的 `domain_policy_context_exposes_kotlin_contribution_for_helper` 阅读策略如何接收方案上下文。

### CSP2D

[Rust 切分几何测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/tests/cutting_geometry_contract.rs)给出切割前后的区域边界，可用于核对刀缝是否正确扣除。

该 [Rust 分配测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/tests/sheet_assignment_contract.rs)断言卷材段数为 3、板材张数为 0、总段长为 12。Kotlin 的 [coilProblemRecoversConsumedLength](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp2d/csp2d-application/src/test/fuookami/ospf/kotlin/framework/csp2d/application/service/Csp2dApplicationContractTest.kt)采用另一组测试数据，检查正的 `consumedLength`，其中 `sheetCount` 还计为 1。**不能把两种语言的同类计数字段直接对译。** 业务输出应先统一板张、卷段、图案种类与消耗长度的定义。

### Gantt

Kotlin [ActivitySchedulingContextTest](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-gantt-scheduling/gantt-scheduling-domain-activity-context/src/test/kotlin/fuookami/ospf/kotlin/framework/gantt_scheduling/domain/activity/ActivitySchedulingContextTest.kt)中的 `resourceCalendarAndMidActivityCapacityDropAreNotMissed` 检查活动中途的停机和容量下降。占用区间左闭右开：4 h 开始不再占用 `[3,4)`。

Rust 的 [排程测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-gantt-scheduling/src/domain/activity/tests/scheduling.rs)使用另一组多资源测试数据，可继续检查分阶段占用和容量日历。

### Network

完整输入在 [Rust 网络流测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-network-scheduling/src/domain/flow/tests.rs)。该测试还检查把违反资源约束的给定解交给提取器会被拒绝。

Kotlin 的 [commodityServiceCostsAvailabilityAndCrossArcResourceAreJointlyEnforced](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-flow-context/src/test/kotlin/fuookami/ospf/kotlin/framework/network_scheduling/domain/flow/FlowContextTest.kt)使用 bulk=2、parcel=1，共占共享资源 3，费用为 8。它是另一组注册/给定向量提取测试数据，不是上节问题的原生优化。
