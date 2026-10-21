# ospf-rust-framework-csp1d

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-framework-csp1d` 是 Kotlin `ospf-kotlin-framework-csp1d` 的 Rust 迁移版本。它提供可复用的一维分切框架，并把下游请求 DTO、公式语言、租户上下文、心跳逻辑和 solver 插件选择留在共享领域 crate 之外。

## 作用范围

本 crate 拥有可复用 CSP1D 内核：material/product 模型、切割方案生成、主问题 produce 建模、yield/waste/length 扩展、render DTO、solution enrichment、recovery、warm start，以及 application 层 MILP/column-generation 入口。

明确非目标：

1. 业务请求协议、公式语言、项目运行时参数、租户上下文、心跳逻辑或 solver 插件编排。
2. 下游专用缺陷、分段、位置或项目特定约束模型，除非它们先成为通用领域实体。
3. solver backend 安装和许可证管理。

## 模块结构

Kotlin 实现拆成多个 Gradle 模块。Rust 迁移在单个 crate 内保持相同职责边界：

| Rust 模块 | Kotlin 模块边界 | 职责 |
| --- | --- | --- |
| `infrastructure` | `csp1d-infrastructure` | Render DTO 和序列化边界。 |
| `domain::material` | `csp1d-domain-material-context` | 产品、需求、物料、设备、配规、切割方案、物理量和 shadow-price key。 |
| `domain::cutting_plan_generation` | `csp1d-domain-cutting-plan-generation-context` | Simple、DFS、N-Same、N-Sum、FullSum、Costar filler、reduced-cost pricing、生成约束、统计和 benchmark snapshot。 |
| `domain::produce` | `csp1d-domain-produce-context` | 主问题输入、produce aggregation、`MetaModel` 注册 context、增量列生命周期、扩展点、policy 和 shadow-price 生命周期。 |
| `domain::yield` | `csp1d-domain-yield-context` | 欠产/超产分析、yield slack aggregation、yield 约束和 yield 目标项。 |
| `domain::wasting_minimization` | `csp1d-domain-wasting-minimization-context` | 修边宽度、余料、物料成本、超产面积和 waste 目标项。 |
| `domain::length_assignment` | `csp1d-domain-length-assignment-context` | 动态产品长度分配、assigned/over-length slack aggregation、bounds 和 length 目标项。 |
| `application` | `csp1d-application` | assignment helper、problem/config builder、MILP 与 column-generation 入口、schedule 入口、warm start、recovery、solution enrichment、KPI、trace 和 render output。 |

## 架构概览

建模路径以 `Csp1dProduceContext` 和 `ProduceAggregation` 为轴心。内置与扩展 pipeline 向 `MetaModel` 注册变量、中间值、约束、目标和 shadow-price metadata。普通 MILP、column-generation LP master 和 final MILP 尽量复用同一 produce context 和 policy family。

application service 负责构造 problem、选择 generator 与 solve config、编排 MILP/LP/final-MILP 阶段、应用 warm start 或 recovery，并组装 `Csp1dSolution` 与 trace/KPI/render output。domain context 拥有建模语义，不应在 application solver flow 中重复实现。

## 核心概念

1. `Product`、`ProductDemand`、`Material`、`Machine`、`Costar`、`CuttingPlanSlice` 和 `CuttingPlan` 定义可复用 material domain。
2. `ProduceInput`、`Produce`、`ProduceAggregation` 和 `Csp1dProduceContext` 定义主问题建模面。
3. `Csp1dShadowPriceKey` 和 `Csp1dShadowPriceLifecycle` 保持 column generation 中 LP dual 提取稳定。
4. 生成策略通过 feasibility、dominance、cache 和 statistics 契约创建 initial/pricing cutting plan。
5. yield、waste 和 length-assignment context 通过标准 pipeline 添加可选约束和目标族。

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `csp1d_problem` / `Csp1dProblemBuilder` | problem 构造 DSL。 | migration |
| `Csp1dMilp` | 普通 MILP application 入口。 | migration |
| `Csp1dColumnGeneration` | 带 trace 的 column-generation application 入口。 | migration |
| `Csp1dSchedule` | Kotlin 兼容 schedule 入口，默认委托给 column generation。 | migration |
| `Csp1dMilpSolver` | 面向 `ProduceInput<V>` 的低层 Kotlin `Csp1dMilpSolver` 对齐入口。 | migration |
| `Csp1dSolveConfig` / `Csp1dExtensionSet` | 建模扩展、policy family、Top-K、partial solution 和 recovery 控制。 | migration |
| `Csp1dModelingExtension` / `Csp1dIncrementalPipeline` | 扩展 pipeline 与 accepted-column refresh hook。 | migration |
| `SimpleInitialCuttingPlanGenerator`、`DFSGenerator`、`NSameGenerator`、`NSumGenerator`、`FullSumGenerator`、`CostarFiller` | 初始方案生成。 | migration |
| `SimplePricingGenerator`、`ReducedCostPricingGenerator` | pricing 生成。 | migration |
| `RenderSchemaDTO` 及相关 DTO | renderer 序列化边界。 | stable within migration |

## 建模扩展点

`Csp1dSolveConfig<V>` 暴露建模扩展和 policy set。建模扩展包装 `Pipeline<MetaModel<f64>>`，并由 `Csp1dExtensionMode` 过滤：

- `MILP`：仅普通 MILP。
- `LP`：仅 column-generation LP master。
- `FINAL_MILP`：仅 column-generation final MILP。
- `ALL`：所有阶段。

可使用 `Csp1dModelingExtension::new(...)`、`Csp1dModelingExtension::with_mode(...)`，或 builder helper `extension_pipeline(...)`、`extension_pipeline_with_mode(...)`、`context_aware_extension_pipeline(...)`、`context_aware_extension_pipeline_with_mode(...)`。

需要响应 accepted pricing columns 的扩展可以实现 `Csp1dIncrementalPipeline`。`Csp1dProduceContext::add_columns(...)` 注册新方案变量、刷新内置 demand/material/machine/yield/length 约束组、重置目标，然后让增量扩展调整 confirmed column batch。

## 扩展策略

`Csp1dExtensionSet<V>` 聚合与 Kotlin 相同的 policy family：

- `Csp1dDomainPolicy`：candidate feasibility 和 width-feasibility override。
- `Csp1dObjectivePolicy`：objective batch coefficient 修改。
- `Csp1dGenerationStrategy`：candidate acceptance、canonical key override 和 dominance acceptance。
- `Csp1dPricingPolicy`：reduced-cost cost/benefit 修改和自定义 improvement 判断。
- `Csp1dFlowPolicy`：initial plan 过滤、column equivalence、early stop、termination selection、partial acceptance 和 recovery fallback。
- `Csp1dExtractionPolicy`：把 solution enrichment 写入 KPI details 和 render KPI maps。

policy 都有 no-op 默认实现以保持内置行为。extraction policy 失败会在 enrichment 期间被捕获，避免单个下游输出 hook 破坏求解结果。

## 泛型数值边界

public domain model 在表达可复用物理量语义时使用泛型数值。solver 注册当前通过 `to_f64`、`from_f64` 和 `convert_solver_value` 等显式 material/domain conversion helper 转为 `MetaModel<f64>`。裸 `f64` 应保留在 solver adapter、registration、extraction 和 renderer/serialization 边界。

## 物理量边界

宽度、长度、物料尺寸、产品尺寸、重量、需求量、产量、余料、修边宽度、assigned length 和 over-length 应通过 `Csp1dQuantity`、`QuantityRange`、`WidthRange` 或明确物理量包装表达。无量纲配置限制、计数、统计和 solver 内部系数可以使用裸数值。


## 热启动和恢复

### 热启动

框架支持从先前解热启动：

- **先前解热启动**：接受 `List<CuttingPlanUsage<V>>`，每个元素配对一个 `CuttingPlan` 和使用量。
- **原生初始值**：热启动使用量作为原生初始赋值写入最终 MILP 模型。
- **配置**：`Csp1dColumnGeneration` 构造函数中的 `warmStartPlanUsages` 参数。

### 恢复

恢复机制处理解适配：

- **先前解恢复**：`Csp1dRecovery` 将先前解适配到修改后的问题。
- **兼容子集过滤**：自动过滤在新问题中仍有效的切割方案使用量。
- **机器容量恢复**：机器容量约束和产出调整的特殊处理。
- **回退控制**：`retryWithoutWarmStart` 标志控制热启动失败时的行为。

### 错误处理

- `Csp1dRecoveryFallbackDisabledException`：当恢复失败且回退禁用时抛出。
- `Csp1dRecoverySolveException`：包装求解器失败，含追踪上下文。
## 求解生命周期

已测试生命周期：

1. 构造 `Csp1dProblem<V>` 和可选 `Csp1dSolveConfig<V>`。
2. 生成或接收初始切割方案。
3. 把 `Csp1dProduceContext` 注册到 `MetaModel`。
4. 普通 MILP 路径直接求解并提取 `Produce`。
5. column generation 路径求解 LP master、刷新 shadow price、运行 pricing，并调用 `add_columns`。
6. 在刷新后的方案池上求解 final MILP。
7. 提取 produce/yield/waste/length 结果、KPI details、render DTO、Top-K 方案、trace 和 partial/recovery status。

## 影子价格生命周期

Rust 保留 Kotlin `CGPipeline` 形态来处理 shadow-price-aware 约束。`DemandConstraintPipeline`、`MaterialConstraintPipeline`、`MachineConstraintPipeline` 和 `YieldConstraintPipeline` 使用 `Csp1dShadowPriceKey` metadata 注册约束，并实现 `Csp1dCGPipeline`。

`Csp1dShadowPriceLifecycle` 从模型约束和 dual slice 刷新 framework shadow price，转换为轻量 `ShadowPriceMap<V>`，并暴露 pricing 所需的 plan-level contribution extraction。当前 service path 在真实 LP solver backend 接入前使用 placeholder optimistic dual，但 refresh/extractor API 已经就位。

## 生成语义

生成支持最大/最小刀数约束、最大超产长度过滤、canonical 去重、candidate filter、来自 domain policy 的 width feasibility override、same-contribution 与 cross-contribution dominance pruning、quantity cache statistics、material width-index cache statistics 和 sequential slice-template cache statistics。并行大规模生成和 Kotlin 完整 statistics 细节仍在迁移。

## 输出

`Csp1dSolution<V>` 包含 selected produce data、可选 yield/waste/length 结果、generated plans、KPI details、render schema、solution status、failure message 和可选 Top-K cutting plans。

`Csp1dColumnGenerationTrace` 记录 initial/final plan counts、priced plan counts、iteration records、termination reason、initial/pricing statistics、final MILP status、partial availability 和 failure messages。

`RenderSchemaDTO`、`RenderCuttingPlanDTO`、`RenderCuttingPlanProductionDTO` 和 `RenderProductionType` 来自 `infrastructure::dto`。启用 `serde` feature 可派生 `Serialize` / `Deserialize`；DTO 字段使用 Kotlin 风格 camelCase，如 `cuttingPlans`、`unitLength` 和 `standardWidth`。

## 使用方式

可直接用 `Csp1dProblem::new(...)` 构造 problem，也可使用 builder helper：

```rust
use ospf_rust_framework_csp1d::{
    csp1d_problem, Csp1dColumnGeneration, Csp1dConfiguration, Csp1dMilp,
};

let problem = csp1d_problem::<f64, _>(|builder| {
    builder
        .products(products)
        .materials(materials)
        .machines(machines)
        .demands(demands)
        .configuration(Csp1dConfiguration {
            max_initial_plans: 128,
            max_pricing_plans: 32,
            iteration_limit: 16,
        })
        .solve_config_with(|config| {
            config
                .top_k_plan_limit(Some(10))
                .allow_partial_solution(true);
        });
});

let milp_solution = Csp1dMilp::default().solve(problem.clone(), None);
let cg_result = Csp1dColumnGeneration::default().solve_with_trace(problem, None);
```

`solve_config` 可挂在 `Csp1dProblem`，也可直接传给 `solve(...)` 或 `solve_with_trace(...)`，还可由 service-level defaults 提供。显式 solve 参数优先级最高。

## 本地验证

```powershell
cargo check -p ospf-rust-framework-csp1d
cargo test -p ospf-rust-framework-csp1d
cargo check -p ospf-rust-framework-csp1d --features serde
cargo test -p ospf-rust-framework-csp1d --features serde render_schema_serializes_with_kotlin_camel_case_fields
cargo test -p ospf-rust-core remove_constraints_by_group_id --lib
```

## 当前边界

当前 Rust public model 覆盖 Kotlin framework-level 实体：`Product`、`ProductDemand`、`Production`、`Costar`、`Material`、`Machine`、`CuttingPlanSlice`、`CuttingPlan`、demand contributions、`Csp1dAssignment`、yield/waste/length 输出、render DTO、warm start、recovery 和 extension policies。

迁移仍在进行中。普通 MILP、LP relaxation 和 final MILP 当前使用确定性启发式后端，同时保留 Kotlin 兼容 public surface 和 model-registration path。

已知待办：

1. 用真实 solver adapter 替换启发式 MILP/LP/final-MILP 后端。
2. 从真实 LP dual 驱动 shadow price，而不是 placeholder dual values。
3. 补齐 Kotlin 完整 generator statistics 和大规模/并行 cache 行为。
4. 扩展 Kotlin fixture parity、真实 solver warm-start smoke 覆盖和模块级 README。

## 初始受限主问题的可行性与 LP 终态

当初始 LP 因为列不足以覆盖全部需求而不可行时，列生成会切到独立的**可行性主问题**：

1. 该阶段的目标只最小化人工需求短缺（`csp1d_phase_one`），真实方案列的目标系数为零。
   否则「补列」会与业务目标混在一起，无法判断主问题是否只是不可行。
2. 人工短缺权重固定为 1、超产为 0：短缺是唯一需要被消除的不可行来源。
3. 若可行性 master 仍不可行，或定价找不到恢复列、或达到迭代上限，则保留失败并标记
   `LpInfeasible`。人工短缺**永远不会**被当成业务产出或最优定价证书。
4. 这只能说明当前生成范围无法恢复，不证明搜索范围之外没有可行列。

LP 终态校验（`application::lp_solver`）：

1. 只有取得**最优解与真实对偶**才交付可用于定价的 LP 结果。
2. 不可行、无界、其它非最优终态都返回显式错误，并保留原始终态描述。
3. 对偶缺失返回显式错误，绝不用零值伪造。

LP 失败原因会同时写入 `trace.lp_failure_message` 与 `solution.failureMessage`：最终 MILP 可能仍用
初始方案成功，此时业务状态可行，失败原因只能通过这条信息暴露。

### 求解器注入边界

`Csp1dLpSolver` 是可注入接口，实现方把已注册的 `MetaModel` 交给真实求解器并如实回报终态、目标与对偶。
未注入时列生成保持既有的启发式影子价格路径，因此本 crate 不引入任何求解器依赖；Gurobi/SCIP 接入仍由
feature-gated 的 core/framework 后端承担。

### 恢复后重建

可行性 master 取得最优解且目标 ≤ `1e-7`（人工短缺已消除）时，丢弃人工短缺、用当前累积列**重建原始
业务 master** 并重新求解；重建成功后本轮定价改用重建 master 的**真实对偶**。重建后仍不可行或终态异常
时保留失败，不声称已恢复。

### 求解器终态适配

`application::solver_adapter` 提供接入真实后端的唯一适配点：把后端各式各样的终态字符串归入四类语义
（不可行 / 无界 / 最优 / 其它），只有最优允许交付定价结果，未识别与子最优终态一律显式拒绝。
缺对偶时返回显式说明，绝不用零值伪造。

### 尚未完成

真实 Gurobi/SCIP 对偶提取仍受限于 core 侧尚未暴露的对偶 API；适配层已把「终态不可用」与「对偶缺失」
两条失败路径固化，接入时只需在适配器内填充对偶来源。

## 相关模块

- [根 README](../README_ch.md)
- [迁移交接](csp1d.md)
- [Kotlin CSP1D README](../../ospf-kotlin/ospf-kotlin-framework-csp1d/README_ch.md)
