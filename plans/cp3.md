# 约束规划第三阶段：公共契约与源码收尾计划

## 1. 文档状态

| 项目 | 内容 |
| --- | --- |
| 状态 | InProgressWithBoundaries |
| 日期 | 2026-08-06 |
| 前置计划 | `plans/cp2.md`，已按 `ImplementedWithBoundaries` 收尾 |
| 公共求解契约 | `plans/schema.md` |
| 求解器插件与 JSCIPOpt 上游 | `plans/solver_cp.md`，JSCIPOpt 工作包当前为 `BlockedByUpstream` |
| 部署环境验收 | `plans/release.md` |
| 远程服务端计划 | `E:/workspace/ospf/ospf/framework/remote-solver/daily.md` |

本计划承接 CP2 中仍需修改 OSPF 源码或完成公共契约的事项：

1. 全链路 stable identity 与统一远程报告。
2. 真实领域 Benders cut serializer 和本地跨模块验收。

原工作流 B～D 中涉及 JSCIPOpt、SCIP 插件条件能力及其后置远程接入的内容已迁移到
`plans/solver_cp.md`。由于 JSCIPOpt 上游当前无法修改，该工作包以 `BlockedByUpstream`
保留，不再阻塞本计划的源码收尾和关闭。

真实 PostgreSQL、S3/MinIO、服务与 worker 进程重启组合、部署级完整终态矩阵和
生产硬件性能基线继续由 `plans/release.md` 管理，不重复纳入本计划的源码完成统计。

## 2. 所有权迁移与剩余结论

- JSCIPOpt 当前基线、建议分支、API 能力判定和完整执行任务统一见 `plans/solver_cp.md`。
- JSCIPOpt 工作包当前为 `BlockedByUpstream`；在获得明确修改/合并权限或采用受维护 fork 前，
  OSPF 保持 model rebuild、assumptions/shrink、exact lowering 和 portable checkpoint。
- 本计划不复制或追踪 JSCIPOpt task ID，只消费已经正式发布且通过版本/capability 门禁的制品。
- `OSPF-SOL-013/022/023` 和领域 Benders serializer 仍是本计划的源码所有权。

### 移交登记表（2026-08-07 审查收尾）

`OSPF-CP3-A205` 与 `OSPF-CP3-A301～A306` 已于 2026-08-07 正式移交
`plans/release.md` 管理：每个交付项只有 release.md 一个实施所有者（满足 A001
“唯一实施所有者”要求），并登记为独立任务编号；移交后不再计入本计划未完成清单。

| 原任务 | 移交任务编号 | 唯一所有者 | 验收位置 | 说明 |
| --- | --- | --- | --- | --- |
| `OSPF-CP3-A205` | `RELEASE-CP-205` | `plans/release.md`（remote-solver 部署 owner） | release.md 移交登记 | 本地 protocol/dispatcher/calculator 部分已完成并有 fixture 回归；Ktorm/数据库 migration（V1～V7）、HTTP 部署链路、PostgreSQL/S3 回放与完整终态矩阵转入部署环境验收 |
| `OSPF-CP3-A301～A306` | `RELEASE-CP-301～306` | `plans/release.md`（真实领域 owner，当前 `BlockedByDomainOwner`） | release.md 移交登记 | 仓库无生产 Logic-Based Benders 样例，后续在 release.md 补充真实领域示例后恢复实施；测试 serializer 与通用 SPI 不计为领域接入完成 |

本计划剩余未完成项仅为 `OSPF-CP3-A101/A102/A401/A402`，均为本地源码/公共契约任务，
不属于部署环境事项，不移交 release.md。

## 3. 仓库与计划边界

- ospf-kotlin 与 remote-solver 是独立仓库，分别提交，不制造跨仓库原子提交假象。
- 只有远程协议、node capability 或 checkpoint/result 字段变化时，才修改 remote-solver；
  进入该阶段时必须同步更新其 `daily.md`。
- JSCIPOpt 上游能力只由 `solver_cp.md` 管理；本计划不得直接修改 `E:/workspace/JSCIPOpt`。
- 每个跨仓库阶段保存兼容 fixture、制品坐标、commit 和完整验证日志。

## 4. 范围与非目标

### 4.1 必做源码与公共契约范围

- 完成 `OSPF-SOL-013` 的全链路稳定身份契约，不用 CP snapshot 的局部完成替代全仓库完成。
- 完成 `OSPF-SOL-022/023` 的线性/二次远程 `SolveReport` 垂直切片，并保持 CP 已有语义。
- 让至少一个真实 Logic-Based Benders 领域接入版本化 cut serializer 和跨进程恢复；
  若仓库没有适合的领域所有者，则必须明确标为 `BlockedByDomainOwner`，不能以测试 serializer 代替。
- 补齐无需生产基础设施即可执行的本地跨模块终态、身份、恢复和失败传播矩阵。
- 同步更新 `schema.md`、`solver_cp.md`、相关双语 README 和必要的 remote-solver `daily.md`。

### 4.2 条件能力范围

- JSCIPOpt、SCIP 插件 session、probing/reoptimization、结构化 conflict、native interval 和
  native checkpoint 均已迁移到 `solver_cp.md`，不属于本计划关闭范围。
- 上游不可用期间，本文只要求现有 fallback 的语义和 capability 保持准确。

### 4.3 非目标

- 不依赖或引入 OR-Tools。
- 不通过访问 SCIP `struct_*`、复制进程内指针或序列化未公开内部结构实现 checkpoint。
- 不把 `writeOrigProblem`、`writeTransProblem`、solution warm start 或 reoptimization 标为 native resume。
- 不把 JNI 中组合 indicator、big-M、linear/cumulative 的 helper 标为 native interval。
- 不在 core、framework、remote DTO 或 portable checkpoint 中暴露 JSCIP/JNI 指针。
- 不因 API 可用就默认迁移 Gantt 或生产 Benders；性能与正确性门禁必须先通过。

## 5. 工作流 A：继续修改源码与公共契约

### Phase A0：现状冻结与所有权校准

- [x] `OSPF-CP3-A001` 建立 `schema.md`、`solver_cp.md`、`release.md`、本计划和
  remote-solver `daily.md` 的任务映射表，保证一个交付项只有一个实施所有者。
- [x] `OSPF-CP3-A002` 为 `OSPF-SOL-013/022/023` 建立源码覆盖清单，列出 core mechanism、
  intermediate triad/tetrad、SCIP/Gurobi、组合求解、远程 DTO、checkpoint 和诊断的已完成/缺失路径。
- [x] `OSPF-CP3-A003` 冻结 stable identity、report 和 Benders document 的 schema 兼容期；
  记录可读旧版本、唯一写入版本、弃用条件和未知未来主版本的拒绝语义。
- [x] `OSPF-CP3-A004` 把生产基础设施事项明确留在 `release.md`，不得用源码计划的勾选状态关闭环境验收。
验收：任务不存在重复所有权或“本地已实现”和“部署已验证”混用；每项边界均能定位到唯一计划。

#### A0 所有权与覆盖映射（2026-08-06）

| 交付项 | 主要源码路径 | 已完成本地证据 | 未完成/边界 |
| --- | --- | --- | --- |
| stable identity | `ModelElementIdentityRegistry`、mechanism model、triad/tetrad、SCIP/Gurobi 插件、组合求解器、remote DTO、checkpoint 与诊断 | registry 15 项、线性/二次机制身份传播、规范化模型身份、remote fixture、checkpoint 身份往返、SCIP/Gurobi 原生名称投影与诊断稳定 ID 回查 | 机制 origin 派生元素统一来源、跨重建终态矩阵 |
| remote report | `SolveReport`、`RemoteReportMapping`、protocol DTO、dispatcher validator | Kotlin 本地垂直切片、canonical fixture、终态矩阵（Optimal～BackendFailure） | 部署级 HTTP、持久化重启回放（`release.md`） |
| Benders resume | `LogicBasedBenders`、`BendersCheckpoint`、serializer SPI | checkpoint schema/digest/fingerprint 门禁、cut-only 恢复、master 状态显式适配器 | 真实领域 serializer 与跨进程持久化（`BlockedByDomainOwner`） |
| 诊断 | `InfeasibilityEvidence`、SCIP/Gurobi analyzer | Farkas/IIS 证据、稳定 ID 回查、组合门禁 | native 能力迁移（`solver_cp.md`） |

（该表于 2026-08-07 第三轮按 A002 交付内容重建，与原 A0 映射表等价。）

- `RemoteSolverClientTest` 已覆盖 Optimal、Feasible 限制终止、Infeasible、Unbounded、InfeasibleOrUnbounded、Unknown、Cancelled、Interrupted、NumericalFailure、BackendFailure、未来 schema 和损坏 artifact；本轮定向结果为 16 项，0 failure、0 error。
- `NormalizedMathematicalModel` 的 canonical fingerprint 已纳入 identity namespace/schema 及变量、约束、目标的 scope/origin；独立重建与来源变化回归位于 `SolveReportTest`。
- 2026-08-07 增加 triad/tetrad 到 `NormalizedMathematicalModel` 的统一本地构造入口，稳定 ID、scope、origin、namespace 和 schema 会随规范化模型生成；canonical 编码同时转义嵌套项使用的逗号和冒号，避免分隔符碰撞。`SolveReportTest` 覆盖线性/二次构造及边界编码。该证据仍不替代机制 origin、插件投影、组合求解器和跨重建全链路验收。
- 上述证据关闭的是 Kotlin 本地协议/规范化层。remote-solver 的 dispatcher、持久化迁移、真实 HTTP 部署和 PostgreSQL/S3 重启回放仍按 `plans/release.md` 保持边界；A205 不因 canonical fixture 读取而提前关闭。
- 2026-08-06 增补线性/二次模型身份 DTO：两仓库的 `SerializedLinearModel`、`SerializedQuadraticModel` 及其变量、约束、目标单元现在保留 `identityId`、`identityScope`、`identityOrigin*`、`identityNamespace` 和 `identitySchemaVersion`，旧字段均有兼容默认值；共享 `remote-linear-model-v2.json` fixture 在 Kotlin framework 与 remote-solver protocol 各有 round-trip 回归。该证据推进 A205 的协议层，但不替代 dispatcher/calculator、持久化 migration、HTTP 部署和完整终态验收。
- 2026-08-06 增加服务端 `RemoteResultValidator`：dispatcher 在切片和最终结果写入状态前校验 schema 主版本、正交状态、解/目标互斥、resultRef/digest 成对关系及严格 v2 的 run/attempt 归属；旧 v1 保持兼容，数学解语义仍由后端适配器复验。protocol 回归覆盖严格 v2、未来版本、跨任务归属和 legacy v1。
- 2026-08-06 增补真实 serializer 回归：`OspfRemoteModelSerializerTest` 使用带稳定变量/约束/目标身份的 triad 实例，验证实际映射输出 `identityId`、scope、origin、namespace 和 schema；协议 scope 统一写为 `STABLE`/`MODEL_LOCAL`。
- 2026-08-06 增补 identity registry 负路径和并发 fallback 回归：空 namespace/schema 返回结构化失败，256 个并行 fallback 解析保持唯一 ID 且通过最终校验；该证据覆盖 registry 层的 A106，不替代机制 origin、全部插件/组合 solver 的稳定绑定。
- 2026-08-06 在 `solver_cp.md` 增加 SCIP、Gurobi、Gurobi 11、COPT/CPLEX/MindOPT/Mosek、Hexaly/Lingo/OPTVerse、CPLEX CP Optimizer 和 Heuristic 的逐插件 identity 适配矩阵；未验收插件明确保持 `ModelLocal`/`Unsupported`，A105 已完成，不能据此关闭 OSPF-SOL-013 总项。
- 2026-08-06 增补 dispatcher 生命周期回归：`RemoteSolverServiceTimeoutPolicyTest` 注入未来主版本的严格切片结果，确认 `RemoteResultValidator` 在任务状态持久化前拒绝结果并生成结构化 `SOLVER_EXECUTION_FAILED`；本地定向结果为 Kotlin 19 项、remote protocol 15 项、dispatcher 7 项，均 0 failure、0 error。
- A205 本地部分（协议 fixture、serializer、dispatcher 边界、README/daily 同步）已完成并有
  回归；剩余 Ktorm/数据库 migration、HTTP 部署链路、对象存储回放和完整终态矩阵已正式移交
  `RELEASE-CP-205`（`plans/release.md`），A205 不再计入本计划未完成清单。

### Phase A1：完成 OSPF-SOL-013 稳定身份

- [ ] `OSPF-CP3-A101` 为机制模型变量、约束、目标和派生元素建立显式 identity source；
  禁止以对象地址、注册顺序、展示名称或普通 JVM hash 生成跨重建稳定 ID。
- [ ] `OSPF-CP3-A102` 让 identity 的 `id/namespace/schemaVersion/scope/origin` 贯穿规范化、
  linear triad、quadratic tetrad、CP snapshot 和 solver artifact，不丢失一对多/多对一来源关系。
- [x] `OSPF-CP3-A103` 完成 SCIP 与 Gurobi 的源元素到 native artifact 投影；辅助变量、辅助约束、
  feasibility artifact 和 presolve 元素使用独立 artifact ID，不能冒充源 ID。
- [x] `OSPF-CP3-A104` 完成串行/并行组合求解器、attempt trace、诊断、checkpoint 和远程 DTO 的身份传播；
  identity 构建失败必须返回结构化错误，不能静默回落为空 namespace。
- [x] `OSPF-CP3-A105` 在 `solver_cp.md` 为其余插件建立逐插件 identity 适配任务；未完成插件保持
  `ModelLocal`/`Unsupported`，不得宣称全仓库稳定。
- [x] `OSPF-CP3-A106` 增加独立重建、注册顺序变化、重名、重复 ID、namespace/schema 冲突、
  组合 solver 选择、远程序列化和 checkpoint 往返测试。

当前已补齐一条本地传播切片：`MetaModel` 的可选 registry 会随 mechanism model、
linear triad/quadratic tetrad 及 Flt64 conversion 传递；变量、约束和子目标保留原始
source reference，objective dump 也能读取显式 objective binding。`LinearMetaModelDumpExplicitConstantsPathTest`
覆盖了变量、约束、目标的实际展开路径。该证据不替代机制 origin、插件投影、组合求解器和跨重建
全链路验收，因此 A101/A102 仍保持未勾选（A106、A103、A104 已按本地测试矩阵关闭）。

2026-08-07 增补规范化模型构造和本地约束求值：triad/tetrad 可直接生成带身份元数据的
`NormalizedMathematicalModel`，线性/二次约束可返回结构化 lhs、rhs、slack、violation、
tolerance 和 satisfied 结果；缺少变量值或非法容差返回 `Failed`。并行线性/二次组合求解器的
进度最佳值比较也已按最大化/最小化方向修正。上述属于本地公共契约证据，不关闭插件投影、
组合 solver 完整 attempt 矩阵或全链路稳定身份任务。

2026-08-07 增补身份错误传播与组合求解器回归：Linear/QuadraticMechanismModel 工厂在展开前校验
registry，Failed/Fatal 直接返回，不再静默展开到中间模型；ParallelCombinatorialLinearSolver 的
solveCombinatorialReport 顶层增加 identity 门禁，修复身份失败被吞成 NoSuccessfulAttempt 报告的问题。
新增 ModelElementIdentityRegistryOrderTest（注册顺序无关）、CombinatorialSolverIdentityTest（串行/并行
身份失败不启动 backend）和 ParallelCombinatorialSelectionTest（First/Best 选择依据与最小化/最大化
方向）。机制 origin 全链路、SCIP/Gurobi 完整投影和跨重建终态矩阵仍未关闭，
因此 A101/A102/A103/A104 保持未勾选（A106 已在 2026-08-07 第二轮关闭）。

2026-08-07 第二轮源码收尾补齐：
- `QuadraticMetaModelDumpIdentityPropagationTest` 为二次机制模型增加与线性对称的身份传播回归，
  验证显式 registry 经 `QuadraticMechanismModel` 展开后进入 tetrad 的变量 ID、约束 ID、scope、
  origin、namespace 和 schema，且 `identityValidation` 通过；A101 的机制侧证据已覆盖线性/二次两条主路径。
- A103 前向投影接入：core 新增 `nativeElementName`/`sanitizeNativeName`（`ModelingPreparation.kt`），
  SCIP 与 Gurobi 的线性/二次求解器在 dump 时用稳定元素 ID 生成 `ospf-variable-*`/`ospf-constraint-*`
  命名空间化原生名称；model-local 元素保留展示名，不会冒充跨重建身份。`ModelingPreparationTest` 覆盖
  稳定 ID 投影、blank/model-local 回退和字符净化。该实现只解决“源元素 -> native artifact”的正向命名投影，
  native 行的反向回查仍由索引/`diagnosticConstraintId` 负责，presolve/辅助元素独立命名；A103 的完整
  feasibility artifact 与 presolve 投影矩阵仍需插件级验收，保持未勾选。
- A106 关闭：独立重建、注册顺序、重名、重复 ID、namespace/schema 冲突、组合 solver 选择、远程序列化
  与 checkpoint 身份往返测试均已存在；`ConstraintProgrammingCheckpointProcessTest` 新增
  `checkpointRoundTripPreservesStableIdentity`，验证 stable scope/origin 经 capture/encode/decode/restore
  后身份 manifest 一致。

2026-08-07 第三轮源码收尾补齐（插件级投影与传播矩阵）：
- A103 插件级验收：新增 `ScipLinearIdentityProjectionIT` 与 `GurobiLinearIdentityProjectionIT`，在 Configuration 回调读取原生
  变量/约束名称，断言稳定 ID 投影为 `ospf-variable-*`/`ospf-constraint-*` 命名空间名称，model-local 元素保留展示名；
  二次求解器与线性共享同一 `nativeElementName` 投影路径，经代码审查确认。诊断辅助模型使用独立命名
  （SCIP `diagnostic-var-*`/`diagnostic-row-*`，Gurobi `var-*`/`row-*`），不会冒充源 ID；presolve 元素由原生求解器
  内部管理，不映射为源元素 ID。因此 A103 已按插件级投影与独立辅助命名矩阵关闭。
- A104 传播矩阵：组合求解器身份门禁（串行/并行线性与二次共 4 项，身份失败不启动 backend）、attempt trace
  保留 attemptId/backendId 及 backend report 的 provenance/fingerprints（`ParallelCombinatorialSelectionTest` 增强）、
  诊断证据回查稳定 ID（`ScipFarkasDiagnosticIT`/`GurobiInfeasibilityDiagnosticIT` 为约束注册稳定 ID 并断言
  `constraintIds` 返回该 ID）、checkpoint 身份往返（A106）、远程 DTO 身份元数据往返（`RemoteModelIdentitySerializationTest`）
  均已存在，identity 构建失败结构化返回；因此 A104 已按本地传播矩阵关闭。

因此 A106、A103、A104 已按本地测试矩阵关闭；A101/A102 仍保持未勾选，理由见下表（A4 证据映射），
避免把局部切片误报为全链路稳定身份完成。

验收：同一逻辑模型独立构建后显式稳定元素 ID 一致；所有报告成员可回查来源；
无法稳定的元素机器可识别为 `ModelLocal`，且不会进入要求跨重建身份的恢复路径。

### Phase A2：完成 OSPF-SOL-022/023 远程报告

- [x] `OSPF-CP3-A201` 冻结 backend-neutral、带 `schemaVersion` 的远程 `SolveReport` DTO，
  保持 problem status、termination reason、solution presence 和 task lifecycle 正交。
- [x] `OSPF-CP3-A202` 完成线性与二次客户端/服务端 adapter，保留 incumbent、objective、best bound、
  gap、proof、diagnostics、statistics、provenance、fingerprints 和 run/attempt 链。
- [x] `OSPF-CP3-A203` 对 CP、线性、二次共用字段建立 canonical JSON fixture；Int64 字段不得经过 Double，
  Flt64 特殊值和精度策略必须显式。
- [x] `OSPF-CP3-A204` 补齐旧 DTO 读取、未知版本拒绝、缺失字段降级、raw/artifact 交叉校验和
  capability negotiation；不能把协议不兼容映射为求解不可行。
- [-] `OSPF-CP3-A205` 同步 remote-solver 的 protocol、dispatcher、calculator、持久化 migration、
  HTTP API、README 和 `daily.md`，并使用同一 fixture 双向验收。
  （本地部分已完成；剩余部署部分已移交 `RELEASE-CP-205`，见移交登记表。）
- [x] `OSPF-CP3-A206` 覆盖 Optimal、Feasible、Infeasible、Unbounded、InfeasibleOrUnbounded、Unknown、
  TimeLimit、NodeLimit、IterationLimit、SolutionLimit、Cancelled、Interrupted、NumericalFailure、BackendFailure，
  以及允许/禁止 incumbent 的组合。

验收：本地与远程对同一 fixture 形成语义等价报告；状态、证明和解不会互相推断或覆盖；
协议错误返回结构化错误，不生成伪造的 solver terminal state。

### Phase A3：领域 Benders cut serializer 实际接入

- [-] `OSPF-CP3-A301` 盘点实际使用 Logic-Based Benders 的 framework/domain，选择至少一个有稳定领域键、
  可重建 master binding 和明确 cut 语义的生产型用例；没有领域所有者时记录 `BlockedByDomainOwner`。
  （已移交 `RELEASE-CP-301～306`，见移交登记表。）
- [-] `OSPF-CP3-A302` 为选定领域实现版本化 `BendersCutSerializer`，payload 只包含 primitive/value object，
  不包含闭包、JVM 对象地址、solver handle 或不可稳定序列化对象。
- [-] `OSPF-CP3-A303` 将 cut ID、validity、provenance、master/subproblem fingerprint、stable binding 和
  serializer schema 纳入摘要与恢复门禁。
- [-] `OSPF-CP3-A304` 接入 capture、对象存储 artifact、独立 JVM/worker decode、master rebuild、cut pool 去重
  和 resume；不兼容 schema、错模型、错 binding、损坏 digest 和未知 cut 必须拒绝。
- [-] `OSPF-CP3-A305` 恢复 Exact cut 前重新验证其证明条件；无法复验时降级为不可用于 Exact 收敛，
  不能仅因 checkpoint 中标记为 Exact 就接受。
- [-] `OSPF-CP3-A306` 建立同进程、独立 JVM、LocalFS/H2 和 HTTP 内存链路测试；真实 PostgreSQL/S3 与
  服务进程重启仍转入 `release.md` 验收。

验收：至少一个真实领域 cut 可跨序列化边界恢复并继续求解；所有错误/篡改路径均结构化失败；
测试 fixture serializer 不计为领域接入完成。

2026-08-07 领域所有权确认（用户确认）：当前仓库没有实际使用 Logic-Based Benders 的生产领域
样例，因此不存在可提供稳定领域键、可重建 master binding 与明确 cut 语义的领域 owner；
A301～A306 正式登记为 `BlockedByDomainOwner`，不属于“有任务未完成”，而是“无领域所有者导致
无法实施”。2026-08-07 审查收尾将该工作包正式移交 `plans/release.md` 的
`RELEASE-CP-301～306`（含独立验收条件与唯一所有者，见移交登记表），后续在 release.md
补充真实领域示例后恢复实施；测试 serializer 与通用 SPI 不计为领域接入完成。

### Phase A4：本地终态与文档闭环

- [ ] `OSPF-CP3-A401` 建立 SCIP/Gurobi、direct/remote/combinatorial、linear/quadratic/CP 的本地终态矩阵，
  覆盖有/无 incumbent、取消竞态、诊断失败和 identity 失败。
- [ ] `OSPF-CP3-A402` 补齐 `schema.md` 中 `OSPF-SOL-013/022/023` 的验收证据并按真实状态勾选；
  其他未完成 SOL 项不得被连带关闭。

A401/A402 是本地源码/公共契约任务（本地终态矩阵与 schema 验收证据），不随
A205/A301～A306 移交 release.md，继续由本计划实施并保持未勾选。
- [x] `OSPF-CP3-A403` 更新 `solver_cp.md` 的剩余插件接入前置条件、capability 和测试模板。
- [x] `OSPF-CP3-A404` 更新相关中英文 README、迁移示例和 capability matrix；公开 API 文档必须说明
  stable/model-local、portable/native、rebuild/reuse/reoptimization 的区别。

2026-08-06 本地文档收尾：`solver_cp.md` 已增加 capability 发布门禁和共享插件测试模板；
solver README 中英文版本已说明 stable/model-local、portable/native、rebuild/reuse/reoptimization
边界及迁移处理。该证据不关闭插件 native 能力、完整终态矩阵、跨进程部署或生产性能任务。

2026-08-07 第三轮验证（历史记录）：Kotlin Surefire XML 汇总 `3258 tests, 0 failures, 0 errors, 6 skipped`
（496 个测试类，含本轮新增的 ModelingPreparation 3 项、checkpoint 身份往返 1 项、二次机制身份传播 1 项），
SCIP/Gurobi Failsafe 汇总 `27 tests, 0 failures, 0 errors`（SCIP 22、Gurobi 5）；remote-solver
Surefire `352 tests, 0 failures, 0 errors`、HTTP Failsafe `1 test, 0 failures, 0 errors`。完整日志位于
`D:/temp/ospf-cp3-kotlin-verification-20260807/`（compile-targeted2.log、targeted-tests5.log、
test-full.out.log + test-full-resume.out.log、verify-plugin.out.log、install-current.log、
remote-test.log、remote-verify.log）。本轮 Kotlin 全量 `mvn test` 因并发 Kotlin 编译 Metaspace OOM
在 77/80 中断一次，使用 `mvn test -rf :ospf-kotlin-framework-bpp3d-application` 续跑完成，两段均为
0 failures/0 errors；这是本机内存环境问题，非源码缺陷。
2026-08-07 第三轮插件级验证：SCIP/Gurobi 插件 `verify`（Failsafe）为 SCIP `22 tests`（新增
`ScipLinearIdentityProjectionIT`）、Gurobi `5 tests`（新增 `GurobiLinearIdentityProjectionIT`），
0 failures/0 errors；`ScipFarkasDiagnosticIT` 与 `GurobiInfeasibilityDiagnosticIT` 增强为约束注册
稳定 ID 并通过。framework 定向测试 `ParallelCombinatorialSelectionTest`/`CombinatorialSolverIdentityTest`
8 项通过。日志位于 `D:/temp/ospf-cp3-progress-20260807-verify-plugin.log` 与
`D:/temp/ospf-cp3-progress-20260807-targeted-framework.log`。Kotlin Surefire 总数维持 3258 项（本轮只增强
既有断言，不新增单元测试方法）。

2026-08-07 审查修复收尾（第四轮，当前权威）：
- `nativeElementName`/`sanitizeNativeName` 改为可逆 UTF-8 字节十六进制转义
  （`_h<2 位 hex>_`，含下划线自身），`a/b`、`a b`、`a_b` 等输入不再碰撞；
  编码超过 180 字符时按“安全前缀 + SHA-256 摘要”截断。`ModelingPreparationTest`
  增至 8 项，新增碰撞区分、无损往返和超长 ID 确定性回归；SCIP/Gurobi 插件级投影
  IT 断言不变（`fixture:variable:x` 等均为安全字符）。
- A205/A301～A306 正式移交登记见第 2 节移交登记表（`RELEASE-CP-205`、
  `RELEASE-CP-301～306`），release.md 为唯一所有者。
- 本机内存被其他项目活跃构建占用，Kotlin 全量并发从 `-T 0.75C` 降至 `-T 0.5C`，
  一次完整 `mvn test` 获得 `BUILD SUCCESS`（不再依赖 `-rf` 续跑）：Surefire XML 汇总
  `3261 tests, 0 failures, 0 errors, 6 skipped`（496 个测试类，含新增 3 项）；
  `mvn clean compile test-compile` 同样一次 `BUILD SUCCESS`；SCIP/Gurobi 插件 `verify`
  Failsafe 汇总 `27 tests, 0 failures, 0 errors`（SCIP 22、Gurobi 5）。
- remote-solver 使用隔离仓库 `D:/temp/ospf-cp3-fix-local-m2`（含更新后的
  `ospf-kotlin-core:1.1.0`）完成 `clean test` 与 `verify`：Surefire `352 tests,
  0 failures, 0 errors`，HTTP Failsafe `1 test, 0 failures, 0 errors`。
- 完整日志位于 `D:/temp/ospf-cp3-review-fix-20260807/`（core-targeted3.log、
  framework-targeted3.log、test-full-fix.log、clean-compile-test-compile-fix.log、
  verify-plugin-fix.log、remote-clean-test-fix2.log、remote-verify-fix.log）。
  该统计来自报告文件与完整日志，不代表 JSCIPOpt、PostgreSQL/S3 或生产部署能力已经验收。

历史记录（2026-08-06）：Kotlin Surefire XML 汇总 `3239 tests, 0 failures, 0 errors, 6 skipped`，
SCIP/Gurobi Failsafe 汇总 `25 tests, 0 failures, 0 errors`；remote-solver 最近一次锁步
验证为 Surefire `352 tests, 0 failures, 0 errors`、HTTP Failsafe `1 test, 0 failures, 0 errors`。本轮 Kotlin 完整日志为
`D:/temp/cp3-kotlin-clean-compile-final.log`、`D:/temp/cp3-kotlin-test-final.log` 和
`D:/temp/cp3-kotlin-verify-final.log`；remote-solver 对应日志仍为
`D:/temp/cp3-remote-clean-compile-final.log`、`D:/temp/cp3-remote-test-final.log` 和
`D:/temp/cp3-remote-verify-final.log`。此前的 `cp3-kotlin-full-test.log`、
`cp3-kotlin-clean-compile-current.log` 与 `cp3-remote-full-test.log` 仅作历史记录。
该统计来自报告文件与完整日志，不代表 JSCIPOpt、PostgreSQL/S3 或生产部署能力已经验收。

### A4 本地证据映射（2026-08-06）

| 终态/路径 | 当前本地证据 | 尚未关闭项 |
| --- | --- | --- |
| CP direct | `FakeConstraintProgrammingSolverTest`、`MipBackedConstraintProgrammingSolverTest`、SCIP Failsafe | Gurobi CP 与所有限制/取消竞态的统一矩阵 |
| 远程 linear/quadratic | `RemoteSolverClientTest`、`RemoteSolverHttpClientTest`、protocol/dispatcher validator tests | 部署级 HTTP、持久化重启和跨模块状态等价 |
| Combinatorial/Benders | `LogicBasedBendersTest`、列生成值转换测试、`ParallelCombinatorialSelectionTest`、`CombinatorialSolverIdentityTest` | 全部 backend attempts 与统一 report 的端到端矩阵 |
| Identity | registry 15 项（含注册顺序）、triad/tetrad 线性与二次身份传播、remote model fixture、组合求解器身份门禁、attempt trace 保留 provenance/fingerprints、checkpoint 身份往返、原生命名投影辅助（`ModelingPreparationTest` 8 项：投影/净化/碰撞/往返/超长）、SCIP/Gurobi 插件级原生名称投影 IT、诊断证据稳定 ID 回查 IT | 机制 origin 的派生元素统一来源与跨重建终态矩阵（SCIP/Gurobi 线性/二次源元素投影及诊断回查已完成） |

该表补齐 A401 的本地覆盖清单，但因后三列仍有缺口，A401 保持未勾选。`schema.md` 中
`OSPF-SOL-013/022/023` 已记录上述局部证据和未完成原因；A402 仍保持未勾选，避免把局部
垂直切片误报为全链路关闭。

## 6. JSCIPOpt 条件能力迁移说明

JSCIPOpt 源码修改及其后置 SCIP 插件/remote-solver 接入已完整迁移到
`plans/solver_cp.md` 的 `BlockedByUpstream` 工作包。原 CP3 B/C/D task ID 不再使用，
本计划不保留重复任务，也不以未提交的 JSCIPOpt 工作树差异作为完成证据。

## 7. 测试矩阵

| 层级 | 必测内容 |
| --- | --- |
| stable identity | 独立重建、顺序变化、重名、重复 ID、组合 solver、远程/checkpoint 往返 |
| remote report | 全终态、有/无 incumbent、proof/diagnostics、Int64/Flt64、旧协议/未来协议 |
| Benders resume | 真实领域 serializer、schema/digest/fingerprint、错 binding、独立 JVM、cut 复验 |
| fallback | model-local、legacy DTO、诊断失败、无领域 serializer、portable rebuild |

JSCIPOpt/JNI、session、probing、reoptimization、conflict 和 interval 的测试矩阵已迁移到
`solver_cp.md`。本计划只统计 OSPF 公共契约和现有 fallback 的测试。

## 8. 构建与验证策略

### 8.1 ospf-kotlin

任务进行中使用受影响模块的增量测试；收尾按仓库规则一次性执行并保留完整日志：

```powershell
$cp3KotlinLogDir = Join-Path $env:TEMP 'ospf-cp3-kotlin-verification'
New-Item -ItemType Directory -Force -Path $cp3KotlinLogDir | Out-Null

mvn clean compile test-compile -T 0.75C *> (Join-Path $cp3KotlinLogDir 'compile.log')
mvn test -T 0.75C *> (Join-Path $cp3KotlinLogDir 'test.log')
mvn verify -pl ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip,ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-gurobi -am -T 0.75C *> (Join-Path $cp3KotlinLogDir 'verify.log')
git diff --check
```

### 8.2 remote-solver

只有工作流 A2/A3 改动服务端时才执行，并先安装本轮 ospf-kotlin 制品到隔离仓库。
服务端同样执行 clean compile/test-compile、全量 test、verify、install，并完整保存日志；
不能用单场景 HTTP E2E 代替单元测试或部署环境验收。

## 9. 风险与控制

| 风险 | 影响 | 控制措施 |
| --- | --- | --- |
| stable ID 只在 CP 局部完成 | 远程/组合/线性路径仍不可重放 | 以 `OSPF-SOL-013` 全链路矩阵关闭，不看类型是否存在 |
| 远程报告只验证局部 fixture | 部署终态仍可能丢字段 | 本地矩阵与 `release.md` 部署矩阵分开验收 |
| 测试 serializer 被当作领域接入 | Benders 跨进程恢复不可用 | 必须由真实领域 owner 提供稳定 schema/binding |
| JSCIPOpt 阻塞被误算为 CP3 未完成 | 公共契约计划长期无法关闭 | 所有 JSCIP 任务由 `solver_cp.md` 独立管理 |

## 10. 执行顺序与同步门禁

```text
A0 所有权与 schema 冻结
  -> A1 stable identity + A2 remote SolveReport
  -> A3 真实领域 cut serializer
  -> A4 本地终态与文档闭环
```

`solver_cp.md` 中的 JSCIPOpt 工作包独立保持 `BlockedByUpstream`，不与上述链路建立关闭依赖。

禁止越过的门禁：

1. stable identity 未完成全链路重建验收前，不将 `ModelLocal` 提升为 `Stable`。
2. remote DTO 未冻结前，不修改服务端持久化 schema。
3. 真实领域 serializer 不存在时，不把通用 SPI 或测试 fixture 标记为领域接入完成。
4. 部署环境证据缺失时，不关闭 `release.md` 中的 PostgreSQL/S3、重启或性能事项。

## 11. 完成与关闭标准

本计划只有同时满足以下条件才能关闭：

- [ ] 工作流 A 的必做源码项完成；领域 serializer 若无实际领域所有者，必须形成
  `BlockedByDomainOwner` 结论并迁移到明确的领域计划。
  （2026-08-07 已确认：仓库无生产领域样例，A301～A306 登记为 `BlockedByDomainOwner`，
  结论见 Phase A3；工作包已正式移交 `plans/release.md` 的 `RELEASE-CP-301～306`，
  后续补充真实领域示例后恢复。）
- [ ] `OSPF-SOL-013/022/023` 按实际完成状态更新，stable ID、远程报告和 Benders 恢复均有端到端源码证据。
- [ ] 所有协议变化已与 remote-solver 锁步更新 fixture、migration、README 和 `daily.md`。
- [ ] ospf-kotlin 与实际修改的 remote-solver 模块完成适用的全量构建、测试、集成验证和
  `git diff --check`，日志保存在仓库外。
- [ ] `release.md` 中真实 PostgreSQL/S3、部署重启矩阵和生产性能基线继续独立管理，
  不因本计划源码完成而自动勾选。

JSCIPOpt 条件能力不属于本计划关闭条件；其重新启动、拒绝结论和最终发布均以
`solver_cp.md` 为唯一权威计划。
