# 任务束编译

:us: [English](README.md) | :cn: 简体中文

本目录包含任务束编译的主问题侧实现，对应 Kotlin `gantt-scheduling-domain-bunch-compilation-context`。

## 职责

- 向 `MetaModel` 注册任务束决策变量和聚合表达式。
- 为列生成流程维护迭代任务束列。
- 从 solver 结果中提取任务和任务束排程解。
- 提供时隙级编译和产能预求解辅助能力。
- 暴露定价子问题使用的影子价格 pipeline。

## 模块

- `model.rs`：任务束编译、任务束聚合、entry、解和解摘要。
- `context.rs`：基础与迭代任务束编译上下文。
- `iterative.rs`：迭代任务束编译状态和列操作。
- `master.rs`：可选的标准任务覆盖、执行器选择和自有目标组件。
- `validation.rs`：实际后端向量、整数、行界及报告目标的生产校验。
- `slot_based.rs`：时隙级任务束编译上下文和产能预求解器。
- `service.rs`：解分析和服务辅助。

## 公共 API

- `BunchCompilation`
- `BunchAggregation`
- `BunchEntry`
- `BunchSolution`
- `BunchSolutionSummary`
- `IterativeBunchCompilation`
- `BunchMasterConfiguration` 与 `BunchExecutorSelection`
- `BasicBunchCompilationContext`
- `IterativeBunchCompilationContext`
- `BunchShadowPricePipeline`
- `TaskShadowPriceKey`
- `SlotBasedBunchCompilationContext`
- `BasicSlotBasedBunchCompilationContext`
- `SlotBasedCapacityPreSolver`
- `StaticSlotBasedCapacityPreSolver`

## 扩展点

主问题行为通过 bunch compilation context、迭代任务束列操作、时隙级产能预求解器和 shadow-price pipeline 扩展。pricing 规则保留在 `bunch_generation`，任务级变量语义保留在 `task_compilation`。

## 生命周期与数据流

compilation context 将初始任务束列和聚合表达式注册到 `MetaModel`，iterative context 在列生成过程中新增或刷新列，shadow-price pipeline 将对偶值暴露给 pricing，solution analyzer 从最终 solver value 中提取任务和任务束排程。

`BunchEntry::id` 是完整业务候选的稳定身份，不是执行器/任务覆盖集合的散列。覆盖同一批任务的
不同恢复方案必须使用不同 ID，调用方须将 ID 绑定到不可变的完整 payload。相同 ID 只有在执行器、
有序任务索引、时隙及归一化费用全部一致时才幂等，冲突使整批失败。`index` 是独立的历史模型列索引：
退休索引不能复用，退休候选以相同业务 ID 重加时必须使用新索引和新变量。

注册只对原模型及未改变的编译定义幂等，跨模型使用失败。注册、加列、带模型的退休操作同时对
context 与 `MetaModel` 实施事务。`remove_columns(indices, model)` 和
`apply_lifecycle(lifecycle, model)` 现在返回 `GanttResult<()>`；退休会固定旧 token 为零、清理
其目标项、重建符号并刷新受管主问题。`next_column_index()` 包含退休历史，时隙批次整体提交，
不再逐时隙留下部分成功结果。

注册前调用 `context.compilation.configure_master(BunchMasterConfiguration {
cancellation_costs, executor_selection })` 可启用标准主问题。每个任务由所选任务束或取消变量恰好
覆盖一次；取消费用 `None` 表示禁止取消，缺少覆盖保留为正常模型不可行。执行器选择明确区分
`AtMostOne` 与 `ExactlyOne`，包含已启用的空闲变量。组件拥有独立约束组及命名最小化子目标，
真实增删列时刷新它们，保留无关的下游约束和目标项。独立二元穷举测试直接检查编译后的后端矩阵。

既有 `service::limits::*::from_compilation` 管道仍是**静态注册快照**，并非实时更新订阅；删列后
查找变量也使用稳定映射。动态自定义行仍需下游显式生命周期 hook；退休变量固定为零，但不会为任意
外部规则自动推导新增列系数。标准主问题本身不认证定价完成度或整数最优，也不代表 Demo4 应用闭环已完成。

## 生产提取与求解方向

`extract_solution_checked(model, values)` / `analyze_solution_checked(model, values)` 校验完整当前
后端矩阵，包括下游追加约束、变量边界、有限性、维度和整数性。它们校验受管变量映射，拒绝非零退休列；
取消任务只从实际 `y` 变量读取，不再把未覆盖任务猜作取消。
`analyze_solution_with_lifecycle_checked` 还拒绝未选中的固定列和已选中的隐藏/退休列，不通过强插列修补结果。
原有不带模型的提取函数仅适用于未校验的预览，不能替代生产入口。

`validate_bunch_incumbent` 可用于 solver adapter 边界，并核对报告目标与实际 `c * x`，拒绝非有限
系数和运算溢出；LP 调用方必须显式传入松弛模型。
`BunchBranchAndPriceAlgorithm::solve_milp_report` 保留正常不可行、限时无 incumbent 和取消报告，
技术失败及非法 incumbent 返回错误；原有 `run` 仍要求获得可用 incumbent，不是新的统一独立服务入口。

任务束算法按已注册模型的实际目标方向重置 `Iteration`，最小化允许零值与负值继续改善。
初始目标为零不意味着定价收敛；最终 MILP 错误不会被已有 incumbent 吞掉。
保留较早 incumbent 时，先扩展向量并在当前模型及生命周期重验，再生成无旧证明的对应报告，
可通过 `incumbent_report()` 只读访问；不能把旧模型报告和新解维度混用。
只有通过生产校验的 incumbent 才更新相应原始界；LP 目标历史不自动构成完整定价域的对偶界。

## 验证

先运行 `cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-scheduling --no-default-features -- --list`，
再运行移除 `-- --list` 的相同命令。`serde`、`async`、`big-decimal`、`big-rational` 分别验证，
仅列出测试不能作为执行证据。

## 相关目录

- [`../bunch_generation`](../bunch_generation/README_ch.md)
- [`../task`](../task/README_ch.md)
- [`../../application/algorithm`](../../application/algorithm/README_ch.md)
