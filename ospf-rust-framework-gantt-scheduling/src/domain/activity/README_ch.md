# 有限候选活动

[English](README.md) | 简体中文

本模块迁移 Kotlin 快照 `3f15899` 的有限候选活动排程和逐事件库存，独立于旧任务束列生成。
不可变准备、事务注册、独立原域校验和单次 MILP 编排共用 context 与生产提取路径。

## 输入准备

`ActivityPreparation::new(problem, adapter, additional_cost)` 接管输入所有权。
只读访问器暴露归一化输入、按稳定键排序的全部完整候选及显式取消费用，不创建或修改
`MetaModel`。可选 `ActivityCostPolicy` 在准备时对每完整候选调用一次并缓存结果；
策略应只读，不能承诺回滚任意外部副作用。

有限开始集合是完整业务域，不是搜索预算，不抽样、截断或静默裁剪。
必做活动无候选是合法输入，后续应形成模型不可行；声明候选越出 horizon 是输入错误。
零时长里程碑可位于 horizon 两端。Running 事实保留完整预计结束，即使超出 horizon，
其资源占用必须显式给出剩余部分。

模式与额外费用可为负；取消费和基线变更费必须非负。取消不再附加基线变更费。
冻结基线必须指向实际候选，非冻结基线允许在新候选域外。
即使无候选，资源、模式、物料、固定事件、事实、基线及扩展行也必须完整校验。
期初库存越界不是输入错误：期初在 horizon 起点事件之前，由模型判不可行，
不能用同点到货追补。

## 数值边界

公共量保留 `Quantity<V, Unit>`，在原 `V` 域归一化，费用同时保存原值与有限 solver
系数。可失败边界在运算前拒绝仿射单位，并检查量纲、非有限、非零下溢、自定义 adapter
非法零输出及日期运算溢出；合法线性温差仍支持，不从舍入后的 solver 系数重建原值。

自定义 `SchedulingSolverValueAdapter` 必须实现 `try_into_value` 和
`try_from_value`。新增 activity 代码不调用旧无错误返回方法；应以 `Err` 报告失败，
不捕获任意用户 panic。非 f64、自定义非恒等适配和单位转换保守标记
`solver_model_only()`。该标记本身不产生证明；有限候选最优不能外推为连续时间排程
或旧任务束列生成的全局最优。

`serde` 只覆盖候选键和语义枚举，不额外承诺运行时物理量与策略对象的通用序列化。

## 嵌入式生命周期

1. `ActivitySchedulingContext::new` 接收 adapter、可选缓存费用策略和注册配置，
   在修改模型前完成全部可失败的费用、行系数和右端转换。
2. 用 `register` 或 `register_with` 注册到 `MetaModel<f64>`。失败时模型与聚合暂存态
   一起回滚，可重试；成功后归属单模型，不能重复注册。闭包外部副作用不保证回滚。
3. 使用聚合暴露的源 token 变量与 `cost_expression` 追加规则或费用。
   本域只追加目标，不覆盖外部目标；`minimize_cost=false` 允许完全自定义目标。
4. 全部扩展完成后 `compile`。私有构造的 `ActivityCompiledModel` 保存真实完整矩阵、
   源结构/context/模型身份、目标 offset，以及按 VariableId 建立的源 token 到 solver 列映射。
5. 用 `compiled.solve(&solver, options)`，把私有构造的 `ActivitySolveExecution` 传给
   `extract_report`。每份编译记录只允许一次求解尝试，失败重试需重新编译；即使矩阵相等，
   也不能重放另一编译记录的执行结果。`extract_solution` 可审计裸报告并返回可行 payload，
   但不能提升任何证明。源模型变更后失效。
   提取入口自身检查完整向量、变量界、展开行、辅助整数性、维度、有限性和目标。
   不可把完整 solver 向量直接回填源 token：DEFERRED fallback 可能增加列。

`validate_activity_selection` 与 `ActivityPreparation::validate` 独立在原 V 域检查选择、
取消、冻结、四端点依赖、半开联合资源段、日历与扩展行。库存同点原子结算，期初先于
起点到货，包含 horizon 两端，不从执行事实推导历史物料流；不同物料不能互相抵偿。
原域比较不使用后端 `1e-7` 容差豁免微小业务违反。

## 单次 MILP 与证据作用域

`application::service::ActivitySchedulingApplicationService::solve` 接收新 context 工厂及
`FrameworkSolveOptions`；`solve_with` 提供事务扩展。同步和 async 入口均只调用一次既有
`ColumnGenerationSolver::solve_milp_report_with_options`，不调用 LP/定价或第二次全局求解。
正常不可行、限时无 incumbent、取消等保留 `GanttResult<SolveReport<ActivitySchedule<V>>>`；
非法输入/解与技术失败返回错误。零列 `Some(solution)` 不等于 `None`，仍保留事实和库存。

报告后端 objective/bound/gap 均为编译模型 `c^T x` 口径。`objective_offset()` 单独保存
按全局方向、子目标方向和权重归一化的声明常数；不能只给 objective 加常数而不处理 bound/gap。
`ActivitySchedule::cost` 从原 V 缓存重算业务费用，排除下游目标和事实沉没费用。

诊断扩展 `activity.solverModelReport.debug` 完整保留原 `SolveReport<f64>` 的 Debug 字符串，
只供审计，不是机器可读 SourceModel 证书。另有完整声明目标 `activity.declaredObjective`、`activity.objectiveOffset` 和
`activity.proofScope`。非 f64、单位转换或非恒等/有损 adapter 模型只转移原域复核后的可行
incumbent；清理顶层 proof、界、gap、逐轮证据、模型映射和诊断证据；无 incumbent 时为
`Unknown`/`None`。原 SolverModel 证据仅在审计扩展中保留。有限候选最优不证明连续时间域
或旧任务束列生成的全局最优。

兼容性与验收边界见[迁移说明](MIGRATION_ch.md)。
