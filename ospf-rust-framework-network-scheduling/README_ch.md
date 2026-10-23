# ospf-rust-framework-network-scheduling

[English](README.md)

通用有向网络、物理量化流模型、资源路径与 VRPTW 分支定价，复用 workspace 的
[统一求解合同](../ospf-rust-core/README_ch.md#统一求解合同)。

## 迁移状态

Kotlin `907d28d`、`3dc1225` 的迁移**尚未整体完成**。当前增量包含共享图/路径与
定价证据基础、不可变 `FlowProblem`、异量纲商品/资源、带生产校验的广义流、
共享建设、显式韧性、根树结构、运输分配、完整路径有限主问题、七个独立 service 和可选 Gantt/Network 桥接。
普通与 BigDecimal ESPPRC 已共用资源路径搜索内核及主问题/定价声明行提供器，覆盖两阶段的 12 类定价矩阵。
任务/束共享批次及 Demo4 的 Gantt 主模型、RMP、定价和最终 MILP 已接通，包含硬旅客行与实际非空 Gurobi 用例。
Demo4 的有界旅客取消/舱位改签、实际终点机队和原始束/锁定任务自动种子也共用该生命周期；
同 OD 航班改签以有界连接流绑定实际恢复版本，重复的旅客/机队占位建模 API 已删除；整体验收仍待完成。
详情见迁移说明和 Demo4 成对 README。
整体迁移验收仍未完成。历史测试数字不作为本轮验收依据。API 变更见[迁移说明](MIGRATION_ch.md)。

## 架构

| 模块 | 职责 |
| --- | --- |
| `infrastructure` | 稳定图 ID、泛型节点/弧、容量/成本值对象和 solver 数值转换。 |
| `domain::flow` | 单/多商品流 context、守恒、容量、最小费用 pipeline 和小实例整数流 oracle。 |
| `domain::design` | 不隐式创建流的共享建设组件、互斥总容量等级、设施、预算及有限流量联动。 |
| `domain::resilience` | 单组建设变量、独立显式场景流、最坏费用目标及结构连通见证。 |
| `domain::structure` | 仅复用建设的根有向树、可选 Steiner 中转、合格设施分区归属及数量配额。 |
| `domain::allocation` | 地点、非递减层级兼容服务、可分拆请求、共享配额和枢纽映射至同一 Flow/Design 内核。 |
| `domain::generalized_flow` | 单输入转换、有限投入容量、硬净供给、生产提取校验及下游表达式。 |
| `domain::path` | solver 无关的泛型资源路径、完整 trace、独立校验、预算及真实完成度。 |
| `domain::path_compilation` | 完整路径二元主问题、显式行单位、事务式列生命周期及 LP 来源校验对偶。 |
| `domain::vrp` | VRPTW 实体、运行时单位、时间窗、路线校验、策略、定价对偶和分支遮罩。 |
| `domain::route_generation` / `domain::route_compilation` | ESPPRC 定价和增量 Phase I/Phase II 受限主问题。 |
| `application` | 七领域独立 context 编排、best-bound Branch-and-Price、限制、取消、界、trace 和解组装。 |

application 层通过 `BranchNodeSolverProvider` 注入单节点求解器。模型注册保留在 route compilation context/aggregation 中，业务可以通过 `RouteCompilationExtension` 追加变量、约束、影子价格或解提取逻辑，而不改动 application 主循环。

### 共享定价行与每模型扩展

`RoutePricingRows<V>` 声明稳定行键、线性物理量单位、上下界和 `RouteRowCoefficient<V>`
提供器。同一不可变目录传给 `RouteCompilationContext::with_pricing_rows` 与
`RouteGraphBuilder::with_pricing_rows`（或 BigDecimal 精确 builder）；
`BranchNodeSolverConfig::pricing_rows` 自动连接两端。完整路线系数包含终点及非可加用量，
逐弧系数来自同一提供器。只有明确认证可加分解并通过完整路线独立回放复核，才启用支配。

全部声明行必须有价格，零对偶也不能省略。双侧行价格是带符号上下界对偶之和。组件管理
列系数缓存、Phase I 可行性人工变量、行刷新及删列重加；Phase II 将全部人工变量固定为零，
并计入真实路线费用。非空、客户不重复的路线由精确覆盖蕴含变量上界一，不再设置引入额外
列边界对偶的冗余显式上界。精确标签、系数、对偶及 reduced cost 保持 BigDecimal，
仅主问题 solver 注册边界允许舍入。

`BranchNodeSolverConfig::extension_factories` 替代跨模型共享扩展实例。
`RouteCompilationExtensionFactory::create` 为每个分支模型返回全新的 boxed 未注册对象；
直接 context 接受独占 boxed 扩展。注册、增删列、价格和提取 hook 均保留。证明性定价拒绝
未映射外部行（包括零价行）、受管目标/有效域篡改及价格 hook 篡改。参与定价的约束应放入
声明目录，不应通过旧 hook 注入无映射行。裸价格仍只用于启发式计算，不构成 LP 证书。

## Public flow

1. 使用 `Quantity<V, Unit>` 和绝对 `TimeWindow<V>` 构造并校验 `VrptwInstance<V>`。
2. 选择距离、行驶时间、弧成本、路线成本和可选的弧可行性策略。
3. 使用注入的 `LinearProgrammingSolver` 或 core `LinearSolver` adapter 构造 `BranchNodeSolver`。
4. 调用 `VrptwApplicationService::solve`，读取统一的 `SolveReport<VrptwSolution<V>>`：使用 `problem_status`、`termination_reason`、`solution_presence`、`statistics` 和 `trace`，通过 `report.incumbent()` 读取已经复核的路线 incumbent。application 不再发布平行的 Branch-and-Price 终态结果。

受限主问题使用客户精确覆盖、有限车队上界、人工 Phase I 变量和 Phase II 真实路线成本。定价器实现 elementary ESPPRC，覆盖时间、负载、reduced cost、等待、支配、Feillet 不可达客户标记和 branch mask。路线签名保留有序弧 ID，因此平行弧不会被合并。

当前离线测试覆盖 graph/flow 合同、单位归一化、路线校验、ESPPRC 穷举对照、Phase I/II 生命周期、扩展 hook、失败路径、MRP 整数流 oracle、双车型 assignment/arc 分支和 BigDecimal 领域值。启用 `big-decimal` 后，显式基础弧可使用 `ExactBigDecimalEspprcPricer`，标签的时间、负载、成本和 reduced cost 全程保持 BigDecimal；隐式完全图仍明确拒绝进入该精确入口。Demo5 保留独立的 direct-MIP、全路线 oracle、受限 smoke、strict-proof 和 SCIP 合法终态夹具；夹具存在不表示这些原生门禁已在本轮通过。

## 正确性合同

- flow 容量、供需与成本按显式运行时单位归一化，硬供需是否可行由模型判断，不将不平衡输入误报为输入错误；重复 commodity ID、自环守恒错误、负容量下界和跨 `MetaModel` 复用都会被拒绝。
- `FlowContext` 与 route compilation 注册是事务性的。中途失败会恢复模型绑定、聚合索引、变量和生命周期状态，修正输入后可以向新的模型重新注册。
- 平行弧、有序 arc-ID 路线签名、客户侧 `RequireArc`、Phase I/II 切换、对偶/证明门禁、路线复核、六类终态和诚实上下界均有回归覆盖。
- f64 与 BigDecimal 定价图通过完整性指纹绑定实例、branch mask、dual snapshot、图内容和 reduced cost。标签搜索前会拒绝过期或公开数据篡改；可复用 f64 图的直接入口仅限 crate 内部，BigDecimal 还逐弧校验起点。
- 只有精确定价完成后的 LP 目标才能成为认证节点下界。定价中断只保留继承的有效下界，不可靠 dual/Farkas 证据不能驱动定价或精确剪枝。
- 小实例全路线 master 与 ESPPRC 穷举 oracle 均和 Branch-and-Price 一致。整数流 oracle 对受限 fixture 完整枚举共享容量、节点守恒和整数流，包括 MRP 形态的 inventory/production 最优解。
- 车型 assignment 和类型内 arc 分支同时约束已有列与后续定价。双车型 fixture 覆盖每类分支的两侧；取消、迭代耗尽、solver 失败、trace 失败和 solution-enricher 失败均保留结构化结果。
- serial combinatorial wrapper 在同步、异步 report 路径中都会消费预先存在或执行中的取消，不会继续启动后续 fallback solver。

## 扩展点

- `PathCompilationPipeline` 参与注册、增删列、对偶刷新和提取。定价行必须在不可变问题中声明；未映射外部约束禁止定价，但不阻止嵌入式提取。
- `PathCompilationContext` 公开行/费用表达式和最终 `candidate_ids`；真实 LP 报告须求解 `pricing_model`，裸行价格仅用于显式启发式 reduced cost。详见[完整路径主问题合同](MIGRATION_ch.md#完整路径有限主问题)。
- `NetworkGraphAccess` 统一图与过滤/反向视图；拓扑算法从访问器读取端点，业务弧不因反转而改变。
- `PathPolicy<N, A, S>` 注入起始、转移、终止、完整校验、支配及排序；状态 `S` 可保持领域物理量和任意精度数值。
- 可选 `ospf-rust-framework-gantt-network` 将完整路径转换为业务束，普通 Gantt 不依赖 Network。
- framework 的 `VerifiedPricingDuals` 由当前 LP 模型的独立证书校验构造；`ColumnGenerationBatch` 只描述请求完成度，不能单独认证对偶来源或整数最优。

- `ArcFeasibilityPolicy` 在初始路线、定价图和最终校验中统一过滤静态弧。
- `RouteCostPolicy`、`DistanceCalculator`、`TravelTimeCalculator` 和 `ArcCostCalculator` 定义成本/时间口径。
- 两条 ESPPRC 数值路径共用 `PathSearchContext`，后续预算中断不会丢失已接纳的终点路线。`ExactBigDecimalRouteGraphBuilder::with_cost_policy` 注入完整 BigDecimal 路线费用，未经可加性认证时禁用支配。两个阶段都在完整终点重算 reduced cost 后筛选排序；输出限制不表示搜索不完整。
- `LabelDominancePolicy` 和 `PricingColumnSelector` 定制 ESPPRC 行为。
- `RouteCompilationExtension` 通过标准生命周期扩展模型和结果提取。
- `TraceListener` 观察 Branch-and-Price 节点 trace。
- `CancellationToken`、deadline 和 `BranchAndPriceConfig` 控制中断与资源上限。

## 数值与序列化边界

领域物理量保持 `V` 泛型；flow 图在构造时归一化单位，并在进入 solver 转换前于 `V` 数值域校验每个商品的总供需。普通 solver 主模型、对偶、reduced cost、上下界和 trace 使用 `f64`。`NetworkSchedulingSolverValueAdapter<V>` 是普通 solver 主链的转换边界。启用 `big-decimal` 后，显式弧精确定价入口使用独立的 BigDecimal 对偶和标签类型，不经过该 f64 边界；测试覆盖两条数值路径。

Flow 注册在写入模型前校验所有步长缩放后的容量、资源和费用系数，以及派生变量界，关闭 `minimize_cost` 也不绕过检查。非有限乘积/商和容量缩放下溢为零均返回错误，不改变模型或 aggregation；管道和下游表达式复用已校验的系数。

`NetworkTopology::shortest_path(&start, &target, &unit, cost)` 的回调返回 `Result<Quantity<V, Unit>>`，结果为 `ShortestPathResult<V>`，其中 `Found.cost` 使用指定单位。单位归一化、比较和累计均保留在 `V` 中，支持超出 f64 精度或范围的 BigDecimal/BigRational。原始单位和目标单位必须兼容且线性；非法单位、非有限权重或累计值返回错误。原有裸数值回调须包装为 `Quantity` 并显式传入单位，无量纲费用使用 `NoneUnit`。

可选 `serde` feature 覆盖可序列化的 ID、图 payload、分支元数据、对偶快照和诊断信息。运行时 `Quantity`/`Unit` 模型暂不派生 serde，因为 workspace 的 quantities crate 尚未定义无损的运行时单位序列化合同。

## 当前边界

- 普通 route generation、校验、master 注册、对偶、上下界和外部 solver adapter 使用 `f64`。任意精度 ESPPRC 仅由显式基础弧的 BigDecimal 入口提供；隐式完全图不冒充精确计算。
- Demo17 原始 100 客户用例仍是受时间/节点预算限制的 smoke，不宣称最优。严格证明使用独立、可手算的 100 客户 fixture；SCIP 25 客户门禁要求合法终态和独立路线复核。
- MRP oracle 是受限 fixture 的完整枚举器，不是生产规模求解器；双车型分支测试证明 branch-mask 行为，不代替大规模性能测试。
- Gurobi 与 SCIP 求解属于外部环境门禁。缺少 runtime 或许可证时必须明确失败或记为 unsupported，不能计入普通离线覆盖。

## 验证

```powershell
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --no-default-features"
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --features serde"
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 test -p ospf-rust-framework-network-scheduling --features big-decimal"
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 clippy -p ospf-rust-framework-network-scheduling --all-targets --no-default-features --no-deps -- -D warnings"
pwsh -NoProfile -Command "cargo fmt -p ospf-rust-framework-network-scheduling -- --check"
```

Demo5 的库内测试与 integration target 是两组不同门禁。下面的库内命令才会实际执行 direct-MIP、100 客户 smoke、strict-proof 和 SCIP 合法终态：

```powershell
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored"
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored"
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored"
pwsh -NoProfile -Command "cargo +nightly-2026-08-01 test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored"
```

`demo5_gurobi_bp` 和 `demo5_scip_bp` integration target 另外覆盖 target 自身的夹具/parser harness，不能替代上面的库内命令。缺少 native runtime 或许可证时，native gate 必须失败，不能计为通过。

不带 `--no-deps` 的完整 clippy 仍可用于整体检查，但当前会报告 `ospf-rust-base` 与 `ospf-rust-math` 中已有的警告。Gurobi/SCIP 执行仍由 feature 控制，并要求对应的本机运行时/许可证或 bundled 构建环境。

普通 route-generation 和 solver-facing 模型仍使用 `f64`，因为 `MetaModel`/外部 solver 的合同如此定义。需要任意精度路径时使用 `ExactBigDecimalRouteGraphBuilder` 与 `ExactBigDecimalEspprcPricer`，并提供显式基础弧；该入口不会把默认欧氏距离策略伪装成精确计算。

## 相关文档

- [English README](README.md)
- [统一求解合同](../ospf-rust-core/README_ch.md#统一求解合同)
- [Solver 原生验收矩阵](../ospf-rust-core/README_ch.md#solver-原生验收矩阵)
- [Solver source traceability](../ospf-rust-core/README_ch.md#source-追踪)
- [Workspace README](../README_ch.md)
