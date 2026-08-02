# 求解执行契约、诊断与可审计能力改进计划

日期：2026-07-26

首阶段插件范围：`ospf-kotlin-core-plugin-scip`、`ospf-kotlin-core-plugin-gurobi`

后续插件计划：`plans/solver_cp.md`

## 1. 背景与当前状态

本计划承接 APS What-if 场景对结构化求解结果、不可行证据、求解器 provenance、取消、重放和跨求解器评测的通用需求。OSPF 负责数学模型、求解执行、诊断证据和通用实验基础设施；上层业务系统负责业务快照、候选生命周期、KPI、审批、权限和正式版本晋升。

OSPF 当前已经具备以下基础：

- 线性、二次求解器统一接口，以及 Gurobi、SCIP 等已实现插件；仓库中还存在 CPLEX、COPT、MindOPT、Mosek、Hexaly、Gurobi 11 等待后续统一验收的插件。
- `FeasibleSolverOutput` 中的目标值、bound、gap、节点数、迭代数和耗时。
- 通用 IIS 计算、列生成、Benders 分解和串并行组合求解器。
- 远程求解、检查点、对象存储和远程停止接口。
- 基于 JMH 的建模与数学热点性能基准。

当前主要问题不是缺少求解算法，而是求解结论、终止原因、解存在性、诊断证据和执行来源没有形成统一且无损的契约：

1. `SolverStatus` 无法区分超时、取消、节点/迭代限制、数值故障等终止原因。
2. 主求解入口返回 `Ret<FeasibleSolverOutput<...>>`，无法表达“超时但保留 incumbent”或“正常完成并证明不可行”。
3. 插件会把多个原生状态压缩为 `Feasible` 或 `SolvingException`。
4. IIS 缺少来源、精度、完整度和缺失原因；诊断失败可能覆盖已经确定的不可行结论。
5. 求解器只有名称字符串，没有运行时能力声明和完整 provenance。
6. 变量、约束和目标缺少跨建模、序列化、求解和回放稳定的元素 ID。
7. 本地异步取消、组合求解取消和远程停止没有统一语义。
8. 串并行组合求解器没有保留各 backend 的 attempt trace。
9. 本地与远程结果协议分裂，远程结果仍依赖布尔值和状态字符串。
10. 现有 benchmark 只覆盖性能热点，没有求解正确性、跨 backend 和重放一致性评测。

## 2. 改进目标

1. 建立正交的求解报告模型，分别表达问题结论、终止原因、解存在性、证明、统计、诊断和 provenance。
2. 首阶段保证 Gurobi 与 SCIP 无损映射原生状态；达到限制或被取消时，只要存在 incumbent 就必须保留解、目标、bound 和 gap。
3. 建立运行时 `SolverCapabilities` 和 `SolverProvenance`，让调用方在求解前协商能力、在求解后审计实际执行环境。
4. 建立统一取消句柄，使协程、`CompletableFuture`、组合求解和远程停止都能触发 backend 原生中断。
5. 将 IIS、冲突集合、松弛证据和 Farkas 证书统一为带来源与精度的诊断模型，禁止把启发式证据冒充精确 IIS。
6. 为模型元素提供稳定 ID，并输出可机器处理的约束 lhs、rhs、slack、violation、容差和 dual。
7. 为规范化模型、实际求解配置和求解器环境生成稳定指纹，支持审计和确定性重放。
8. 让串行、并行和远程求解复用同一报告协议，并保留完整 attempt trace。
9. 建立 Gurobi/SCIP 求解正确性 fixture、跨 backend 矩阵和 golden/replay 门禁，并为后续插件提供可复用合同。
10. 在核心契约稳定后提供纯函数式批量实验工具，为离线 MILP 样本、积分间隙和分支策略实验提供通用基础设施。

## 3. 边界

本计划不把以下能力放入 OSPF：

- 业务方案、候选、滚动轮次、业务版本、历史修订、审批和租户模型。
- APS、MPS、MRP、排程、裁切、装箱等业务 KPI 和业务对象影响分析。
- 硬约束是否允许放宽、候选是否晋升、是否允许跨算法或跨求解器回退等业务决策。
- Agent 提示、自然语言解释、权限校验、仓储、事务、消息和正式下发。
- 在生产路径中自动启用积分间隙预测或学习分支。

OSPF 可以提供通用回退执行机制和 attempt trace，但是否启用回退以及回退顺序必须由调用方显式配置。

### 3.1 首阶段求解器插件范围

- 本计划只交付并验收 `ospf-kotlin-core-plugin-scip` 与 `ospf-kotlin-core-plugin-gurobi` 的状态、报告、provenance、取消、诊断和重放垂直切片。
- 本文中的 “Gurobi” 特指 `ospf-kotlin-core-plugin-gurobi`；`ospf-kotlin-core-plugin-gurobi11` 不在首阶段范围。
- core 中的 `SolveReport`、稳定 ID、诊断、指纹、取消和远程 DTO 仍保持 backend-neutral，但通用类型存在不代表其他插件已完成适配或验收。
- COPT、CPLEX、Gurobi 11、MindOPT、Mosek、Hexaly、Lingo、OPTVerse 和 Heuristic 的通用求解合同及 CP 支持统一转入 `plans/solver_cp.md`。
- 其余插件在完成 `plans/solver_cp.md` 对应门禁前保持现有行为；未实现的新能力必须声明 `Unsupported`，不得纳入本计划完成统计、跨 backend 矩阵或远程 capability。

## 4. 目标契约

### 4.1 求解报告

新增统一的 `SolveReport<V>`，至少包含：

| 字段 | 语义 |
|------|------|
| `problemStatus` | `Feasible`、`Infeasible`、`Unbounded`、`InfeasibleOrUnbounded`、`Unknown` |
| `terminationReason` | `Completed`、`TimeLimit`、`NodeLimit`、`IterationLimit`、`SolutionLimit`、`Cancelled`、`Interrupted`、`NumericalFailure`、`BackendFailure` 等 |
| `solution` | 可选 incumbent/最优解、目标值和解池；无解时为 null |
| `proof` | 最优、不可行或无界证明及其可靠性；没有证明时显式为空 |
| `statistics` | solve time、iterations、nodes、best bound、gap 等通用统计 |
| `diagnostics` | IIS、冲突、约束求值、警告、降级和诊断失败信息 |
| `provenance` | solver/backend/plugin/native 版本、实际配置、随机种子和执行环境摘要 |
| `fingerprints` | 模型、实际配置和求解器环境指纹 |

`Ret<SolveReport<V>>` 的语义如下：

- 已经启动且能够形成可信报告的求解尝试，无论最优、可行、不可行、超时、取消或达到限制，都返回 `Ok(SolveReport)`。
- 输入合同非法、模型无法构建、插件无法加载或发生无法形成可信报告的内部合同错误时返回 `Failed`/`Fatal`。
- backend 在启动后失败时应尽量形成 `terminationReason = BackendFailure` 的报告并保留已获得的统计和 provenance；只有报告本身不可信时才进入错误通道。

旧的 `Ret<FeasibleSolverOutput<...>>` 入口保留为兼容 facade：只在报告包含可行解时返回旧类型，其余终态映射为原有错误。新功能不得继续扩展旧输出模型。

### 4.2 能力与来源

`SolverDescriptor`、`SolverCapabilities` 和 `SolverProvenance` 至少覆盖：

- 稳定 solver ID、backend 名称与版本、OSPF 插件版本、原生库版本。
- LP、MIP、QP/QCP、solution pool、warm start、dual、Farkas、native IIS、callback、interrupt、checkpoint/resume 等能力。
- 请求配置、解析后的实际配置、backend 默认值、线程数、随机种子和确定性模式。
- 被忽略或降级的选项，以结构化 warning 返回，不只记录日志。
- 敏感连接参数只记录脱敏摘要，不进入普通日志或指纹明文。

### 4.3 模型身份、诊断和指纹

- `ModelElementId`、`VariableId`、`ConstraintId` 和 `ObjectiveId` 由建模入口稳定提供，并贯穿机制模型、中间模型、远程序列化和求解结果。
- `name` 只作为展示标签，不再作为 dual、IIS、slack 或回放的唯一关联键。
- `ConstraintEvaluation` 输出元素 ID、lhs、rhs、relation、signed slack、violation、tolerance、satisfied 和可选 dual。
- `InfeasibilityEvidence` 输出来源、精度、完整度、成员约束/变量界、耗时和缺失/失败原因。
- 规范化模型按稳定 ID 和确定的数值编码排序，生成带 schema 版本的加密摘要；不得使用普通 `hashCode`/`contentHashCode` 作为回放证明。

### 4.4 取消、组合和远程协议

- 每次求解创建独立 `SolveHandle`；`cancel()` 幂等并记录调用来源与时间。
- 协程取消和 `CompletableFuture.cancel()` 必须委托到 backend 的 terminate/interrupt/abort 接口。
- 串并行组合求解返回 `CombinatorialSolveReport`，包含每次 `SolveAttemptTrace`、选择依据和被取消 backend。
- 全部 backend 失败时返回完整 attempts，不得压缩为无上下文的 `SolverNotFound`。
- 远程协议增加 `schemaVersion`，无损传输与本地相同的报告；任务生命周期状态与求解结论保持正交。

## 5. 分阶段计划

### Phase S0：核心报告合同与兼容层（P0）

- 冻结状态正交模型、`SolveReport`、Result 语义和旧 API 兼容策略。
- 为线性、二次、解池和泛型数值路径提供统一报告入口。
- 先使用 fake solver 覆盖所有终态组合，不依赖商业求解器环境。

### Phase S1：插件状态、能力和 provenance（P0）

- 先完成 Gurobi 与 SCIP 两条垂直切片，覆盖一个商业 backend 和一个开源 backend。
- 本计划不继续迁移其它插件；剩余插件统一由 `plans/solver_cp.md` 分批实施。
- 占位或未完整实现的插件在后续验收前必须准确声明 capability，不得通过存在模块推断可用能力。

### Phase S2：统一取消与终止（P0）

- 建立 `SolveHandle`/`CancellationToken` 及 backend 原生中断适配。
- 首阶段为 Gurobi/SCIP 打通同步、协程、`CompletableFuture`、组合求解和远程停止；其余插件适配转入 `plans/solver_cp.md`。
- 覆盖取消竞态、重复取消、求解刚完成时取消和取消后资源释放。

### Phase S3：模型身份、诊断和指纹（P1）

- 稳定元素 ID 贯穿线性、二次、dual、IIS、导出和远程序列化路径。
- 实现通用约束求值与结构化不可行证据。
- 原生 IIS 优先，通用弹性/删除过滤作为有明确证据等级的降级路径。
- 建立规范化模型、实际配置和求解器环境指纹。

### Phase S4：组合求解与远程协议统一（P1）

- 串并行组合求解器返回 attempt trace 和最终选择依据。
- 本地和远程统一为版本化报告协议，检查点恢复继续沿用原 run/attempt 关联。
- 保留旧远程 DTO 的兼容读取期，并为协议升级提供显式版本错误。

### Phase S5：正确性评测与重放门禁（P1）

- 建立固定 LP/MIP/QP、不可行、无界、超时、取消和数值异常 fixture。
- 首阶段建立 Gurobi/SCIP backend 矩阵测试；缺少许可证或原生库时显式标记 skipped，不冒充通过。
- 记录预期状态、目标/界容差、可行性残差、IIS 期望和 provenance 条件。
- JMH 继续负责性能热点，正确性评测使用独立测试/报告入口。

### Phase S6：通用离线实验设施（P2）

- 提供确定性批量场景运行器、并发预算、结果集合和报告导出。
- 生成样本必须记录生成器版本、种子、模型/配置指纹和求解 provenance。
- 积分间隙预测和学习分支只输出离线观测，不自动影响正式求解策略。

## 6. 详细事项

### 6.1 核心报告合同

- [x] `OSPF-SOL-001` 定义正交的 `ProblemStatus`、`TerminationReason`、`SolutionPresence` 和 `ProofStatus`，禁止用单一枚举组合所有状态。
- [x] `OSPF-SOL-002` 定义泛型 `SolveReport<V>`、`SolveSolution<V>`、`SolveProof`、`SolveStatistics`、`SolveDiagnostics` 和结构化 warning/error 摘要。
- [ ] `OSPF-SOL-003` 为线性、二次、解池和泛型数值入口增加报告 API，并实现旧 `FeasibleSolverOutput` facade 与弃用策略。
- [ ] `OSPF-SOL-004` 统一输入非法、环境、许可证、数值、callback、解析和 backend 故障分类，明确哪些是正常终态、哪些进入 `Failed`/`Fatal`。

### 6.2 插件能力与 provenance

- [x] `OSPF-SOL-005` 定义 `SolverDescriptor` 和 `SolverCapabilities`，覆盖模型类型、IIS、dual/Farkas、warm start、解池、中断和检查点能力。
- [x] `OSPF-SOL-006` 定义 `SolverProvenance`，记录 backend/plugin/native 版本、实际参数、线程、随机种子、确定性模式和脱敏环境摘要。
- [ ] `OSPF-SOL-007` 将不可规范化的 `extraConfig: Any?` 收敛为可类型化、可序列化、可脱敏和可计算指纹的 backend 配置契约。
- [ ] `OSPF-SOL-008` 完成 Gurobi/SCIP 原生状态、统计、能力和 provenance 的无损映射垂直切片。
- [x] `OSPF-SOL-009` 将其余插件的通用合同迁移与 CP 支持转交 `plans/solver_cp.md`；本计划关闭不再依赖其它插件实现。

### 6.3 取消与资源释放

- [x] `OSPF-SOL-010` 定义每次求解独立的 `SolveHandle`/`CancellationToken`，并支持幂等取消和终止原因记录。
- [ ] `OSPF-SOL-011` 为 Gurobi/SCIP 接入原生 terminate/interrupt/abort，保证协程和 `CompletableFuture` 取消能中断阻塞求解；其余插件由 `plans/solver_cp.md` 承接。
- [ ] `OSPF-SOL-012` 统一远程 `stop`、检查点和本地取消语义，覆盖取消竞态、重复取消和资源释放测试。

### 6.4 模型身份与诊断

- [ ] `OSPF-SOL-013` 增加稳定 `ModelElementId`、`VariableId`、`ConstraintId`、`ObjectiveId`，贯穿机制模型、中间模型、插件和远程序列化。

**2026-08-05 实施边界说明：** `ospf-kotlin-core` 已提供显式的
`ModelElementIdentityRegistry`，并将稳定变量/约束/目标 ID、namespace、schema、scope 和
origin 传递到线性 triad 与二次 tetrad artifact；未注册元素明确降级为
`model-local-*`，重复 ID 和重复绑定返回结构化错误，诊断 helper 优先使用显式 ID。
该子项不需要 PostgreSQL、对象存储或外部 worker。`OSPF-SOL-013` 总项仍保持未完成，
因为机制模型的统一 origin 生产、全部插件/组合求解器接入以及远程序列化的跨重建稳定
契约尚未在本计划范围内完成；不能用 core 局部 registry 的测试替代全仓库验收。

**2026-08-07 第二轮源码收尾：** 二次机制模型新增与线性对称的身份传播回归
（`QuadraticMetaModelDumpIdentityPropagationTest`），registry 经机制展开进入 tetrad 的
变量/约束/目标 ID、scope、origin、namespace/schema 均通过断言。SCIP 与 Gurobi 的线性/二次
求解器 dump 时使用 `nativeElementName`/`sanitizeNativeName` 将稳定 ID 投影为
`ospf-variable-*`/`ospf-constraint-*` 命名空间化原生名称，model-local 元素保留展示名；
该正向命名投影不改变按索引回查的 `diagnosticConstraintId` 反向路径，presolve/辅助元素
仍使用独立名称。`plans/cp3.md` A106（身份测试矩阵）已勾选：独立重建、注册顺序、重名、
重复 ID、namespace/schema 冲突、组合 solver 选择、远程序列化与 checkpoint 身份往返均已有
回归。`OSPF-SOL-013` 总项仍保持未完成：机制派生元素的统一 origin 生产与跨重建终态矩阵仍需
组合链路验收（SCIP/Gurobi 完整投影矩阵已于第三轮插件级验收）。

**2026-08-07 第三轮源码收尾（插件级投影与传播矩阵）：** 新增 `ScipLinearIdentityProjectionIT` 与
`GurobiLinearIdentityProjectionIT`，在 Configuration 回调读取原生变量/约束名称，验证稳定 ID
投影为 `ospf-variable-*`/`ospf-constraint-*` 命名空间名称，model-local 元素保留展示名；
SCIP/Gurobi 二次求解器与线性共享同一 `nativeElementName` 投影路径（代码审查确认）。诊断辅助
模型使用独立命名（SCIP `diagnostic-var-*`/`diagnostic-row-*`，Gurobi `var-*`/`row-*`），不会
冒充源 ID；presolve 元素由原生求解器内部管理，不映射为源元素 ID。`ScipFarkasDiagnosticIT` 与
`GurobiInfeasibilityDiagnosticIT` 为约束注册稳定 ID 并断言诊断证据 `constraintIds` 回查该 ID；
`ParallelCombinatorialSelectionTest` 增强验证 attempt trace 保留 attemptId/backendId 及 backend
report 的 provenance/fingerprints。插件 Failsafe：SCIP 22 项、Gurobi 5 项全部通过，0 failure/0 error。
`OSPF-SOL-013` 总项仍保持未完成：机制派生元素的统一 origin 生产与跨重建终态矩阵仍需组合链路验收。
- [ ] `OSPF-SOL-014` 实现 `ConstraintEvaluation` 和变量界求值，输出 lhs、rhs、relation、slack、violation、tolerance、satisfied 和可选 dual。

**2026-08-07 本地实现边界：** `ospf-kotlin-core` 已提供 triad/tetrad 的纯函数约束求值入口，
返回结构化 `ConstraintEvaluation<Flt64>`，并对缺少变量值、负容差返回 `Failed`。该入口不包含
后端 dual、原生诊断或部署级状态复验，因此 `OSPF-SOL-014` 总项仍需结合 solver/plugin
和完整报告链路验收。
- [x] `OSPF-SOL-015` 定义统一 `InfeasibilityEvidence`，包含 source、exactness、completeness、成员 ID、耗时和缺失/失败原因。
- [ ] `OSPF-SOL-016` 首阶段优先调用 Gurobi/SCIP 原生 IIS/Farkas；通用弹性过滤和删除过滤按实际保证标记为精确、不可约、启发式或未知，其余 backend analyzer 转入 `plans/solver_cp.md`。
- [ ] `OSPF-SOL-017` 将诊断变为求解报告的附属结果；诊断失败不得覆盖 Infeasible/Unbounded 等已确定结论。

### 6.5 指纹、组合和远程协议

- [x] `OSPF-SOL-018` 定义带 schema 版本的规范化线性/二次数学模型表示和确定数值编码。
- [x] `OSPF-SOL-019` 生成 `ModelFingerprint`、`ConfigurationFingerprint` 和 `SolverFingerprint`，禁止普通对象哈希承担审计语义。
- [x] `OSPF-SOL-020` 定义 `SolveAttemptTrace`，记录每个 backend 的状态、耗时、错误、provenance、取消原因和父子 attempt。
- [ ] `OSPF-SOL-021` 改造串并行组合求解器，返回 attempts、选择依据和最终报告；全部失败时保留原始错误集合。
- [ ] `OSPF-SOL-022` 建立带 `schemaVersion` 的远程 `SolveReport` DTO，保持任务生命周期和求解结论正交。
- [ ] `OSPF-SOL-023` 改造远程线性/二次 adapter，无损保留超时、取消、无界、incumbent、bound、诊断、provenance 和指纹。

**CP3 本地垂直切片证据（2026-08-06）：** framework 已提供 backend-neutral 的远程报告映射，
线性/二次客户端保留 `schemaVersion`、正交状态、incumbent、目标、best bound、gap、诊断、统计、
provenance、fingerprint schema 及 run/attempt；Kotlin 与 remote-solver protocol 共同读取 CP v2
和线性 v2 canonical fixture。`OSPF-SOL-022/023` 总项仍保持未完成，原因是组合求解、完整终态矩阵、
HTTP/持久化 migration 和部署级跨进程验收尚未闭环，不能用本地 fixture 单独勾选总项。

**CP3 文档与测试映射（2026-08-06）：** `plans/cp3.md` A4 表将 direct CP、远程
linear/quadratic、Combinatorial/Benders 和 identity registry 的本地测试逐项列出。该映射
确认 `OSPF-SOL-022/023` 的协议字段与失败传播已有源码证据，但不替代组合 solver 的统一
attempt report、部署 HTTP、持久化重启和全终态矩阵；因此总项继续保持未完成。

### 6.6 评测、文档和实验

- [ ] `OSPF-SOL-024` 建立固定 solver fixture 和最小反例，覆盖最优、可行、不可行、无界、超时、取消、限制和数值异常。
- [ ] `OSPF-SOL-025` 建立 Gurobi/SCIP golden/replay 矩阵，校验状态、目标容差、约束残差、bound、IIS 和指纹一致性；其余插件矩阵由 `plans/solver_cp.md` 扩展。
- [x] `OSPF-SOL-026` 提供纯函数式批量实验运行器、并发预算、确定性排序和结构化报告导出。
- [ ] `OSPF-SOL-027` 更新 core solver、Gurobi/SCIP、framework combinatorial/remote 和 benchmark 双语文档，提供兼容迁移示例，并链接 `plans/solver_cp.md` 的后续插件范围。
- [x] `OSPF-SOL-028` 为生成式 MILP 样本记录生成器版本、种子、模型/配置指纹、解和 provenance。
- [x] `OSPF-SOL-029` 为积分间隙预测和学习分支提供离线观测接口；默认实现不得改变生产求解参数或分支决策。

## 7. 验收标准

### 7.1 架构与契约

- [ ] 求解结论、终止原因、解存在性和运行生命周期彼此正交，不存在 `TimeoutWithSolution` 一类组合枚举膨胀。
- [ ] 已启动求解的 Optimal、Feasible、Infeasible、Unbounded、TimeLimit、NodeLimit、Cancelled 和 NumericalFailure 都能形成结构化报告。
- [ ] 正常不可行、无界、达到限制和取消不通过异常或无上下文 `Failed` 表达。
- [ ] 旧 API 有明确兼容期、映射规则和弃用文档，新能力只扩展统一报告 API。

### 7.2 插件状态与取消

- [ ] Gurobi 与 SCIP 覆盖最优、不可行、无界、超时有解、超时无解和用户取消测试；状态不被压缩为通用 `Feasible`/`SolvingException`。
- [ ] 其余插件不计入本计划验收；在 `plans/solver_cp.md` 完成前，新合同能力保持 `Unsupported` 且不进入远程 solver selection。
- [ ] 取消协程或 `CompletableFuture` 后，backend 原生求解在限定时间内停止并释放资源。
- [ ] 远程停止、本地取消和组合求解取消返回一致的终止原因，重复取消保持幂等。

### 7.3 诊断与解释

- [ ] 每个约束、变量界和目标均可通过稳定 ID 从报告回查到原模型元素；重名不影响关联。
- [ ] 可行解可以生成结构化约束求值，slack 与 violation 在声明容差内正确。
- [ ] 原生 IIS、通用过滤、Farkas 和无证据状态可区分；启发式证据不会被标记为精确 IIS。
- [ ] IIS 计算失败时仍保留原始 Infeasible 结论，并返回诊断失败原因。

### 7.4 Provenance、指纹与重放

- [ ] Gurobi/SCIP 每次报告包含稳定 solver ID、backend/plugin/native 版本和实际生效配置；Gurobi native 主版本可区分，Gurobi 11 插件适配仍由 `plans/solver_cp.md` 验收。
- [ ] 同一规范化模型和配置重复运行产生相同模型/配置指纹；模型或实际配置变化会改变对应指纹。
- [ ] 指纹使用规范化内容的加密摘要，不使用 JVM 对象哈希；敏感参数不会以明文进入报告。
- [ ] 检查点恢复、远程求解和组合求解均保留原 run/attempt、指纹和 provenance 链路。

### 7.5 组合、远程与评测

- [ ] 串行和并行组合求解可返回全部 attempts、失败原因、被取消 backend 和结果选择依据。
- [ ] 全部 backend 失败时不再错误映射为单一 `SolverNotFound`。
- [ ] 本地与远程对同一 fixture 返回语义等价的报告，远程协议版本不兼容时返回结构化协议错误。
- [ ] fixture 覆盖 LP/MIP/QP、不可行、无界、超时、取消和数值异常；跨 backend 目标比较使用显式容差。
- [ ] solver 不可用或许可证缺失在矩阵报告中显式 skipped/unsupported，不计为通过。
- [ ] JMH 性能门禁与求解正确性门禁分离，二者报告都可追溯。

### 7.6 边界验收

- [ ] OSPF 中不存在 `PlanScheme`、业务候选、审批、租户、RollingCycle 或业务 KPI 模型。
- [ ] OSPF 不自动放宽业务硬约束，不隐式启用跨 solver 回退，不让离线学习结果直接影响生产求解。
- [ ] 上层可以只依赖稳定元素 ID、报告、诊断和 provenance 完成业务对象映射，无需解析求解器日志或展示字符串。

## 8. 验证策略

任务进行中优先执行受影响模块的增量编译和测试；最终验收按仓库规则执行全量构建与测试，并完整保存一次构建的输出：

```powershell
mvn -B -ntp compile test-compile -T 0.75C > solve-contract-compile.log 2>&1
mvn -B -ntp test -T 0.75C > solve-contract-test.log 2>&1
```

本计划的插件集成测试只按 Gurobi/SCIP backend profile 分组。商业求解器许可证、原生库或远程服务不可用时，不重复运行完整构建；应从同一次日志中确认并在评测报告中记录环境前置条件。其余插件的 profile、SDK/license 矩阵和验收命令由 `plans/solver_cp.md` 管理。
