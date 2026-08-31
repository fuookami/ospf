# CSP1D 迁移交接

## 当前状态

本轮迁移已在 Rust workspace 中新增 `ospf-rust-framework-csp1d` crate，并把它加入根 `Cargo.toml` 的 workspace members。当前 crate 已能通过 `cargo check -p ospf-rust-framework-csp1d` 与 `cargo test -p ospf-rust-framework-csp1d`，CSP1D crate 自身 warning 已清理；剩余 warning 来自上游 `ospf-rust-math` 和 `ospf-rust-quantities` 的既有问题。

本轮工作主要覆盖 CSP1D 迁移相关文件，并在 `ospf-rust-core` 中补了支撑动态建模刷新的约束移除 API。工作树中已有的 BPP3D 改动未触碰。

## 已完成事项

- 建立 Rust CSP1D crate 基础结构，按 application/domain/infrastructure 分层。
- 迁移 CSP1D 核心公开模型的初始 Rust 版本，包括 assignment helper、问题、配置、解、KPI、恢复、列生成追踪、材料、产品、需求、设备、配规、切割方案、渲染 DTO 等。
- 补齐领域子模块骨架：material、produce、cutting plan generation、yield、waste minimization、length assignment。
- 建立初始生成器、定价生成器、reduced-cost 生成器的公开 trait 与简化实现。
- 建立扩展集合与主要扩展点类型，包含建模扩展、领域策略、目标策略、生成策略、定价策略、流程策略、提取策略。
- 增加 `ProduceInput` 与 `ProduceAggregation`，支持按方案 id 和 canonical key 去重保存方案池。
- 为 `Csp1dModelingExtension` 增加静态 pipeline、context-aware pipeline 构造与解析入口。
- 扩展策略 trait 的默认方法，使下游扩展点形状更接近 Kotlin 版本。
- 补齐 `Csp1dProblemBuilder`、`Csp1dSolveConfigBuilder`、`csp1d_problem`、`csp1d_solve_config`，并通过 crate 根导出。
- 落地 `Csp1dProduceContext` 与 `Csp1dProduceContextBuilder`，能围绕 `MetaModel` 注册方案变量、需求/物料/设备约束、目标函数和 warm start 初始值。
- 落地 `DemandConstraintPipeline`、`MaterialConstraintPipeline`、`MachineConstraintPipeline`，约束 metadata 中携带 shadow price key 字符串。
- 增加 `Csp1dDefaultShadowPriceMap` 与 `Csp1dShadowPriceLifecycle`，支持从模型约束 args 与 dual slice 提取轻量 `ShadowPriceMap<V>`。
- 扩展切割方案生成输入，支持 candidate filter、width feasibility check、canonical key override、dominance accept override，并对接 `Csp1dGenerationStrategy`。
- 改进 `SimpleInitialCuttingPlanGenerator` 与 `SimplePricingGenerator`，开始应用宽度可行性、domain policy、candidate filter 和生成策略。
- 改进 `ReducedCostPricingGenerator`，已按 shadow price、目标成本、pricing modifier、pricing policy、is-improving、canonical 去重和 `max_generated_plans` 筛选候选。
- 补齐 warm start plan-pool adapter 的核心行为，能把 warm start 方案池作为初始方案池，并从 `previous_solution` 中按 canonical key 提取兼容使用量。
- `Csp1dRecovery::solve_with_trace` 已对齐 Kotlin 默认 unsupported adapter、fallback-disabled trace、`retry_without_warm_start` 和 flow policy 覆盖语义。
- 新增 `Csp1dColumnGenerationRecovery` 等价入口，可复用列生成服务与 warm-start adapter，追踪字段与 `Csp1dRecovery` 保持一致。
- 列生成服务不再固定返回空解：已生成初始方案、简化 pricing 迭代、构建 produce context，并用确定性启发式回填 solution、KPI、render、trace。
- solution analyzer/enrichment 会同步写入基础 KPI details 与 render KPI，保留 Kotlin 稳定 key。
- crate 根导出已覆盖新增 builder、context、pipeline、shadow price helper、生成/定价函数型扩展点。
- yield/waste/length 已形成轻量结果提取闭环：从选中方案提取产出/欠产/超产、trim width、rest material、material cost、over-production area、动态长度分配与 over length，并回填 KPI details/render KPI。
- yield/waste/length 轻量建模管线已接入 `MetaModel`：注册 yield under/over slack、length assigned/over slack、yield balance/upper-bound、length bound/link 约束，并把 yield/waste/length objective terms 汇入统一目标。
- 已拆出 `YieldObjectivePipeline`、`WasteObjectivePipeline`、`LengthObjectivePipeline`，由 `Csp1dProduceContext` 统一调用并汇总目标项，形状更接近 Kotlin objective pipeline。
- `MetaModel`/`BasicModel` 新增按约束组移除约束的轻量 API，供列生成 add-column 后刷新内置约束族使用。
- `Csp1dProduceContext::add_columns` 已在新增列变量后刷新 demand/material/machine/yield/length 内置约束组，并重设完整目标，避免旧约束和增量目标残留。
- `Csp1dIncrementalPipeline` 已接入 `Csp1dProduceContextBuilder` 和 `Csp1dProduceContext::add_columns`，下游增量扩展可在新增列后同步刷新。
- yield/length 结果提取已优先从 solver slack/length 变量结果回填，并与分析型结果合流；最终启发式 MILP 会同步写入 yield under/over、length assigned/over 的模型变量结果。
- 新增 `GenerationConstraints`、`DominanceStrategy`、`NSameGenerator`、`NSumGenerator`、`DFSGenerator`、`FullSumGenerator` 与 `CostarFiller` 的 Rust 公开入口；当前为轻量确定性实现，覆盖 Kotlin 生成器族的调用面、amount/knife 约束、组合方案和配规切片填充主语义。
- `CuttingPlanGenerationBenchmarkSnapshot::to_stable_line` 已按 Kotlin 契约输出 stop reason 稳定枚举名，并新增 `from_statistics` 工厂。
- solution enrichment 已补齐 Kotlin generation statistics 稳定 key：initial/pricing 统计、width/knife/length prune、缓存计数、stop reason、render 短 key 与 details 同步输出。
- Top-K cutting plans 已按 Kotlin `usedWidth` 降序语义选择，并在 enrichment 后同步更新 `TopPlanCount` 与 yield/waste/length metric count 的 details/render KPI。
- extraction policy enrichment 已对齐 Kotlin 异常边界：单个下游策略 panic/失败不会打断 solution enrichment，其余策略仍可继续写入 details/render KPI。
- flow policy helpers 已从占位实现改为 Kotlin 对齐的策略调用语义：支持初始方案过滤、自定义等价判断、提前停止、termination selection、partial 接受和 recovery fallback fold；列生成 pricing loop 已接入自定义等价去重、early stop 与 termination reason 覆盖。
- `GenerationConstraints.enable_dominance_pruning` 与 `dominance_strategy` 已接入 Rust 侧 `PlanCollector`：支持同贡献余宽 dominance 替换、跨贡献 relaxed dominance 计数、Kotlin 式 `dominatedCandidates` / `crossContributionDominated` 统计，并让 dominance override 看到当前 accepted plan 列表。
- DFS/N-Sum/FullSum 组合生成路径已补轻量物料宽度索引缓存、数量贡献缓存和顺序切片模板缓存统计，`materialWidthIndexCacheHits`、`materialSliceTemplateCacheHits`、`materialSliceTemplateCacheMisses`、`quantityCacheHits`、`quantityCacheMisses` 会进入 generation statistics、trace、KPI details 和 render KPI。
- render DTO 与 selected cutting plan 映射已补齐 Kotlin 关键字段：方案 group、used width、standard width、info.planId，以及 production name/x/width/unit length/type/info.amount；默认 solution analyzer 和 enrichment 都会输出 selected cutting plans 的完整 render 列表。
- render DTO 已在 `serde` feature 下对齐 Kotlin `@Serializable` 能力，并使用 `camelCase` 字段名输出 `cuttingPlans`、`unitLength`、`standardWidth` 等 Kotlin 稳定序列化口径。
- 新增 `README.md` 与 `README_ch.md`，按规则互链，并记录 Rust 单 crate 与 Kotlin 多模块的映射、public API、扩展点、泛型/物理量边界、render serde 能力、当前启发式后端边界和本地验证命令。
- 新增 `tests/csp1d_migration.rs`，覆盖 builder、produce aggregation、produce context 注册、shadow price roundtrip/lifecycle、CGPipeline refresh/extractor、生成器扩展、NSame/NSum/DFS/FullSum/CostarFiller、dominance pruning/override、轻量缓存统计、flow policy helper/filter/equivalence/early-stop、初始方案 flow context、benchmark stable line、solution enrichment statistics、Top-K used width、extraction policy panic boundary、reduced-cost pricing、默认列生成、普通 MILP 独立入口、MILP/LP solver surface、MILP 默认 length bounds 推导、domain policy width override、warm start adapter、recovery warm start trace/fallback-disabled、yield/waste/length pipeline、add-column 刷新、remove-column 退役、增量扩展管线与 solver slack 回填共 44 条迁移回归测试。
- 对照 Kotlin 实现和 Rust Gantt/BPP3D 迁移模式完成第一轮接口盘点。
- `DemandConstraintPipeline`、`MaterialConstraintPipeline`、`MachineConstraintPipeline`、`YieldConstraintPipeline` 已实现 Rust 侧轻量 `Csp1dCGPipeline`，支持按 constraint group 和 `constraint.args` 刷新 shadow price，并提供方案级 extractor 计算 pricing 对偶收益；`Csp1dShadowPriceLifecycle` 可持有 CG 管线、刷新框架 map、转换轻量 `ShadowPriceMap<V>` 并计算单方案 shadow price contribution。
- `Csp1dIterativeContext::extract_shadow_price` 已返回 pricing 可消费的 `ShadowPriceMap<V>`；`Csp1dColumnGeneration` 的简化 pricing 循环已先构建 LP produce context 并通过 lifecycle 路径提取 shadow price，再喂给 pricing generator，后续接真实 LP 对偶解时可复用同一调用面。
- `Csp1dIterativeContext::remove_columns` 已有保守退役实现：不重排方案池索引，而是把对应列变量范围固定为 0，并刷新内置约束组和完整目标；内置约束、目标、warm start 与 solution extraction 会跳过退役方案。
- `Csp1dMilp` 已从 `Csp1dColumnGeneration` type alias 拆为独立普通 MILP 入口：只生成初始方案并进入启发式 MILP，不运行 pricing loop；trace 中 iteration/priced/pricing statistics 保持为空，final MILP status 与 solution KPI 同步回填。
- 新增 `Csp1dMilpSolver`、`Csp1dMilpSolveResult`、`Csp1dLpSolveResult` 的 Rust 公开入口，先以轻量启发式后端承接 Kotlin `Csp1dMilpSolver` 的 MILP/LP 调用面，并暴露 model、变量解、shadow price 与 framework shadow price map。
- 新增 `Csp1dSchedule` Rust 公开入口，对齐 Kotlin 最小排程入口，默认委托 `Csp1dColumnGeneration::solve`。
- 新增 `Csp1dAssignment` Rust 公开 helper，对齐 Kotlin application model 中的方案使用量变量组创建、索引访问和注册能力。
- `Csp1dSolveConfig::all_extensions` 已按 Kotlin `allExtensions` 语义合并 `extensions` 与 `extension_set.modeling_extensions`：只去除同一扩展重复项，不再按 mode 误删同阶段不同扩展。
- `Csp1dMilpSolver` 已对齐 Kotlin `resolveDefaultLengthBounds`：仅配置 `dynamicProductIds` 且缺少 assigned length lower/upper bound 时，会从需求/方案贡献/惩罚项推导 0 下界，并从产品 `max_over_produce_length` 推导上界，确保 assigned length 变量注册与结果提取。
- 已新增 `width_feasibility_check_from_policies`，按 Kotlin `widthFeasibilityCheckFromPolicies` 语义只让 `overrides_width_feasibility = true` 的 domain policy 接管原始 `canCut` 判断；`Csp1dColumnGeneration`、`Csp1dMilp` 的初始生成和列生成 pricing 输入均已接入。
- 服务层 trace 的 `initial_plan_count` 已改为过滤、去重、截断后的实际初始方案池大小，并在 pricing 新增列后仍与 `final_plan_count` 分离，避免偏离 Kotlin trace 语义。
- 新增 `filter_initial_plans_by_policies_with_context`，`Csp1dColumnGeneration` 与 `Csp1dMilp` 初始方案过滤会传入包含 iteration、current plans、iteration limit、allow partial 的 flow context，对齐 Kotlin 初始方案 filter context。
- `CuttingPlan::canonical_key` 已对齐 Kotlin `CuttingPlanCanonicalKey` 结构语义：纳入 material、machine、capacity consumption、按 production type/id/width 合并排序后的 slices，以及按 product/unit 合并排序后的 demand contributions，避免 warm start、方案池去重和 pricing duplicate 判断漏判结构差异。
- `shadow_price_key_to_string` 已改为输出 Kotlin `name` 口径（如 `product-demand:p1:m`、`material-usage:m1`、`yield-over-production-bound:p1:m`），约束 args、shadow price lifecycle 和 mock dual 映射统一使用该稳定格式；`shadow_price_key_from_string` 保留旧 Rust 管道格式解析兼容。
- `WidthRange::can_cut` 与 `Material::enabled` 已对齐 Kotlin 行为：产品/候选宽度只需不超过上界即可切割，物料方案可行性按方案总 `usedWidth` 检查上界，设备加工物料仍按物料幅宽上下界闭区间判断。
- `Product::weight_for`、`Product::weight`、`CuttingPlanDemandContribution::from_demand/of/quantity_of` 已迁移 Kotlin 重量贡献公式：当需求单位与产品 `unitWeight` 单位一致且长度存在时，按 `width * length * unitWeight * amount` 计算贡献，否则按切片份数贡献；Simple、pricing 和组合生成路径均已复用该 helper。
- 补齐 Kotlin material-context 轻量公共 helper：`QuantityRange` 开闭区间、`DefaultQuantityArithmetic` 的 add/subtract/zero/正负判断、`WidthRange::with_step`、动态长度产品带 `unitWeight` 构造，以及 `Costar` 的 length/unitWeight 生产属性；`CuttingPlanProduction::Costar` 渲染可读取 length，`Product::weight` 会按长宽与单位重量推导质量单位，`CuttingPlan::rest_width` 保留 Kotlin 直接减法结果。
- 补齐 `Product::legacy(ProductLegacyInput)` 与 `convert_solver_value`，承接 Kotlin legacy 产品输入和 solver 边界值转换 helper；`Material::enabled_without_width_check_with_machines` 已对齐 Kotlin 跳过宽度但仍检查设备加工范围的重载，并接入 initial/pricing/DFS/N-Sum/FullSum 生成路径。
- 补齐 Kotlin generation-context 的 `CuttingPlanConstraint` public 契约：新增 `CuttingPlanConstraintContext`、Max/Min knife、MaxOverProduceLength、WidthUpperBound 约束类型，以及 `GenerationConstraints::to_constraints()` 转换入口。

## 未完成事项

- `YieldConstraintPipeline`、`LengthConstraintPipeline`、`YieldObjectivePipeline`、`LengthObjectivePipeline`、`WasteObjectivePipeline` 已有轻量 Rust 实现；内置 add-column 刷新、增量扩展管线回调和 yield/length solver slack 回填已迁移。
- 普通 MILP 入口已独立迁移，但仍使用轻量启发式 MILP 后端；列生成求解流程仍是简化替代路径，尚未接真实 LP RMP、dual refresh、solver backend 和最终 MILP 后端。
- remove-column 当前是变量固定为 0 的退役语义，尚未实现 Kotlin 未来可能需要的物理删除、池压缩、历史 iteration 重排或 solver basis/warm-start 同步。
- 初始切割方案生成器族已有轻量 Rust 入口：Simple、DFS、N-Same、N-Sum、FullSum、CostarFiller 可被调用并覆盖小规模确定性语义；内置 dominance pruning、轻量物料宽度索引缓存、数量缓存统计和顺序切片模板缓存已迁移，尚未迁移 Kotlin 的并发模板缓存、复杂扩展场景复用、大规模性能和全部 statistics 细节。
- reduced-cost pricing 已有 Kotlin 主语义的轻量实现，但还未接真实 LP dual，generation statistics 中 duplicate/dominated/infeasible 等细分计数仍不完整。
- yield、waste、length 的轻量建模聚合、变量、约束和目标项已落地；完整 Kotlin objective pipeline 类型和真实 solver 变量结果来源仍需继续迁移。
- shadow price key、轻量 map、constraint args 提取与 Rust 侧轻量 CGPipeline refresh/extractor 生命周期已落地；但真实 LP RMP 尚未接入，当前服务层仍使用占位对偶解驱动 lifecycle，尚未由 solver dual solution 驱动。
- recovery/warm start adapter 已支持 plan-pool、previous-solution usage、默认 unsupported adapter、`Csp1dColumnGenerationRecovery` 与 fallback-disabled trace；完整失败/partial solve exception 映射仍需随真实 solver 接入继续对齐。
- solution enrichment、Top-K cutting plans、render 输出、KPI details、extraction policy 回填与异常边界已有轻量实现；selected cutting plan render 的核心字段和 serde 字段名已对齐 Kotlin，剩余全量 UI/序列化细节仍需逐项核销。
- flow policy 的过滤、等价、early-stop、termination selection、partial/recovery fallback 已接入；真实 LP 失败路径仍需随求解后端继续迁移。
- 顶层 README.md、README_ch.md 已补齐；模块 README、使用示例和验收 fixture 尚未补齐。
- Kotlin 对照测试已覆盖 Rust 侧 DFS/NSum/FullSum/CostarFiller 的轻量 smoke/migration 场景，并补充 dominance pruning/override 与轻量缓存统计回归；尚未覆盖 Kotlin 大规模 baseline、并发模板缓存/并行统计和真实 solver 场景。

## 后续计划

1. 收窄 yield/waste/length 差异。
   - 继续对齐 Kotlin objective pipeline 的完整类型和目标项语义。
   - 在真实 solver 接入后，用后端 solution 驱动 yield/length slack 回填，并扩展到失败/部分结果路径。

2. 补齐列生成真实生命周期。
   - 接入真实 LP RMP 求解、shadow price refresh/extractor、pricing、add_columns 和最终 MILP 后端。
   - 将真实 RMP 的新增列流程接到 `Csp1dIterativeContext::add_columns`，并用实际对偶值驱动 pricing。
   - 对齐 Kotlin `AllDuplicates`、`LpInfeasible`、`LpSolveFailed`、flow policy termination selection 等细节。

3. 扩展 warm start/recovery。
   - 增加 `Csp1dColumnGenerationRecovery` 等价入口。
   - 对齐 adapter unsupported 默认行为、retryWithoutWarmStart、异常类型携带 trace、fallback-disabled 和 solve-failed 语义。
   - 增加问题变化后的兼容子集过滤测试。

4. 迁移完整切割方案生成。
   - 继续完善 DFS、N-Same、N-Sum、FullSum 生成器和 CostarFiller 的 Kotlin 细节。
   - 继续迁移完整 width/knife/length pruning、并发 slice template cache、parallel merge 和细分 statistics。
   - 对齐 Kotlin 的 canonical key override 与 dominance override 在复杂生成器中的完整应用。

5. 迁移剩余主问题管线。
   - `DemandConstraintPipeline`: demand fulfillment / under-over production 约束。
   - `MaterialConstraintPipeline`: material available batches 约束。
   - `MachineConstraintPipeline`: machine batch 和 capacity 约束。
   - `YieldConstraintPipeline`/`YieldObjectivePipeline`: 欠产、超产、超产上界与目标惩罚。
   - `LengthConstraintPipeline`/`LengthObjectivePipeline`: 动态长度分配、batch coefficient、over length。
   - `WasteObjectivePipeline`: trim width、rest material、material cost、over production area。

6. 继续迁移求解服务。
   - 将 `Csp1dMilp` 从轻量启发式后端切换到真实 MILP solver adapter。
   - `Csp1dColumnGeneration` 初始方案、LP RMP、shadow price、pricing、add columns、最终 MILP、partial fallback、trace。
   - `Csp1dRecovery` 和 `Csp1dColumnGenerationRecovery` 在真实 solver 失败/partial 场景下的异常和 fallback 流程。
   - 对齐 Kotlin `Csp1dTerminationReason`、`Csp1dFinalMilpStatus`、trace、failure message。

7. 补充结果增强。
   - 继续核销 render DTO 的扩展 info 字段和全量 UI 细节。
   - top-k plans 输出。
   - extraction policy 写入 details/render KPI。

8. 文档与测试。
   - 继续补充模块 README、使用示例和验收 fixture。
   - 建立 Kotlin 对照测试，包括 canonical key、domain policy、extra pipeline、CG lifecycle、application acceptance、yield/waste/length context。
   - 增加 Rust-only smoke tests，覆盖 builder、generate、produce aggregation、extension mode、KPI、shadow price key。
   - 引入 fixture 后做端到端验收。

9. 最终复审。
   - 重新阅读 Kotlin `ospf-kotlin-framework-csp1d` 全量实现。
   - 建立接口/能力对照表，逐项核销。
   - 对照已完成迁移的 Gantt 和 BPP3D，确认 Rust 侧架构风格一致。
   - 运行 `cargo check -p ospf-rust-framework-csp1d`、`cargo test -p ospf-rust-framework-csp1d` 和必要的 workspace 级检查。

## 修改清单

- `Cargo.toml`
  - 将 `ospf-rust-framework-csp1d` 加入 workspace members。

- `ospf-rust-framework-csp1d/Cargo.toml`
  - 新增 CSP1D crate manifest。
  - 配置对 core/framework/quantities/base 的依赖。
  - 暴露 serde、async、solver backend 相关 feature 转发。
  - 增加 `serde_json` dev 依赖，用于验证 render DTO 的 Kotlin camelCase 序列化契约。

- `ospf-rust-framework-csp1d/README.md`
  - 新增英文顶层说明，记录 Kotlin 模块到 Rust crate 模块的映射、public API、扩展点、shadow price lifecycle、生成语义、render serde 和验证命令。
  - 明确当前 MILP/LP/final-MILP 仍为启发式后端，真实 solver adapter、LP dual 和完整并行生成统计仍待后续迁移。

- `ospf-rust-framework-csp1d/README_ch.md`
  - 新增中文顶层说明，并与英文 README 互链。
  - 与英文版同步说明当前能力边界、迁移状态和本地验证命令。

- `ospf-rust-framework-csp1d/src/lib.rs`
  - 新增 crate 模块导出。
  - 新增 `Csp1dError` 与 `Csp1dResult<T>`。
  - 汇总导出 application/domain/infrastructure 主要类型。
  - 补充导出 builder、produce context、iterative context、constraint pipeline、CGPipeline shadow price helper、generation/pricing 函数型扩展点与 domain policy width helper。

- `ospf-rust-framework-csp1d/src/application/model.rs`
  - 新增 assignment helper、问题定义、配置、解、KPI key、KPI、solution analyzer。
  - 初步对齐 Kotlin `Csp1dProblem.kt`、`Csp1dSolution.kt`。
  - `Csp1dAssignment` 已对齐 Kotlin `Csp1dAssignment.kt` 的 `create`、`get` 和 `register` 调用面。
  - 新增问题/求解配置 builder 与 DSL 便利函数。
  - KPI details 与 render KPI 同步写入 Kotlin 稳定 key。
  - 默认 solution analyzer 已直接渲染 selected cutting plans，对齐 Kotlin analyzer 的 render schema 输出。

- `ospf-rust-framework-csp1d/src/application/service.rs`
  - 新增 column generation trace/result、termination reason、final MILP status。
  - 新增 warm start、recovery、adapter、异常类型。
  - 新增 `Csp1dColumnGeneration`/`Csp1dMilp`/`Csp1dRecovery` 骨架。
  - 列生成入口补充初始生成、简化 pricing、最终启发式 MILP、KPI/render/trace 回填。
  - 简化 pricing 循环已通过 LP `Csp1dProduceContext` 和 `Csp1dIterativeContext::extract_shadow_price` 获取 shadow price map，再传入 pricing generator；真实 LP solver 对偶值仍待替换当前占位 dual。
  - warm start plan-pool adapter 支持初始方案池与 previous-solution usage 提取。
  - `Csp1dRecovery::solve_with_trace` 支持 warm start 解析、adapter 应用和 trace 回填。
  - 新增默认 unsupported warm-start adapter、`Csp1dColumnGenerationRecovery`、fallback-disabled `Csp1dError` trace 和 flow policy fallback 覆盖。
  - solution enrichment 会同步输出完整 generation statistics details/render KPI，stop reason 使用 Kotlin 稳定枚举名。
  - Top-K plans 改为按 `used_width` 降序选择，并在 enrichment 中同步 TopPlanCount 与 metric count。
  - extraction policy enrichment 已加 panic 边界，单个策略失败不会破坏求解结果。
  - flow policy helper 已调用下游策略，覆盖 initial plan filter、自定义等价、pricing duplicate、early stop、termination selection、partial/recovery fallback 等扩展点。
  - 最终启发式 MILP 会根据选中方案写入 yield/length slack 变量结果，供结果提取按 solver 变量回填。
  - `Csp1dMilp` 已拆为独立普通 MILP 入口，生成初始方案后直接求 MILP，并用空 pricing trace 对齐 Kotlin `Csp1dMilp`。
  - 新增 `Csp1dMilpSolver`、`Csp1dMilpSolveResult`、`Csp1dLpSolveResult` 公开调用面；当前复用轻量启发式 MILP/LP 注册与提取路径，真实 solver adapter 待后续替换内部实现。
  - 新增 `Csp1dSchedule` 公开入口，默认以 `Csp1dColumnGeneration` 作为求解路径。
  - `solve_milp_input_heuristic` 已接入 Kotlin `resolveDefaultLengthBounds` 等价推导，只有 `dynamic_product_ids` 而未显式配置 assigned lower/upper bound 时仍会注册 assigned length 变量并提取 length result。
  - `Csp1dColumnGeneration` 与 `Csp1dMilp` 已在生成输入中接入 domain policy width override helper；trace 初始方案数改为实际方案池数量，初始方案 flow policy filter 使用 Kotlin-like context。

- `ospf-rust-framework-csp1d/src/domain/material/model.rs`
  - 新增 Product/Material/Machine/Costar/ProductDemand/CuttingPlan 等核心领域模型。
  - 新增 quantity alias、demand mode、shadow price key、render mapper。
  - 新增 canonical key 结构化实现，对齐 Kotlin `CuttingPlanCanonicalKey` 的 slice/contribution 合并排序与 capacity consumption 字段。
  - 补充 material/machine 可行性、used/rest width、Kotlin `Csp1dShadowPriceKey.name` 口径序列化、旧管道格式解析兼容和数值转换 helper。
  - `WidthRange::can_cut`、`WidthRange::contains`、`Material::enabled`、`Machine::enabled` 已按 Kotlin `WidthRange.canCut`、`Material.enabled`、`Machine.enabled` 语义分离候选宽度判断、方案总宽判断和设备幅宽闭区间判断。
  - 补齐 `Product::weight_for` / `Product::weight` 与 `CuttingPlanDemandContribution` 工厂函数，Simple/DFS/N-Sum/FullSum/pricing 生成路径会按 Kotlin `quantityOf` 公式生成离散或重量贡献。
  - 新增 `QuantityRange`、`DefaultQuantityArithmetic`、`WidthRange::with_step`、`Product::dynamic_length_of_with_unit_weight`，并让 `Costar` 实现 length/unitWeight 生产属性。
  - 新增 `ProductLegacyInput`、`Product::legacy`、`convert_solver_value` 与 `Material::enabled_without_width_check_with_machines`。
  - `CuttingPlanProduction::length` 已支持 Costar length，`CuttingPlan::rest_width` 已按 Kotlin `upperBound - usedWidth` 保留负值。
  - render mapper 已按 Kotlin selected-plan 契约输出 production 横向坐标、方案 used/standard width、group 和 info。

- `ospf-rust-framework-csp1d/src/domain/produce/mod.rs`
  - 新增 Produce、usage、ProduceInput、ProduceAggregation。
  - 新增 model context、iterative context、modeling extension、extension set。
  - 新增 domain/objective/generation/pricing/flow/extraction policy trait 和默认方法。
  - 新增流程策略辅助函数骨架。
  - 增加 plan variable register、add_columns_to_model、默认需求/物料/设备约束管线、目标注册、shadow price lifecycle、produce context builder。
  - 增加 yield/waste/length 轻量结果提取，基于选中方案生成产出、欠产/超产、余宽/余料、材料成本、动态长度分配等结果。
  - 增加 `YieldConstraintPipeline`、`LengthConstraintPipeline`、`YieldObjectivePipeline`、`WasteObjectivePipeline`、`LengthObjectivePipeline`，并在 `Csp1dProduceContext::register` 中注册对应 slack 变量、约束和目标项。
  - `add_columns` 后会按内置约束组刷新 demand/material/machine/yield/length 约束并重设完整目标。
  - `Csp1dIncrementalPipeline` 已可通过 builder 注入，并在 add-column 后收到新增列回调。
  - yield/length 结果提取优先读取 solver slack/length 变量值，缺失部分继续由分析型结果补齐。
  - 新增 `Csp1dCGPipeline` 与 `Csp1dShadowPriceExtractor`，内置 demand/material/machine/yield 管线已实现 refresh/extractor；`Csp1dProduceContext` 维护 `cg_pipelines` 并通过 lifecycle 提取 `ShadowPriceMap<V>`。
  - `remove_columns` 支持退役列变量、刷新内置约束与目标，并在结果提取中忽略退役方案。
  - 新增 `filter_initial_plans_by_policies_with_context`，下游 flow policy 可在初始方案过滤阶段读取服务层传入的完整 flow context。

- `ospf-rust-framework-csp1d/src/domain/cutting_plan_generation/*`
  - 新增 generation input/report/statistics/benchmark/pricing input/objective config。
  - 新增 initial/pricing generator trait 和简化生成器。
  - 扩展 generation/pricing 输入，支持 candidate filter、width check、canonical key override、dominance override、pricing modifiers、is-improving judge、pricing policy。
  - `ReducedCostPricingGenerator` 开始按 dual benefit、objective cost、improving、排序和上限筛选。
  - 新增 `GenerationConstraints`、`DominanceStrategy`、`NSameGenerator`、`NSumGenerator`、`DFSGenerator`、`FullSumGenerator` 和 `CostarFiller` 的轻量 Rust 实现。
  - `PlanCollector` 已接入 Kotlin 式同贡献 / 跨贡献 dominance pruning，并让 dominance override 接收当前 accepted plan 列表；DFS/N-Sum/FullSum 已补轻量物料宽度索引缓存、数量缓存和顺序切片模板缓存统计。
  - `CuttingPlanGenerationBenchmarkSnapshot` 补齐 Kotlin 风格 `from_statistics` 与稳定 stop reason 文本输出。
  - 新增 `width_feasibility_check_from_policies`，支持 domain policy 替换默认宽度可行性判断，并避免非 width override policy 意外放宽原始 `canCut`。
  - 新增 `CuttingPlanConstraint`、`CuttingPlanConstraintContext`、`MaxKnifeCountConstraint`、`MinKnifeCountConstraint`、`MaxOverProduceLengthConstraint`、`WidthUpperBoundConstraint`，并让 `GenerationConstraints::to_constraints()` 输出 Kotlin 同形约束列表。
  - 生成器贡献量构造改为复用 `CuttingPlanDemandContribution::from_demand`，quantity cache key 纳入 width，避免重量需求下不同切片宽度复用错误贡献。

- `ospf-rust-framework-csp1d/tests/csp1d_migration.rs`
- 新增 56 条默认迁移回归测试，覆盖 builder、assignment helper、solve config allExtensions 合并、Kotlin structural canonical key、material width feasibility、width override machine feasibility、weight demand contribution、QuantityRange/QuantityArithmetic/Production helper、Product/ProductDemand legacy helper、generation constraint predicate、domain policy contribution helper、produce context、shadow price、CGPipeline extractor、context shadow price extraction、服务层 pricing shadow price 接线、生成器扩展、NSame/NSum/DFS/FullSum/CostarFiller、dominance pruning/override、轻量缓存统计、flow policy helper/filter/equivalence/early-stop、初始方案 flow context、benchmark stable line、solution enrichment statistics、Top-K used width、extraction policy panic boundary、reduced-cost pricing、列生成、普通 MILP 独立入口、Schedule 入口、MILP/LP solver surface、MILP 默认 length bounds 推导、domain policy width override、warm start、recovery、yield/waste/length pipeline、add-column 刷新、remove-column 退役、增量扩展管线、solver slack 回填与结果闭环；`serde` feature 下另有 render schema camelCase 序列化口径测试。

- `ospf-rust-core/src/model/basic_model.rs`
  - 新增 `retain_constraints` 与 `remove_constraints_by_group_id`，支持动态建模按约束组刷新。

- `ospf-rust-core/src/model/meta_model.rs`
  - 暴露 `retain_constraints` 与 `remove_constraints_by_group_id`，并增加按 group id 删除约束的回归测试。

- `ospf-rust-framework-csp1d/src/domain/yield/*`
  - 新增 yield config/result/analysis/output/aggregation/context 骨架。
  - 结果提取已能产出 output、under production、over production。
  - 新增 `YieldSlackAggregation`，按 Kotlin key 口径管理 under/over slack 变量索引。

- `ospf-rust-framework-csp1d/src/domain/wasting_minimization/*`
  - 新增 waste config/result/analysis/aggregation/material cost 骨架。
  - 结果提取已能产出 total trim width、total rest material、over-production area、material cost 与 measure metadata。

- `ospf-rust-framework-csp1d/src/domain/length_assignment/*`
  - 新增 length assignment input/result/config/context 骨架。
  - `DefaultLengthDerivation` 与 `LengthAssignmentContext::assign` 已支持动态长度分配和 over length 记录。
  - `LengthAssignmentModelingConfig` 已补齐 Kotlin 的 dynamic product、assigned length bound、over length bound/penalty、total length penalty、batch min penalty 字段；新增 `LengthSlackAggregation` 管理 assigned/over length 变量索引。

- `ospf-rust-framework-csp1d/src/infrastructure/dto.rs`
  - 新增 render DTO，并补齐 Kotlin `RenderCuttingPlanProductionDTO` / `RenderCuttingPlanDTO` 的 name、x、group、width、standardWidth、info 等关键字段。
  - 在 `serde` feature 下派生 `Serialize`/`Deserialize`，并使用 `camelCase` 字段名对齐 Kotlin `@Serializable` DTO。

- `ospf-rust-framework-csp1d/daily.md`
  - 新增本交接文档。

## 当前验收结果（2026-06-15 更新）

- 已运行：`cargo check -p ospf-rust-framework-csp1d`
- 结果：通过。
- 已运行：`cargo test -p ospf-rust-framework-csp1d`
- 结果：通过，57 条迁移测试通过。
- 已运行：`cargo check -p ospf-rust-framework-csp1d --features serde`
- 结果：通过。
- 已运行：`cargo test -p ospf-rust-framework-csp1d --features serde render_schema_serializes_with_kotlin_camel_case_fields`
- 结果：通过，1 条 serde feature 序列化口径测试通过。
- 已运行：`cargo test -p ospf-rust-core remove_constraints_by_group_id --lib`
- 结果：通过，1 条 core 回归测试通过。
- 修复：测试 `merge_generation_reports` 中 `CuttingPlanGenerationReport` 缺少泛型参数 `V` 导致编译失败，已补 `<f64>`；测试总数从 56 增至 57。
- 已知 warning：
  - `ospf-rust-math` 和 `ospf-rust-quantities` 的既有 warning。
  - CSP1D crate 自身当前无新增 warning。

## 最终验收标准

- 架构一致：
  - Rust CSP1D 按 domain context / aggregation / model component / pipeline / application service 分层。
  - 主问题建模围绕 `MetaModel`，不在 application service 中集中堆约束细节。
  - 列生成生命周期覆盖 register、add_columns、remove_columns、refresh_shadow_price、extract_shadow_price、finalize、extract_solution。

- 接口一致：
  - Kotlin 公开模型、配置、builder、solution、trace、recovery、warm start、render DTO 在 Rust 侧有自然等价 API。
  - 扩展点覆盖 modeling extension、incremental pipeline、domain policy、objective policy、generation strategy、pricing policy、flow policy、extraction policy。
  - shadow price key、KPI key、termination reason、final MILP status 与 Kotlin 语义一致。

- 特性一致：
  - 支持普通 MILP、列生成、最终 MILP、partial solution、top-k plans。
  - 支持 yield、waste、length assignment。
  - 支持 warm start 和 recovery fallback。
  - 支持 context-aware extra pipeline 与增量加列扩展。
  - 支持 domain policy 替换宽度可行性判断、candidate filter、canonical key override、dominance override。
  - 支持 pricing policy 修正 reduced cost 的 cost/benefit/is-improving。

- 能力一致：
  - 初始方案生成能力覆盖 Simple、DFS、N-Same、N-Sum、FullSum、CostarFiller。
  - reduced-cost pricing 读取 shadow price 并按策略筛选新增列。
  - generation statistics 与 benchmark snapshot 字段完整。
  - solution analyzer/enrichment/render 输出与 Kotlin 对齐。

- 质量标准：
  - `cargo check -p ospf-rust-framework-csp1d` 通过。
  - `cargo test -p ospf-rust-framework-csp1d` 通过。
  - 新增 README.md 和 README_ch.md 互链。
  - 与 Kotlin 测试语义等价的 Rust 测试覆盖关键路径。
  - 重新阅读 Kotlin 全量实现后，差异清单全部核销或明确记录为 Rust 侧有意调整。
