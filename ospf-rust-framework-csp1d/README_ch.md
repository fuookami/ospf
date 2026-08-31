# ospf-rust-framework-csp1d

[English](README.md)

`ospf-rust-framework-csp1d` 是 Kotlin `ospf-kotlin-framework-csp1d` 的 Rust 迁移版本。它沉淀可复用的一维分切框架；下游请求 DTO、公式语言、租户上下文、心跳逻辑和 solver 插件选择仍由业务适配层负责。

## 模块结构

Kotlin 实现拆成多个 Gradle 模块。Rust 迁移在单个 crate 内保持相同职责边界：

| Rust 模块 | Kotlin 模块边界 | 描述 |
|-----------|-----------------|------|
| `infrastructure` | `csp1d-infrastructure` | render DTO 与序列化边界。 |
| `domain::material` | `csp1d-domain-material-context` | 产品、需求、物料、设备、配规、切割方案、物理量和影子价格 key。 |
| `domain::cutting_plan_generation` | `csp1d-domain-cutting-plan-generation-context` | Simple、DFS、N-Same、N-Sum、FullSum、Costar filler、reduced-cost 定价、生成约束、统计和 benchmark 快照。 |
| `domain::produce` | `csp1d-domain-produce-context` | 主问题输入、产出聚合、MetaModel 注册上下文、增量加列生命周期、扩展点、策略和影子价格生命周期。 |
| `domain::yield` | `csp1d-domain-yield-context` | 欠产/超产分析、yield slack 聚合、yield 约束和目标项。 |
| `domain::wasting_minimization` | `csp1d-domain-wasting-minimization-context` | 余宽、余料、材料成本、超产面积和 waste 目标项。 |
| `domain::length_assignment` | `csp1d-domain-length-assignment-context` | 动态产品长度分配、assigned/over-length slack 聚合、上下界和 length 目标项。 |
| `application` | `csp1d-application` | assignment helper、问题/配置 builder、MILP 和列生成入口、Schedule 入口、warm start、recovery、solution enrichment、KPI、trace 与 render 输出。 |

## 当前边界

Rust public 模型覆盖 Kotlin framework 层的通用实体：`Product`、`ProductDemand`、`Production`、`Costar`、`Material`、`Machine`、`CuttingPlanSlice`、`CuttingPlan`、需求贡献、`Csp1dAssignment`、yield/waste/length 输出、render DTO、warm start、recovery 和扩展策略。

业务请求协议、公式语言、项目运行参数、租户上下文、心跳逻辑、特定缺陷约束、分段语义或 solver 插件编排仍不进入共享领域 crate。

迁移仍在进行中。普通 MILP、LP relaxation 和最终 MILP 目前使用确定性启发式后端，同时保留 Kotlin 对齐的 public surface 和模型注册路径。真实 LP RMP 对偶来源、solver adapter 和最终 MILP 后端仍待接入。

## 基本使用

可以直接使用 `Csp1dProblem::new(...)`，也可以通过 builder helper 构建问题：

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

`solve_config` 可以挂在 `Csp1dProblem`，也可以作为 `solve(...)` 或 `solve_with_trace(...)` 参数显式传入，还可以通过 service 层默认 generator 和 warm-start usages 等配置提供。显式求解参数优先级最高。`Csp1dSchedule` 也作为 Kotlin 兼容的排程入口暴露，默认委托列生成求解。

为了对齐 Kotlin 低层 `Csp1dMilpSolver`，Rust 暴露了面向 `ProduceInput<V>` 的 `Csp1dMilpSolver::solve(...)` 和 `Csp1dMilpSolver::solve_lp(...)`。返回值包含构建出的 `MetaModel`、变量取值、framework shadow-price map 和 pricing 可消费的轻量 `ShadowPriceMap<V>`。

## 建模扩展

`Csp1dSolveConfig<V>` 暴露建模扩展和策略集合。建模扩展包装 `Pipeline<MetaModel<f64>>`，并通过 `Csp1dExtensionMode` 控制适用阶段：

- `MILP`：仅普通 MILP。
- `LP`：仅列生成 LP master。
- `FINAL_MILP`：仅列生成最终 MILP。
- `ALL`：所有阶段。

可使用 `Csp1dModelingExtension::new(...)`、`Csp1dModelingExtension::with_mode(...)`，或 builder helper：`extension_pipeline(...)`、`extension_pipeline_with_mode(...)`、`context_aware_extension_pipeline(...)`、`context_aware_extension_pipeline_with_mode(...)`。

需要响应新增 pricing 列的扩展可以实现 `Csp1dIncrementalPipeline`。`Csp1dProduceContext::add_columns(...)` 会注册新方案变量、刷新内置 demand/material/machine/yield/length 约束组、重设目标函数，然后让增量扩展调整确认后的新增列批次。

## 扩展策略

`Csp1dExtensionSet<V>` 聚合与 Kotlin 对齐的策略族：

- `Csp1dDomainPolicy`：候选可行性和幅宽可行性覆盖。
- `Csp1dObjectivePolicy`：目标函数批次系数修正。
- `Csp1dGenerationStrategy`：候选验收、canonical key 覆盖和 dominance 验收。
- `Csp1dPricingPolicy`：reduced-cost 的 cost/benefit 修正和自定义改善判断。
- `Csp1dFlowPolicy`：初始方案过滤、列等价判断、提前停止、终止原因选择、partial 接受和 recovery fallback。
- `Csp1dExtractionPolicy`：向 KPI details 和 render KPI map 写入下游增强输出。

所有策略都有 no-op 默认语义以保持内置行为。提取策略的失败会在 enrichment 阶段被捕获，单个下游输出 hook 不会破坏求解结果。

## 影子价格生命周期

Rust 保留 Kotlin CGPipeline 形状来承接影子价格约束。`DemandConstraintPipeline`、`MaterialConstraintPipeline`、`MachineConstraintPipeline` 和 `YieldConstraintPipeline` 会以 `Csp1dShadowPriceKey` metadata 注册约束，并实现 `Csp1dCGPipeline`。

`Csp1dShadowPriceLifecycle` 根据模型约束和 dual slice 刷新 framework shadow price，转换出轻量 `ShadowPriceMap<V>`，并提供方案级 shadow-price contribution 提取供 pricing 使用。当前服务流程仍使用占位乐观 dual，等待真实 LP solver 后端接入；但 refresh/extractor API 已经就位。

## 生成语义

crate 暴露 `SimpleInitialCuttingPlanGenerator`、`DFSGenerator`、`NSameGenerator`、`NSumGenerator`、`FullSumGenerator`、`CostarFiller`、`SimplePricingGenerator` 和 `ReducedCostPricingGenerator`。

生成过程支持最大/最小刀数、最大超产长度过滤、canonical 去重、候选过滤、domain policy 接管幅宽可行性、同贡献与跨贡献 dominance 剪枝、数量缓存统计、物料幅宽索引缓存统计和顺序切片模板缓存统计。并行大规模生成和 Kotlin 全量 statistics 细节仍在迁移中。

## 输出

`Csp1dSolution<V>` 包含选中产出数据、可选 yield/waste/length 结果、生成方案池、KPI details、render schema、解状态、失败信息和可选 Top-K 切割方案。

`Csp1dColumnGenerationTrace` 记录初始/最终方案数量、每轮 pricing 新增数量、iteration records、终止原因、初始/pricing 统计、最终 MILP 状态、partial 可用性和失败信息。

`RenderSchemaDTO`、`RenderCuttingPlanDTO`、`RenderCuttingPlanProductionDTO` 和 `RenderProductionType` 位于 `infrastructure::dto`。启用 `serde` feature 后会派生 `Serialize` / `Deserialize`；DTO 字段按 Kotlin 风格输出 camelCase，例如 `cuttingPlans`、`unitLength` 和 `standardWidth`。

## 本地验证

Rust 侧窄验证命令：

```powershell
cargo check -p ospf-rust-framework-csp1d
cargo test -p ospf-rust-framework-csp1d
cargo check -p ospf-rust-framework-csp1d --features serde
cargo test -p ospf-rust-framework-csp1d --features serde render_schema_serializes_with_kotlin_camel_case_fields
cargo test -p ospf-rust-core remove_constraints_by_group_id --lib
```

已知后续工作：

- 将启发式 MILP/LP/最终 MILP 后端替换为真实 solver adapter。
- 用真实 LP dual 驱动 shadow price，而不是当前占位 dual。
- 继续补齐 Kotlin 生成器全量统计和大规模/并行缓存行为。
- 扩充 Kotlin fixture 对照、真实 solver warm-start smoke 和模块级 README。
