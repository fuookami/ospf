# ospf-rust-framework-network-scheduling

:us: [English](README.md) | :cn: 简体中文

`ospf-rust-framework-network-scheduling` 提供通用有向网络/流原语和 VRPTW 分支定价主链。本 README 是该 crate 长期维护的交付与验收合同。实现以 `ospf-kotlin/ospf-kotlin-framework-network-scheduling` 作为数学参考，并消费 workspace 的[统一求解合同](../docs/solve-contract_ch.md)。

## 交付状态

Network Scheduling 迁移已于 2026-08-14 正式关闭，`99/99` 项实现与验收全部完成。其依赖的公共 solver 合同迁移也已完成（`201/201`）：公共终态、证明门禁、取消、attempt 身份、remote report、CP/Logic-Based Benders 和 portable checkpoint 均由 core/framework 的正式 solver 文档所有，本 crate 不再定义平行合同。

收尾环境中已经执行的验证证据如下。这些是历史执行结论，不是需要提交到仓库的生成物。

| 门禁 | 已记录结果 |
| --- | --- |
| Network 默认 / `serde` / `big-decimal` 库测试 | `50/50`、`50/50`、`53/53` 通过 |
| Example 默认测试 | `8/8` 通过 |
| Network 格式与严格 Clippy（`--no-deps`） | 通过 |
| Gurobi Demo5 库内门禁 | `3/3` 通过：25 客户 direct-MIP、100 客户受限 smoke、100 客户 strict-proof |
| SCIP Demo5 库内门禁 | `1/1` 通过：25 客户合法终态复核 |
| Gurobi / SCIP Demo5 integration target | `3/3` / `2/2` 通过 |
| 100 客户 strict-proof fixture | direct-MIP 与 Branch-and-Price 均为 `Optimal`；目标、下界和上界均为 `100` |

## 架构

| 模块 | 职责 |
| --- | --- |
| `infrastructure` | 稳定图 ID、泛型节点/弧、容量/成本值对象和 solver 数值转换。 |
| `domain::flow` | 单/多商品流 context、守恒、容量、最小费用 pipeline 和小实例整数流 oracle。 |
| `domain::vrp` | VRPTW 实体、运行时单位、时间窗、路线校验、策略、定价对偶和分支遮罩。 |
| `domain::route_generation` / `domain::route_compilation` | ESPPRC 定价和增量 Phase I/Phase II 受限主问题。 |
| `application` | best-bound Branch-and-Price、节点限制、截止时间、取消、界、trace 和解组装。 |

application 层通过 `BranchNodeSolverProvider` 注入单节点求解器。模型注册保留在 route compilation context/aggregation 中，业务可以通过 `RouteCompilationExtension` 追加变量、约束、影子价格或解提取逻辑，而不改动 application 主循环。

## Public flow

1. 使用 `Quantity<V, Unit>` 和绝对 `TimeWindow<V>` 构造并校验 `VrptwInstance<V>`。
2. 选择距离、行驶时间、弧成本、路线成本和可选的弧可行性策略。
3. 使用注入的 `LinearProgrammingSolver` 或 core `LinearSolver` adapter 构造 `BranchNodeSolver`。
4. 调用 `VrptwApplicationService::solve`，读取统一的 `SolveReport<VrptwSolution<V>>`：使用 `problem_status`、`termination_reason`、`solution_presence`、`statistics` 和 `trace`，通过 `report.incumbent()` 读取已经复核的路线 incumbent。application 不再发布平行的 Branch-and-Price 终态结果。

受限主问题使用客户精确覆盖、有限车队上界、人工 Phase I 变量和 Phase II 真实路线成本。定价器实现 elementary ESPPRC，覆盖时间、负载、reduced cost、等待、支配、Feillet 不可达客户标记和 branch mask。路线签名保留有序弧 ID，因此平行弧不会被合并。

当前离线测试覆盖 graph/flow 合同、单位归一化、路线校验、ESPPRC 穷举对照、Phase I/II 生命周期、扩展 hook、失败路径、MRP 整数流 oracle、双车型 assignment/arc 分支和 BigDecimal 领域值。启用 `big-decimal` 后，显式基础弧可使用 `ExactBigDecimalEspprcPricer`，标签的时间、负载、成本和 reduced cost 全程保持 BigDecimal；隐式完全图仍明确拒绝进入该精确入口。Demo5 的 Gurobi 门禁已通过 25 客户 direct-MIP 对照、5 客户全路线 master oracle、受限的 Demo17 100 客户 smoke，以及目标值和上下界均为 100 的独立 100 客户严格证明 fixture；本机存在 native solver 时，SCIP 门禁通过 25 客户合法终态检查。

## 正确性合同

- flow 容量、供需与成本按显式运行时单位归一化，并在泛型 `V` 数值域校验商品平衡；重复 commodity ID、自环守恒错误、负容量下界和跨 `MetaModel` 复用都会被拒绝。
- `FlowContext` 与 route compilation 注册是事务性的。中途失败会恢复模型绑定、聚合索引、变量和生命周期状态，修正输入后可以向新的模型重新注册。
- 平行弧、有序 arc-ID 路线签名、客户侧 `RequireArc`、Phase I/II 切换、对偶/证明门禁、路线复核、六类终态和诚实上下界均有回归覆盖。
- f64 与 BigDecimal 定价图通过完整性指纹绑定实例、branch mask、dual snapshot、图内容和 reduced cost。标签搜索前会拒绝过期或公开数据篡改；可复用 f64 图的直接入口仅限 crate 内部，BigDecimal 还逐弧校验起点。
- 只有精确定价完成后的 LP 目标才能成为认证节点下界。定价中断只保留继承的有效下界，不可靠 dual/Farkas 证据不能驱动定价或精确剪枝。
- 小实例全路线 master 与 ESPPRC 穷举 oracle 均和 Branch-and-Price 一致。整数流 oracle 对受限 fixture 完整枚举共享容量、节点守恒和整数流，包括 MRP 形态的 inventory/production 最优解。
- 车型 assignment 和类型内 arc 分支同时约束已有列与后续定价。双车型 fixture 覆盖每类分支的两侧；取消、迭代耗尽、solver 失败、trace 失败和 solution-enricher 失败均保留结构化结果。
- serial combinatorial wrapper 在同步、异步 report 路径中都会消费预先存在或执行中的取消，不会继续启动后续 fallback solver。

## 扩展点

- `ArcFeasibilityPolicy` 在初始路线、定价图和最终校验中统一过滤静态弧。
- `RouteCostPolicy`、`DistanceCalculator`、`TravelTimeCalculator` 和 `ArcCostCalculator` 定义成本/时间口径。
- `LabelDominancePolicy` 和 `PricingColumnSelector` 定制 ESPPRC 行为。
- `RouteCompilationExtension` 通过标准生命周期扩展模型和结果提取。
- `TraceListener` 观察 Branch-and-Price 节点 trace。
- `CancellationToken`、deadline 和 `BranchAndPriceConfig` 控制中断与资源上限。

## 数值与序列化边界

领域物理量保持 `V` 泛型；flow 图在构造时归一化单位，并在进入 solver 转换前于 `V` 数值域校验每个商品的总供需。普通 solver 主模型、对偶、reduced cost、上下界和 trace 使用 `f64`。`NetworkSchedulingSolverValueAdapter<V>` 是普通 solver 主链的转换边界。启用 `big-decimal` 后，显式弧精确定价入口使用独立的 BigDecimal 对偶和标签类型，不经过该 f64 边界；测试覆盖两条数值路径。

可选 `serde` feature 覆盖可序列化的 ID、图 payload、分支元数据、对偶快照和诊断信息。运行时 `Quantity`/`Unit` 模型暂不派生 serde，因为 workspace 的 quantities crate 尚未定义无损的运行时单位序列化合同。

## 当前边界

- 普通 route generation、校验、master 注册、对偶、上下界和外部 solver adapter 使用 `f64`。任意精度 ESPPRC 仅由显式基础弧的 BigDecimal 入口提供；隐式完全图不冒充精确计算。
- Demo17 原始 100 客户用例仍是受时间/节点预算限制的 smoke，不宣称最优。严格证明使用独立、可手算的 100 客户 fixture；SCIP 25 客户门禁要求合法终态和独立路线复核。
- MRP oracle 是受限 fixture 的完整枚举器，不是生产规模求解器；双车型分支测试证明 branch-mask 行为，不代替大规模性能测试。
- Gurobi 与 SCIP 求解属于外部环境门禁。缺少 runtime 或许可证时必须明确失败或记为 unsupported，不能计入普通离线覆盖。

## 验证

```powershell
pwsh -NoProfile -Command "cargo test -p ospf-rust-framework-network-scheduling --no-default-features"
pwsh -NoProfile -Command "cargo test -p ospf-rust-framework-network-scheduling --features serde"
pwsh -NoProfile -Command "cargo test -p ospf-rust-framework-network-scheduling --features big-decimal"
pwsh -NoProfile -Command "cargo clippy -p ospf-rust-framework-network-scheduling --all-targets --no-default-features --no-deps -- -D warnings"
pwsh -NoProfile -Command "cargo fmt -p ospf-rust-framework-network-scheduling -- --check"
```

Demo5 的库内测试与 integration target 是两组不同门禁。下面的库内命令才会实际执行 direct-MIP、100 客户 smoke、strict-proof 和 SCIP 合法终态：

```powershell
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored"
```

`demo5_gurobi_bp` 和 `demo5_scip_bp` integration target 另外覆盖 target 自身的夹具/parser harness，不能替代上面的库内命令。缺少 native runtime 或许可证时，native gate 必须失败，不能计为通过。

不带 `--no-deps` 的完整 clippy 仍可用于整体检查，但当前会报告 `ospf-rust-base` 与 `ospf-rust-math` 中已有的警告。Gurobi/SCIP 执行仍由 feature 控制，并要求对应的本机运行时/许可证或 bundled 构建环境。

普通 route-generation 和 solver-facing 模型仍使用 `f64`，因为 `MetaModel`/外部 solver 的合同如此定义。需要任意精度路径时使用 `ExactBigDecimalRouteGraphBuilder` 与 `ExactBigDecimalEspprcPricer`，并提供显式基础弧；该入口不会把默认欧氏距离策略伪装成精确计算。

## 相关文档

- [English README](README.md)
- [统一求解合同](../docs/solve-contract_ch.md)
- [Solver 原生验收矩阵](../docs/solver-native-matrix_ch.md)
- [Solver source traceability](../docs/solver-traceability_ch.md)
- [Workspace README](../README_ch.md)
