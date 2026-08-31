# Rust framework demo 改进计划

## 目标与边界

本计划用于继续推进 `ospf-rust-example/src/framework` 对 Kotlin 版
`E:/workspace/ospf-kotlin/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo`
的迁移对齐，同时吸收
`E:/workspace/fsra-proof/fsra-domain-bunch-generation-context/src/main/com/wintelia/fuookami/fsra/domain`
中的 `bunch_generation_context` 算法。

核心目标：

1. 删除已确认冗余的 `demo2/domain/airworthiness/service/limits/capacity_limit.rs`。
2. 将 Rust demo 的文件结构调整为更便于和 Kotlin 逐文件核对的结构。
3. 完整补齐 Demo4 `bunch_generation` 的列生成子问题算法，而不是只保留骨架。
4. 保持 `framework-architecture.md` 要求的 context / aggregation / model component / pipeline 边界。
5. 每个阶段都能通过明确的编译或局部验证命令回归。

非目标：

1. 不在本轮重写 solver adapter 或 framework 底层 API。
2. 不引入与迁移无关的大规模重构。
3. 不把 Kotlin 的类式结构机械照搬为不符合 Rust 模块系统的写法；`mod.rs`、snake_case 文件名、`pub use` 聚合保留为 Rust 必需差异。

## 执行优先级

| 优先级 | 工作项 | 目的 |
|---|---|---|
| P0 | 删除冗余 `capacity_limit.rs` 并清理引用 | 消除重复约束，避免同一 position 最大载重被注册两次 |
| P0 | 补齐 Demo4 `bunch_generation` 算法 | 这是目前最大功能缺口 |
| P1 | 对齐 Rust 文件结构，拆分过大的 `mod.rs` / 合并文件 | 降低后续 Kotlin/Rust 双向核对成本 |
| P1 | 处理 Rust-only 约束的归属 | 明确哪些是扩展点、哪些需要 Kotlin 同步拆分 |
| P2 | 文档、示例、测试补齐 | 固化迁移语义和回归边界 |

## P0-1 删除 Demo2 冗余容量约束

### 现状

`demo2/domain/airworthiness/service/limits/capacity_limit.rs` 与
`demo2/domain/stowage/service/limits/load_weight_limit.rs` 都在表达：

    sum(cargo_weight * x[c][p]) <= position.max_weight

该约束属于 stowage 的位置装载重量限制，不应重复出现在 airworthiness 中。

### 处理步骤

1. 删除文件：
   - `ospf-rust-example/src/framework/demo2/domain/airworthiness/service/limits/capacity_limit.rs`
2. 清理模块声明：
   - 从 `demo2/domain/airworthiness/service/limits/mod.rs` 移除 `capacity_limit`。
   - 从 `demo2/domain/airworthiness/service/pipeline_list_generator.rs` 移除对应调用。
   - 从任何 `use limits::{ ... }` 聚合导入中移除 `apply_capacity_limit` 或同名函数。
3. 确认唯一保留实现：
   - `demo2/domain/stowage/service/limits/load_weight_limit.rs`
4. 回归检查：
   - 使用 `rg "capacity_limit|CapacityLimit|apply_capacity" ospf-rust-example/src/framework/demo2` 确认没有悬挂引用。
   - 使用 `cargo check -p ospf-rust-example --features backend-gurobi` 验证 feature 下编译。

### 验收标准

1. `airworthiness` 不再注册 position 最大载重约束。
2. `stowage/load_weight_limit.rs` 是该约束的唯一注册入口。
3. 编译无错误。

## P0-2 补齐 Demo4 bunch_generation

### 参考来源

FSRA 完整参考模块：

1. `bunch_generation_context/Aggregation.kt`
2. `bunch_generation_context/BunchGenerationContext.kt`
3. `bunch_generation_context/model/Graph.kt`
4. `bunch_generation_context/model/FlightTaskReverse.kt`
5. `bunch_generation_context/service/Operator.kt`
6. `bunch_generation_context/service/FlightTaskFeasibilityJudger.kt`
7. `bunch_generation_context/service/RouteGraphGenerator.kt`
8. `bunch_generation_context/service/FlightTaskBunchGenerator.kt`
9. `bunch_generation_context/service/InitialFlightTaskBunchGenerator.kt`
10. `bunch_generation_context/service/AggregationInitializer.kt`

Rust 当前目录：

1. `demo4/domain/bunch_generation/aggregation.rs`
2. `demo4/domain/bunch_generation/mod.rs`
3. `demo4/domain/bunch_generation/model/mod.rs`
4. `demo4/domain/bunch_generation/service/mod.rs`

当前 Rust 只有骨架，应拆分并补齐算法。

### 目标文件结构

新增或调整为：

1. `demo4/domain/bunch_generation/context.rs`
2. `demo4/domain/bunch_generation/aggregation.rs`
3. `demo4/domain/bunch_generation/model/mod.rs`
4. `demo4/domain/bunch_generation/model/graph.rs`
5. `demo4/domain/bunch_generation/model/flight_task_reverse.rs`
6. `demo4/domain/bunch_generation/service/mod.rs`
7. `demo4/domain/bunch_generation/service/operator.rs`
8. `demo4/domain/bunch_generation/service/flight_task_feasibility_judger.rs`
9. `demo4/domain/bunch_generation/service/route_graph_generator.rs`
10. `demo4/domain/bunch_generation/service/flight_task_bunch_generator.rs`
11. `demo4/domain/bunch_generation/service/initial_flight_task_bunch_generator.rs`
12. `demo4/domain/bunch_generation/service/aggregation_initializer.rs`

`mod.rs` 只保留模块声明和必要 `pub use`，不承载核心逻辑。

### 数据模型迁移

#### graph.rs

实现 FSRA `Graph.kt` 对应结构：

1. `Node` 枚举：
   - `RootNode`
   - `TaskNode`
   - `EndNode`
2. `Edge`：
   - 起点、终点。
   - 连接成本或连接时间引用。
   - 是否来自任务对换的标记。
3. `Graph`：
   - 节点集合。
   - 边集合。
   - 从节点查出边。
   - BFS / DFS 所需的邻接查询。

注意：

1. 节点 id 要稳定，不能依赖临时 Vec 下标表达业务身份。
2. 与 `demo4/domain/task/model` 中的 `FlightTask` / `FlightTaskBunch` 保持引用关系，不重复定义任务实体。
3. 若需要 HashMap key，优先定义小型 key struct，避免裸 tuple 在多个服务中散落。

#### flight_task_reverse.rs

实现 FSRA `FlightTaskReverse.kt`：

1. `ReversiblePair`：
   - 原任务。
   - 可对换任务。
   - 对换后时间、机场、成本调整相关字段。
2. `FlightTaskReverse`：
   - 支持按任务查找可对换集合。
   - 支持 `reverse_enabled` 判断。
   - 支持 `symmetrical` 判断。
   - 支持双向 pair 查询。

迁移注意：

1. 对换判断不要写死在 route graph generator 中，应由该模型或 strategy 提供。
2. 对换规则需要保留扩展点，便于后续按机型、航线、机场、维护规则扩展。

### 服务层迁移

#### operator.rs

迁移 FSRA `Operator.kt` 中的函数类型别名，Rust 使用 trait 或 boxed closure 表达：

1. `RuleChecker`
2. `ConnectionTimeCalculator`
3. `MinimumDepartureTimeCalculator`
4. `CostCalculator`
5. `TotalCostCalculator`
6. `FeasibilityJudger`

建议：

1. 轻量、无状态的计算可使用泛型闭包参数。
2. 需要在 aggregation/context 中长期保存的策略使用 `Arc<dyn Trait + Send + Sync>`。
3. 避免在 application 层直接拼接具体业务判断。

#### flight_task_feasibility_judger.rs

实现 FSRA `FlightTaskFeasibilityJudger.kt` 的可行性检查：

1. 机型匹配。
2. 子机型或机型族匹配。
3. 容量匹配。
4. 飞机可用性。
5. 起降机场衔接。
6. 飞行时间和过站时间。
7. 任务时间窗。
8. 维护 / AOG / recovery 规则。
9. rule context 的额外业务规则。
10. 对换任务的特殊可行性判断。

验收标准：

1. 可行性判断以策略注入为主。
2. `route_graph_generator` 只调用判断结果，不硬编码规则细节。
3. 能对不可行原因保留诊断字段，至少便于测试断言。

#### route_graph_generator.rs

实现 FSRA `RouteGraphGenerator.kt`：

1. 为每架 aircraft 构建 root -> task -> end 的有向图。
2. BFS 或队列式扩展候选边。
3. 对每个任务连接调用 feasibility judger。
4. 支持 `with_order_change`：
   - 构造反向任务边。
   - 将 `FlightTaskReverse` 中允许的对换关系加入图。
5. 调用 connection time / minimum departure time 计算器。
6. 输出每架 aircraft 的 `Graph`。

注意：

1. 图生成只负责候选路径空间，不直接注册优化变量。
2. 图生成结果进入 `Aggregation`，供初始列和 pricing 子问题复用。

#### flight_task_bunch_generator.rs

实现 FSRA `FlightTaskBunchGenerator.kt` 的 Label Setting pricing 算法：

1. 输入：
   - aircraft。
   - route graph。
   - shadow price map。
   - cost calculators。
   - delay / recovery 参数。
   - reduced cost 阈值。
2. Label 状态至少包含：
   - 当前节点。
   - 已访问任务序列。
   - 当前时间。
   - 原始成本。
   - shadow price 扣减。
   - reduced cost。
   - delay / recovery 信息。
3. 扩展规则：
   - 从当前节点沿出边扩展。
   - 更新起飞时间、到达时间、成本。
   - 叠加任务 shadow price。
   - 对不可行扩展产生诊断但不输出列。
4. 支配规则：
   - 同 aircraft、同末端节点、同任务集合或可比较状态下，成本和时间均不劣者保留。
   - 支配逻辑必须单独封装，方便测试。
5. 输出：
   - reduced cost 小于阈值的 `FlightTaskBunch`。
   - 可选 top N 限制。
   - 定价诊断，如扩展 label 数、剪枝 label 数、输出列数。

验收标准：

1. 能在 branch-and-price 迭代中作为新增列来源。
2. shadow price 刷新后不需要重建全部静态图。
3. 单元测试覆盖负 reduced cost 列生成、无可行列、支配剪枝三类场景。

#### initial_flight_task_bunch_generator.rs

实现 FSRA `InitialFlightTaskBunchGenerator.kt`：

1. 为每架 aircraft 构造初始 bunch。
2. 支持 locked task。
3. 支持 soft recovery。
4. 支持 empty bunch。
5. 输出初始列集合并写入 aggregation。

验收标准：

1. 没有 pricing 结果时仍能生成最小可行初始列。
2. locked task 不会被遗漏。
3. empty bunch 的成本、覆盖关系和选择变量语义清晰。

#### aggregation_initializer.rs

实现 FSRA `AggregationInitializer.kt`：

1. 初始化 `FlightTaskReverse`。
2. 按 aircraft 构造 route graph。
3. 生成初始 bunch。
4. 将 graphs、initial bunches、reverse map 写入 `Aggregation`。
5. 如使用并行，优先检查项目现有依赖；没有明确收益时先单线程实现，保留扩展点。

验收标准：

1. application 只调用 initializer / context，不直接构造 graph 和 bunch。
2. 初始化失败能返回明确错误，而不是静默跳过 aircraft。

### context 与 aggregation 集成

#### context.rs

新增 `BunchGenerationContext`：

1. 接收 aircraft、task、rule、parameter、cost strategy。
2. 负责调用 `AggregationInitializer`。
3. 对外提供：
   - 初始 bunch 查询。
   - pricing 生成新 bunch。
   - shadow price 刷新入口。
4. 不直接注册 master model 约束；约束仍由 `bunch_selection` / `bunch_compilation` 负责。

#### aggregation.rs

扩展 `Aggregation`：

1. `flight_task_reverse`
2. `graphs_by_aircraft`
3. `initial_bunches`
4. `generated_bunches`
5. `pricing_diagnostics`
6. `parameters`

若字段公开，按项目规则补齐中英双语文档注释。

### 与 Demo4 其他模块的关系

1. `task/model/flight_task_bunch.rs` 应作为 bunch 输出实体，避免在 bunch_generation 中重复定义。
2. `rule/service/*` 的连接时间、成本、可行性计算要通过 operator 注入。
3. `bunch_selection/service/branch_and_price_algorithm.rs` 应从 `BunchGenerationContext` 获取新列。
4. `bunch_compilation` 只负责编译约束和容量约束，不承担 route graph 或 pricing 逻辑。

## P1-1 Rust 文件结构对齐

### 原则

1. Kotlin 中一个承担核心业务职责的类，Rust 尽量有一个同名 snake_case 文件对应。
2. `mod.rs` 只做模块声明、`pub use` 和极少量 glue code。
3. 不强制为纯 DTO 或非常小的 enum 拆文件，但当 Kotlin 已是独立文件且逻辑超过简单数据定义时应拆。

### Demo1 拆分

#### bandwidth_context/service

当前：

1. `service/mod.rs` 中包含 `generate_pipelines`。

目标：

1. 新增 `service/pipeline_list_generator.rs`。
2. `mod.rs` 声明模块并 re-export `generate_pipelines`。
3. 保持调用方 API 兼容。

#### route_context/model

当前：

1. `route_context/model.rs` 合并 `Assignment`、`Graph`、`Service`。

目标：

1. 改为 `route_context/model/mod.rs`。
2. 新增 `route_context/model/assignment.rs`。
3. 新增 `route_context/model/graph.rs`。
4. 新增 `route_context/model/service.rs`。
5. 更新引用路径为 `route_context::model::{Assignment, Edge, Node, Service}`。

#### route_context/service

目标：

1. 新增 `route_context/service/pipeline_list_generator.rs`。
2. `mod.rs` 只保留模块声明和 re-export。

### Demo2 拆分

#### airworthiness 命名

Kotlin 路径是 `airworthiness_security`，Rust 当前是 `airworthiness`。

计划：

1. 优先评估是否将 Rust 目录改为 `airworthiness_security`。
2. 如果重命名成本可控：
   - 移动目录。
   - 更新 `demo2/domain.rs`、`demo2/domain/mod.rs`、`pipeline.rs`、各 application 引用。
   - 短期可保留 `pub mod airworthiness_security;`，不保留旧 `airworthiness` 兼容层，避免迁移期双名长期存在。
3. 如果短期不重命名：
   - 在 `daily` 后续执行记录中明确偏差原因。
   - 文档中说明 Rust 缩名与 Kotlin 全名的对应关系。

建议执行重命名，因为用户已明确文件结构不按 Kotlin 也算改进项。

#### airworthiness/service/limits

1. 删除 `capacity_limit.rs`。
2. 保留 Kotlin 已有对应项：
   - `ballast_weight_limit.rs`
   - `clim_limit.rs`
   - `cumulative_limit.rs`
   - `envelope_limit.rs`
   - `horizontal_stabilizer_limit.rs`
   - `linear_density_limit.rs`
   - `low_payload_limit.rs`
   - `payload_limit.rs`
   - `surface_density_limit.rs`
   - `total_weight_limit.rs`
   - `unsymmetrical_linear_density_limit.rs`
   - `zone_load_weight_limit.rs`
3. `adjacent_gap_limit.rs` 属于 Rust-only 扩展：
   - 若业务确认需要，Kotlin 也新增同名 `AdjacentGapLimit.kt`。
   - 若业务确认不需要，再删除 Rust 文件。
   - 当前计划先标记为待确认扩展，不与 `capacity_limit.rs` 一起删除。

#### mac/model

当前 Rust `mac/model/mod.rs` 合并多个模型。

目标：

1. `mac/model/torque.rs`
2. `mac/model/mac.rs`
3. `mac/model/horizontal_stabilizer.rs`
4. `mac/model/mod.rs` re-export。

#### payload_maximization

目标：

1. 若 `service/mod.rs` 或 `limits/mod.rs` 承载实际约束逻辑，拆出：
   - `payload_maximization/context.rs`（如存在 context 语义）
   - `payload_maximization/service/pipeline_list_generator.rs`
   - `payload_maximization/service/limits/payload_maximization_limit.rs` 或与 Kotlin 对齐的命名。
2. application 只调用 context / pipeline generator。

#### recommended_weight_equalization

目标：

1. 拆出 model 文件，避免 `model/mod.rs` 混合定义。
2. 拆出 limit 文件，避免 `service/limits/mod.rs` 放业务逻辑。
3. 与 Kotlin 对应文件逐项建表核对。

#### soft_security / loading_effectiveness / express_effectiveness

Rust-only 约束处理原则：

1. `must_ship_limit.rs`
2. `priority_order_limit.rs`
3. `source_early_limit.rs`

这些如果是业务合理增强，则 Kotlin 同步新增独立 limit 文件和 pipeline 注册；Rust 保留。
如果它们只是迁移误加且与既有 Kotlin 约束重复，再单独删除。
本计划不在未确认前删除，避免误删业务能力。

### Demo4 拆分

#### bunch_compilation

当前 Rust：

1. `model/mod.rs`
2. `service/limits/mod.rs`

目标对齐 Kotlin：

1. `model/compilation.rs`
2. `model/fleet_balance.rs`
3. `model/flight_capacity.rs`
4. `model/flight_link.rs`
5. `service/free_aircraft_selector.rs`
6. `service/pipeline_list_generator.rs`
7. `service/limits/fleet_balance_limit.rs`
8. `service/limits/flight_link_limit.rs`

同时保持 `context.rs`、`aggregation.rs` 作为 Rust/Kotlin 已有结构对应。

#### passenger / crew / rule / task

逐项检查 `model/mod.rs` 是否合并了 Kotlin 独立文件：

1. 合并超过两个业务实体时拆分。
2. 每次拆分后跑 `cargo check -p ospf-rust-example --features backend-gurobi`。
3. 避免一次性移动过多导致路径错误难定位。

## P1-2 Rust-only 约束归属策略

### 删除类

已确认删除：

1. `airworthiness/service/limits/capacity_limit.rs`

删除条件：

1. 与另一个 domain 的同一硬约束完全重复。
2. 注册后会导致模型语义重复或收紧。
3. Kotlin 没有对应且业务上也无独立意义。

### 同步到 Kotlin 类

计划让 Kotlin 同步拆分或新增：

1. `AdjacentGapLimit`
2. `MustShipLimit`
3. `PriorityOrderLimit`
4. `SourceEarlyLimit`

同步条件：

1. Rust 实现不是重复约束。
2. 对应业务规则在 Kotlin 当前 `PipelineListGenerator` 中没有等价实现。
3. 该规则可以作为独立 limit 接入，不破坏 application 编排边界。

### 命名差异类

不强制删除：

1. Rust `balance_limit.rs` 如果只是聚合了 Kotlin `LongitudinalBalanceLimit` 和 `LateralBalanceLimit`，建议拆为两个文件以便对照。
2. Rust `cumulative_limit.rs` 与 Kotlin `CumulativeLoadWeightLimit.kt` 是命名差异，建议改为 `cumulative_load_weight_limit.rs` 或在 `pub use` 中保留清晰别名。

## P2 文档与测试

### 文档

1. 为 `ospf-rust-example/src/framework/demo4` 补充 README 或在现有 README 中说明：
   - bunch generation 的输入输出。
   - route graph 与 pricing 的关系。
   - shadow price 如何参与 reduced cost。
   - initial bunch 与 generated bunch 的区别。
2. README / README_ch 如新增，按项目规则互相链接。

### 测试

建议新增最小测试夹具：

1. `route_graph_generator`：
   - 两个可连接任务生成边。
   - 不可连接任务不生成边。
   - with_order_change 生成反向边。
2. `flight_task_bunch_generator`：
   - shadow price 使 reduced cost 为负时输出列。
   - 无负 reduced cost 时不输出列。
   - dominated label 被剪枝。
3. `initial_flight_task_bunch_generator`：
   - locked task 必须被覆盖。
   - empty bunch 可生成。
4. `airworthiness`：
   - 删除 `capacity_limit.rs` 后，position max weight 只由 stowage 注册一次。

如果 binary crate 不方便写集成测试，先添加 `#[cfg(test)] mod tests` 到对应 service 文件或拆分出的纯逻辑模块。

## 回归命令

每个阶段至少执行：

    cargo check -p ospf-rust-example --features backend-gurobi

涉及全局路径重命名后执行：

    cargo test -p ospf-rust-example --features backend-gurobi

快速检索悬挂引用：

    rg "capacity_limit|CapacityLimit|apply_capacity" ospf-rust-example/src/framework
    rg "airworthiness::|airworthiness_security" ospf-rust-example/src/framework/demo2
    rg "bunch_generation" ospf-rust-example/src/framework/demo4

## 建议执行顺序

1. 删除 `capacity_limit.rs`，清理引用并编译。
2. 拆 Demo4 `bunch_generation` 文件结构，不先实现复杂算法，只移动骨架并保持编译。
3. 迁移 FSRA graph 和 flight task reverse 模型，添加局部测试。
4. 迁移 feasibility judger 和 route graph generator，添加局部测试。
5. 迁移 initial bunch generator，接入 aggregation initializer。
6. 迁移 label setting pricing，接入 branch-and-price。
7. 拆 Demo4 `bunch_compilation` 的 model / limit 文件。
8. 拆 Demo1、Demo2 过大的合并文件。
9. 处理 Rust-only 约束：保留并推动 Kotlin 同步，或经确认后删除。
10. 补文档和最终全量回归。
