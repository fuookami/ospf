# ospf-rust-framework-gantt-scheduling 迁移交接文档

## 1. 目标

将 `E:\workspace\ospf\ospf-kotlin\ospf-kotlin-framework-gantt-scheduling` 迁移到 Rust workspace 中的新 crate：

```text
E:\workspace\ospf\ospf-rust\ospf-rust-framework-gantt-scheduling
```

迁移目标不是逐文件翻译 Kotlin，而是用 Rust 风格复刻 Gantt Scheduling 领域框架能力，并严格对齐当前 Rust 项目的 framework 架构规范：

1. 以 `ospf_rust_core::model::MetaModel` 为模型装配轴心。
2. 以 context / aggregation / model component / pipeline 组织领域能力。
3. application 层只负责流程编排、solver 选择、状态映射、trace/KPI/render 组织，不直接堆积变量、约束、目标和 token 解析逻辑。
4. 普通 MILP、列生成 LP、列生成 final MILP 尽量共享同一套 context / aggregation / pipeline 注册路径。
5. public API 保持泛型数值和物理量边界，求解器适配边界内再落到 `f64`。

第一阶段目标是建立可编译、可测试、可扩展的 Rust 领域框架骨架，并完成基础设施、任务领域模型和普通任务编译 MILP 的最小闭环。完整 branch-and-price 可以作为后续阶段完成。

## 2. 当前前置条件评估

Rust 基础能力层已经具备迁移条件：

| 能力 | Rust 现状 | 迁移结论 |
| --- | --- | --- |
| 元模型 | `ospf_rust_core::model::MetaModel<V>` | 可承接 Kotlin `MetaModel<Flt64>` |
| 求解器入口 | `ospf_rust_framework::solver::ColumnGenerationSolver` | 可承接 MILP / LP / typed solve |
| 对偶解 | `LinearDualSolution`、`MetaDualSolution` | 可承接 shadow price 提取 |
| 管道扩展 | `Pipeline`、`CGPipeline` | 可承接 limits / objective / extractor |
| 影子价格 | `ShadowPriceMap`、`BasicShadowPriceMap` | 可作为 Gantt shadow price map 基础 |
| 物理量 | `Quantity<V, U>`、`UnitTrait`、`NoneUnit` | 可承接时间、产能、资源、产出量 |
| workspace | 已新增 `ospf-rust-framework-gantt-scheduling` | 可继续实现 |

主要缺口在 Gantt 领域 adapter 层：

1. 一维/二维变量集合封装。
2. 领域对象到变量 token 的稳定映射。
3. 线性中间表达式集合封装。
4. solution 结果提取。
5. 动态列生命周期中的 `add_columns` / `remove_columns` / `refresh_shadow_price` 统一协议。

## 3. Kotlin 子模块到 Rust 包映射

| Kotlin 子模块 | Rust 包路径 | 说明 |
| --- | --- | --- |
| `gantt-scheduling-infrastructure` | `crate::infrastructure` | 时间原语、时间窗口、工作日历 |
| `gantt-scheduling-domain-task-context` | `crate::domain::task` | 任务、执行者、分配策略、任务束、成本、影子价格参数 |
| `gantt-scheduling-domain-task-compilation-context` | `crate::domain::task_compilation` | 任务级 MILP、时间变量、完工时间、任务级 limits |
| `gantt-scheduling-domain-task-generation-context` | `crate::domain::task_generation` | 任务生成扩展点，占位优先 |
| `gantt-scheduling-domain-bunch-compilation-context` | `crate::domain::bunch_compilation` | 列生成主问题、任务束选择、束级解 |
| `gantt-scheduling-domain-bunch-generation-context` | `crate::domain::bunch_generation` | 定价问题、Graph、Label、任务束生成器 |
| `gantt-scheduling-domain-capacity-scheduling-context` | `crate::domain::capacity_scheduling` | 时隙产能分配、产能列、产能 limits |
| `gantt-scheduling-domain-resource-context` | `crate::domain::resource` | 执行/存储/连接资源、资源松弛、资源 limits |
| `gantt-scheduling-domain-produce-context` | `crate::domain::produce` | 产出、消耗、生产任务、产出/消耗 limits |
| `gantt-scheduling-application` | `crate::application` | APS/MPS/LSP 标记、迭代状态、列生成/分支定价编排 |

## 4. 推荐模块结构

当前 crate 已创建如下骨架，后续实现应在此基础上细化：

```text
src/
  lib.rs
  infrastructure/
    mod.rs
  domain/
    mod.rs
    task/
      mod.rs
      model.rs
    task_compilation/
      mod.rs
      model.rs
      service.rs
    task_generation/
      mod.rs
    bunch_compilation/
      mod.rs
      model.rs
      service.rs
    bunch_generation/
      mod.rs
      model.rs
      service.rs
    capacity_scheduling/
      mod.rs
      model.rs
      service.rs
    resource/
      mod.rs
      model.rs
      service.rs
    produce/
      mod.rs
      model.rs
      service.rs
  application/
    mod.rs
    model/
      mod.rs
      task.rs
      bunch.rs
    service/
      mod.rs
      task.rs
      bunch.rs
```

如实现增长较快，允许继续拆分为：

```text
domain/task/model/
domain/task_compilation/model/
domain/task_compilation/service/limits/
```

但对外 re-export 应保持稳定，避免调用方依赖内部文件拆分。

## 5. 迁移原则

### 5.1 不逐文件机械翻译

Kotlin 代码依赖 class hierarchy、lateinit、DSL operator、multiarray 下标和动态模型刷新。Rust 迁移应保留领域语义，但改用：

1. trait + struct 组合表达领域能力。
2. 显式 `Result` 返回错误。
3. 结构体配置或 builder 聚合复杂参数。
4. 明确所有权边界，避免大范围共享可变状态。

### 5.2 建模层遵循 framework 架构

领域建模组件必须通过 context / aggregation / model component / pipeline 注册到 `MetaModel`：

1. context：应用层入口，组装 aggregation 和 extra pipeline。
2. aggregation：组合多个 model component，协调注册顺序。
3. model component：持有变量、中间表达式、派生表达式和结果提取引用。
4. pipeline / limit：单一约束族、目标族、惩罚项或 shadow price 提取。

### 5.3 泛型和物理量边界

public API 优先使用泛型数值 `V` 和 `Quantity<V, U>`。只有 solver adapter、model registration、solution extraction 边界可以使用 `f64`。

时间、持续时间、产能、资源量、产出量不得在 public API 里以裸 `f64` 随意混算。

### 5.4 注释与文档

Rust 公共 API 注释必须中英双语，中文在前，英文在后。新增 README 时保持 `README.md` 与 `README_ch.md` 互链。

## 6. 阶段计划

### 阶段 0：基础校验与迁移矩阵

目标：确认 Kotlin 行为基线，避免迁移范围失控。

事项：

1. 读取 Kotlin 各子模块 README 和测试。
2. 建立迁移矩阵：Kotlin 类型、Rust 目标类型、迁移阶段、测试来源、是否第一版支持。
3. 明确第一版不支持项。

建议第一版不支持：

1. `IterativeTaskSchedulingTaskTime` 的 `withRedundancy` 路径。Kotlin 当前也抛 `UnsupportedOperationException`。
2. 完整 application 层 branch-and-price 的所有策略细节。
3. 复杂业务 DAG、复杂班次日历和下游自定义成本公式。

验收：

1. `gantt.md` 迁移矩阵补充完整。
2. 每个 Kotlin 子模块都有 Rust 目标包。
3. 不支持项在 README 或模块文档中明确说明。

### 阶段 1：基础设施层

目标：迁移 `gantt-scheduling-infrastructure`。

Kotlin 来源：

1. `TimeRange.kt`
2. `TimeWindow.kt`
3. `TimeSlot.kt`
4. `DurationRange.kt`
5. `WorkingCalendar.kt`
6. `LocalDateOffset.kt`
7. `dto/RenderTaskDTO.kt`

Rust 目标：

1. `crate::infrastructure::TimeRange`
2. `crate::infrastructure::TimeWindow<V>`
3. `crate::infrastructure::TimeSlot`
4. `crate::infrastructure::DurationRange`
5. `crate::infrastructure::WorkingCalendar`
6. `crate::infrastructure::LocalDateOffset`
7. `crate::infrastructure::RenderTaskDto`，如启用 serde 则挂 `serde` feature

重点设计：

1. 统一时间库。建议选 `time` 或 `chrono`，并在此阶段固定。
2. `TimeWindow<V>` 要提供从时间到 solver 数值边界的转换能力。
3. 连续时间和离散时间边界要显式表达，避免后续任务时间模型重复判断。

测试迁移：

1. `TimeWindowTest.kt`
2. `TimeRangeFindTest.kt`
3. `TimeRangeDifferenceTest.kt`
4. `WorkingCalendarTest.kt`

验收：

1. `cargo test -p ospf-rust-framework-gantt-scheduling infrastructure`
2. 时间范围交并差、查找、工作日历行为与 Kotlin 测试一致。
3. 无裸 `f64` 泄露到 public 时间 API，除非明确是 solver 数值边界。

### 阶段 2：任务领域模型

目标：迁移 `gantt-scheduling-domain-task-context` 的纯领域对象。

Kotlin 来源：

1. `Executor.kt`
2. `Assignment.kt`
3. `Task.kt`
4. `TaskPlan.kt`
5. `TaskBunch.kt`
6. `TaskStepGraph.kt`
7. `Cost.kt`
8. `ShadowPriceMap.kt`
9. `SchedulingSolverValueAdapter.kt`

Rust 目标：

1. `ExecutorTrait`
2. `AssignmentPolicyTrait<E>`
3. `TaskTrait<E, A>`
4. `TaskPlanTrait<E>`
5. `TaskBunch<T, E, A, V>`
6. `Cost<V>`
7. `GanttSchedulingShadowPriceMap`
8. `SchedulingSolverValueAdapter<V>`

重点设计：

1. Kotlin `AbstractTask` 的大量默认属性应拆成 trait 默认方法。
2. `TaskKey` 不要依赖 JVM class，Rust 可用 `TypeId` + `String` 或显式 `TaskType`。
3. `ManualIndexed` 迁移为稳定索引字段或 adapter 注册索引，不建议全局 mutable index。
4. `Cost<V>` 要保留多成本项、求解成本和业务成本的边界。

测试迁移：

1. `TaskQuantityFltXPathTest.kt`
2. `CostQuantityAlternativeTest.kt`

验收：

1. 任务提前、延迟、超最大提前、超最大延迟计算正确。
2. `TaskBunch` 的 busy time、total delay、total advance、executor change、contains 行为正确。
3. cost 可转换为 solver 目标系数，但 public API 保持泛型。

### 阶段 3：Gantt 建模 adapter

目标：补齐 Kotlin DSL 到 Rust `MetaModel` 的承接层。

必须实现的内部能力：

1. 一维变量集合，例如 `VariableArray1<K, T>`。
2. 二维变量集合，例如 `VariableArray2<K1, K2, T>`。
3. 领域 key 到变量 index/token 的双向映射。
4. 线性中间表达式集合。
5. 注册后结果提取 helper。
6. 列移除时的变量范围冻结与 token 状态更新。

建议位置：

```text
domain/common/
  variable_array.rs
  expression_array.rs
  solution_extraction.rs
  model_component.rs
```

如不想新增 `domain::common`，可先放入 `domain::task_compilation::model`，但后续 bunch/resource/produce 复用时应抽出。

验收：

1. adapter 能在测试中向 `MetaModel<f64>` 注册一维/二维二进制变量。
2. adapter 能注册中间线性表达式。
3. adapter 能从 solution vector 提取领域结果。
4. 不在 application 层直接暴露 token 解析细节。

### 阶段 4：任务编译普通 MILP

目标：迁移 `gantt-scheduling-domain-task-compilation-context` 的普通任务调度模型。

Kotlin 来源：

1. `Aggregation.kt`
2. `model/Compilation.kt`
3. `model/Switch.kt`
4. `model/TaskTime.kt`
5. `model/Makespan.kt`
6. `model/Solution.kt`
7. `service/SolutionAnalyzer.kt`
8. `service/limits/*`

Rust 目标：

1. `TaskCompilationAggregation`
2. `TaskCompilation`
3. `TaskSchedulingSwitch`
4. `TaskSchedulingTaskTime`
5. `Makespan`
6. `TaskSolution`
7. `SolutionAnalyzer`
8. `Pipeline<MetaModel<V>>` 或 Gantt 专用 pipeline traits

建模能力：

1. 任务分配变量 `x[task, executor]`。
2. 任务取消变量 `y[task]`。
3. 执行器空闲变量 `z[executor]`。
4. `task_assignment[task, executor]`。
5. `task_compilation[task]`。
6. `executor_compilation[executor]`。
7. 预估开始时间 `estimate_start_time[task]`。
8. 预估结束时间 `estimate_end_time[task]`。
9. 延迟、提前、超最大延迟、超最大提前。
10. 最晚结束延迟、最早结束提前。
11. on-time / not-on-time。
12. makespan。

重点风险：

1. Kotlin 使用 `SlackFunction`、`MaskingFunction`、`OrFunction`。Rust 侧需要确认 core 是否已有等价函数符号；没有则用线性化约束封装。
2. Kotlin `lateinit` 延迟初始化在 Rust 中应改为 `Option` 或 builder 初始化。
3. 多个 limits 同时注册目标时要统一目标方向和权重。

第一批 limits：

1. `TaskCompilationConstraint`
2. `ExecutorCompilationConstraint`
3. `TaskConflictConstraint`
4. `TaskTimeConflictConstraint`
5. `TaskExecutorCostMinimization`
6. `TaskCostMinimization`
7. `MakespanMinimization`
8. `TaskDelayTimeConstraint`
9. `TaskDelayTimeMinimization`
10. `TaskAdvanceTimeConstraint`
11. `TaskAdvanceTimeMinimization`

验收：

1. 最小任务-执行器样例可以注册完整模型。
2. task compilation 约束保证每个任务被分配或取消一次。
3. executor compilation 正确表达执行器是否被使用。
4. 时间相关变量范围与 Kotlin 逻辑一致。
5. solution analyzer 能输出已分配任务、取消任务、延迟/提前。

### 阶段 5：资源、产出、产能上下文

目标：迁移 `resource`、`produce`、`capacity_scheduling` 三类横向约束。

Resource 来源：

1. `ExecutionResource.kt`
2. `StorageResource.kt`
3. `ConnectionResource.kt`
4. `Resource.kt`
5. `ResourceSlack.kt`
6. `ResourceCapacityConstraint.kt`
7. `ResourceOverQuantityMinimization.kt`
8. `ResourceLessQuantityMinimization.kt`

Produce 来源：

1. `Produce.kt`
2. `Consumption.kt`
3. `ProductionTask.kt`
4. `ProduceSlack.kt`
5. produce / consumption quantity constraints and minimizations

Capacity 来源：

1. `CapacitySchedulingContext.kt`
2. `Aggregation.kt`
3. `Capacity.kt`
4. `CapacityColumn.kt`
5. `CapacityCompilation.kt`
6. `ExecutorCapacityConstraint.kt`
7. `OrderConstraint.kt`
8. `CapacityCostMinimization.kt`

验收：

1. resource quantity / slack 测试对齐 Kotlin。
2. produce / consumption quantity 测试对齐 Kotlin。
3. capacity column 基础测试对齐 Kotlin。
4. 所有数量 public API 使用 `Quantity<V, U>` 或明确领域量类型。

### 阶段 6：任务束编译与任务束生成

目标：迁移列生成主问题和定价问题。

Bunch compilation 来源：

1. `BunchCompilationContext.kt`
2. `Aggregation.kt`
3. `model/BunchAggregation.kt`
4. `model/SlotBasedBunch.kt`
5. `model/SlotBasedBunchAggregation.kt`
6. `model/BunchSchedulingSolution.kt`
7. `service/BunchSolutionAnalyzer.kt`
8. `service/TaskSolutionAnalyzer.kt`
9. `service/SlotBasedCapacityPreSolver.kt`
10. `service/limits/BunchCostMinimization.kt`

Bunch generation 来源：

1. `model/Graph.kt`
2. `model/Label.kt`
3. `service/SlotBasedBunchGenerator.kt`
4. `service/PlannedTaskBunchGenerator.kt`
5. `service/UnplannedTaskBunchGenerator.kt`

重点设计：

1. `TaskBunch` 是列生成中的列，应和 `add_columns` 生命周期强绑定。
2. `Label` 和 `Graph` 可先独立于 `MetaModel` 测试。
3. 定价问题应通过策略 trait 注入成本、冲突、资源可行性和 reduced cost。

验收：

1. `LabelGenerateBunchTest.kt` 对应 Rust 测试通过。
2. bunch solution summary 行为与 Kotlin 对齐。
3. Slot-based capacity pre-solver 能生成可用初始列。

### 阶段 7：迭代列生成生命周期

目标：迁移 iterative task/bunch compilation 的动态列能力。

必须覆盖生命周期：

1. `register`：注册初始变量、中间值、约束、目标。
2. `add_columns`：新增列时注册变量并刷新中间值、约束和目标。
3. `remove_columns`：移除列时同步更新变量范围、模型 token 和聚合状态。
4. `refresh_shadow_price` / `extract_shadow_price`：从 LP 对偶解提取影子价格。
5. `finalize` / `extract_solution`：从 solver solution 回填领域解。

Kotlin 来源：

1. `IterativeAggregation.kt`
2. `IterativeContext.kt`
3. `IterativeTaskCompilation`
4. `IterativeTaskSchedulingTaskTime`
5. application 中 `BranchAndPriceAlgorithm` 对 context 的调用方式

验收：

1. 初始列注册成功。
2. 新增列后模型变量数量增加，相关表达式刷新。
3. 移除列后变量范围冻结，解提取不再选择 removed column。
4. shadow price map 能通过 pipeline / context 提取。

### 阶段 8：application 层列生成与分支定价

目标：迁移 application 编排，不把领域建模逻辑写进 application。

Kotlin 来源：

1. `application/APS.kt`
2. `application/MPS.kt`
3. `application/LSP.kt`
4. `application/model/IterationSnapshot.kt`
5. `application/model/task/Iteration.kt`
6. `application/model/bunch/Iteration.kt`
7. `application/service/task/BranchAndPriceAlgorithm.kt`
8. `application/service/bunch/BranchAndPriceAlgorithm.kt`
9. `ColumnGenerationAlgorithm.kt`

Rust 目标：

1. `application::APS`
2. `application::MPS`
3. `application::LSP`
4. `Iteration`
5. `IterationSnapshot`
6. `TaskBranchAndPriceAlgorithm`
7. `BunchBranchAndPriceAlgorithm`
8. `TaskColumnGenerationAlgorithm`
9. `BunchColumnGenerationAlgorithm`

编排步骤：

1. 创建 `MetaModel`。
2. context register。
3. 注册初始列。
4. 求解初始 MILP。
5. 求解 RMP LP。
6. 提取 shadow price。
7. 调用定价问题生成新列。
8. add columns。
9. 固定/保留/隐藏列或执行器。
10. 最终 MILP。
11. solution analyzer 输出领域解。

验收：

1. application 不直接拼领域变量、约束和目标。
2. 所有可变业务规则通过 policy / pipeline / context 注入。
3. 迭代状态包含 LP/IP 目标、lower bound、optimal rate、运行时间、慢改进判断。
4. 支持 mock solver 或小规模真实 solver 的端到端测试。

## 7. 测试迁移清单

优先级 P0：

1. infrastructure 全部测试。
2. task cost / quantity 测试。
3. task compilation 基础变量注册测试。
4. task compilation solution summary 测试。
5. resource / produce quantity 测试。
6. capacity column / aggregation 测试。

优先级 P1：

1. bunch solution summary。
2. label generate bunch。
3. slot-based capacity result。
4. iterative add columns。
5. shadow price extraction。

优先级 P2：

1. full column generation。
2. full branch-and-price。
3. async solver feature。
4. gurobi/scip backend integration。

## 8. 验收命令

基础验收：

```powershell
cargo check -p ospf-rust-framework-gantt-scheduling
cargo test -p ospf-rust-framework-gantt-scheduling --no-run
cargo test -p ospf-rust-framework-gantt-scheduling --lib
```

涉及 async 路径：

```powershell
cargo check -p ospf-rust-framework-gantt-scheduling --features async
```

涉及 Gurobi：

```powershell
cargo test -p ospf-rust-framework-gantt-scheduling --features gurobi10
```

涉及 SCIP：

```powershell
cargo test -p ospf-rust-framework-gantt-scheduling --features scip
```

## 9. 完成定义

第一版完成定义：

1. crate 可编译并进入 workspace。
2. infrastructure 完整迁移并有测试。
3. task domain 完整迁移并有测试。
4. task compilation 普通 MILP 可注册、可求解、可提取解。
5. 至少 5 个核心 limits 以 pipeline 形式迁移。
6. README / README_ch 描述 public API、扩展点、暂不支持项。
7. `gantt.md` 中每个 P0 项都有完成状态。

完整迁移完成定义：

1. Kotlin README 范围内所有子模块均有 Rust 实现或明确不支持说明。
2. 普通 MILP、列生成 LP、列生成 final MILP 共享 context / aggregation / pipeline 注册路径。
3. branch-and-price 能跑通小规模端到端样例。
4. resource / produce / capacity / bunch generation 均有核心测试。
5. public API 无迁移期命名痕迹，不使用 `Generic` / `Typed` 等后缀表达迁移状态。
6. application 层没有大量直接 `model.add_*` 或 token 解析逻辑。

## 10. 迁移矩阵

### Phase 0：基础校验 ✅

| Kotlin 类型 | Rust 目标类型 | 迁移状态 | 测试来源 | 第一版支持 |
| --- | --- | --- | --- | --- |
| `TimeRange` | `crate::infrastructure::TimeRange` | ✅ 完成 | `TimeRangeDifferenceTest`, `TimeRangeFindTest` | 是 |
| `TimeWindow<V>` | `crate::infrastructure::TimeWindow<V>` | ✅ 完成 | `TimeWindowTest` | 是 |
| `TimeSlot` | `crate::infrastructure::TimeSlot` (trait) | ✅ 完成 | — | 是 |
| `DurationRange` | `crate::infrastructure::DurationRange` | ✅ 完成 | — | 是 |
| `WorkingCalendar<V>` | `crate::infrastructure::WorkingCalendar<V>` | ✅ 完成 | `WorkingCalendarTest` | 是（不含 Productivity） |
| `LocalDateOffset` | `crate::infrastructure::LocalDateOffset` | ✅ 完成 | — | 是 |
| `RenderTaskDTO` | `crate::infrastructure::dto::*` | ✅ 完成 | — | 是 |

### Phase 1：基础设施层 ✅

所有 P0 基础设施测试已迁移：

| Kotlin 测试 | Rust 测试 | 状态 |
| --- | --- | --- |
| `TimeRangeDifferenceTest` | `time_range::tests::test_difference_*` | ✅ |
| `TimeRangeFindTest` | `time_range::tests::test_find_*` | ✅ |
| `TimeWindowTest` | `time_window::tests::test_*` | ✅ |
| `WorkingCalendarTest` | `working_calendar::tests::test_*` | ✅（基础用例） |

### 第一版不支持项

| 项目 | 原因 |
| --- | --- |
| `IterativeTaskSchedulingTaskTime.withRedundancy` | Kotlin 也抛 `UnsupportedOperationException` |
| `Productivity` / `ProductivityCalendar` | 涉及 task domain 概念，移至 Phase 2+ |
| `WorkingCalendar` 条件连接时间 (`ConditionalConnectionTime`) | 低优先级，按需补充 |
| `find` 并行版本 | 可通过 `rayon` feature 后续支持 |
| `rsplit` 反向拆分 | 使用频率低 |
| 完整 application 层 branch-and-price | Phase 7-8 范围 |

### Phase 2-8：待实现

| Phase | 模块 | 状态 |
| --- | --- | --- |
| 2 | `domain::task` (TaskTrait, ExecutorTrait, Cost, ShadowPriceMap) | ⬜ 未开始 |
| 3 | Gantt 建模 adapter (VariableArray, ExpressionArray, SolutionExtraction) | ⬜ 未开始 |
| 4 | `domain::task_compilation` (普通 MILP) | ⬜ 未开始 |
| 5 | resource / produce / capacity_scheduling | ⬜ 未开始 |
| 6 | bunch_compilation / bunch_generation | ⬜ 未开始 |
| 7 | 迭代列生成生命周期 | ⬜ 未开始 |
| 8 | application 层列生成与分支定价 | ⬜ 未开始 |

## 11. 执行建议

下一个会话接手时，建议按以下顺序开始：

1. ~~运行 `cargo check -p ospf-rust-framework-gantt-scheduling` 确认骨架状态。~~ ✅
2. ~~先实现 `infrastructure`，迁移 Kotlin infrastructure 测试。~~ ✅
3. 再实现 `domain::task` 的 trait 和基础 struct。
4. 在实现 `task_compilation` 前先设计并测试变量/表达式 adapter。
5. 每完成一个阶段更新本文件清单状态，避免后续实现者重复盘点。
